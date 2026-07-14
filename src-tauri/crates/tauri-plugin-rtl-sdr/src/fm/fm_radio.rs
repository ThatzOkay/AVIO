use std::ffi::{c_char, c_int, c_void};
use std::sync::Mutex;
use std::thread;

use desperado::dsp::decimator::Decimator;
use desperado::dsp::DspBlock;
use fmradio::rds::{RdsDecoder, RdsResamplerCustom, StereoDecoderPLL};
use fmradio::{AdaptiveResampler, DeemphasisFilter, PhaseExtractor};
use num_complex::Complex;
use tokio::sync::mpsc::UnboundedSender;

type AudioCallback = Box<dyn Fn(Vec<f32>) + Send + Sync>;

pub type RtlSdrDev = *mut c_void;
pub type ReadAsyncCb = extern "C" fn(buf: *mut u8, len: u32, ctx: *mut c_void);

// Superseded by the caller-side SAMPLE_RATE in fm_radio_service.rs; kept for reference.
#[allow(dead_code)]
const SAMPLE_RATE: u32 = 2048000;
#[allow(dead_code)]
const OUTPUT_RATE: u32 = 48000;

#[link(name = "rtlsdr")]
unsafe extern "C" {
    pub fn rtlsdr_get_device_count() -> u32;
    pub fn rtlsdr_get_device_name(index: u32) -> *const c_char;
    pub fn rtlsdr_open(dev: *mut RtlSdrDev, index: u32) -> c_int;
    pub fn rtlsdr_close(dev: RtlSdrDev) -> c_int;
    pub fn rtlsdr_set_center_freq(dev: RtlSdrDev, freq: u32) -> c_int;
    pub fn rtlsdr_set_sample_rate(dev: RtlSdrDev, rate: u32) -> c_int;
    pub fn rtlsdr_set_tuner_gain_mode(dev: RtlSdrDev, manual: c_int) -> c_int;
    pub fn rtlsdr_set_tuner_gain(dev: RtlSdrDev, gain: c_int) -> c_int;
    pub fn rtlsdr_reset_buffer(dev: RtlSdrDev) -> c_int;
    pub fn rtlsdr_read_async(
        dev: RtlSdrDev,
        cb: ReadAsyncCb,
        ctx: *mut c_void,
        buf_num: u32,
        buf_len: u32,
    ) -> c_int;
    pub fn rtlsdr_cancel_async(dev: RtlSdrDev) -> c_int;
}

struct FfiDevHandle(RtlSdrDev);
unsafe impl Send for FfiDevHandle {}

impl FfiDevHandle {
    /// Returns the raw pointer through a method call rather than tuple-struct
    /// destructuring, so closures capture the whole `Send` wrapper instead of the
    /// (non-`Send`) pointer field directly via disjoint capture.
    fn ptr(&self) -> RtlSdrDev {
        self.0
    }
}

static FFI_DEV: Mutex<Option<FfiDevHandle>> = Mutex::new(None);
static FFI_SAMPLE_RATE: Mutex<u32> = Mutex::new(2_048_000);

static RDS_STATE: Mutex<RdsInfo> = Mutex::new(RdsInfo {
    program_id: 0,
    program_type: String::new(),
    station_name: None,
    radio_text: None,
});

enum StreamCommand {
    #[allow(dead_code)]
    Tune(u32),
    Stop,
}

static CMD_TX: Mutex<Option<UnboundedSender<StreamCommand>>> = Mutex::new(None);

pub fn fm_open(index: u32) -> Result<i32, String> {
    let ffi_count = unsafe { rtlsdr_get_device_count() };
    if (index as usize) < ffi_count as usize {
        let mut dev: RtlSdrDev = std::ptr::null_mut();
        let result = unsafe { rtlsdr_open(&mut dev, index) };
        if result == 0 {
            *FFI_DEV.lock().unwrap() = Some(FfiDevHandle(dev));
            return Ok(0);
        }
    }
    Err("Failed to open device".into())
}

pub fn fm_close() {
    if let Some(FfiDevHandle(dev)) = FFI_DEV.lock().unwrap().take() {
        unsafe {
            rtlsdr_cancel_async(dev);
            rtlsdr_close(dev);
        }
    }
}

pub fn fm_set_sample_rate(rate: u32) -> i32 {
    *FFI_SAMPLE_RATE.lock().unwrap() = rate;
    match FFI_DEV.lock().unwrap().as_ref() {
        Some(FfiDevHandle(dev)) => unsafe { rtlsdr_set_sample_rate(*dev, rate) },
        None => -1,
    }
}

pub fn fm_set_gain(gain: i32) {
    if let Some(FfiDevHandle(dev)) = FFI_DEV.lock().unwrap().as_ref() {
        unsafe {
            if gain < 0 {
                rtlsdr_set_tuner_gain_mode(*dev, 0);
            } else {
                rtlsdr_set_tuner_gain_mode(*dev, 1);
                rtlsdr_set_tuner_gain(*dev, gain);
            }
        }
    }
}

pub fn fm_set_frequency(freq: u32) -> i32 {
    match FFI_DEV.lock().unwrap().as_ref() {
        Some(FfiDevHandle(dev)) => unsafe { rtlsdr_set_center_freq(*dev, freq) },
        None => -1,
    }
}

fn update_rds_state(rds: RdsInfo) {
    let mut state = RDS_STATE.lock().unwrap();
    *state = rds;
}

pub fn fm_get_rds() -> RdsInfo {
    RDS_STATE.lock().unwrap().clone()
}

/// Only forwards the raw IQ bytes over a channel; FM demod/RDS decode happens
/// on a separate thread (see `fm_read`). rtlsdr_read_async invokes this
/// callback in-line on its own dedicated thread and won't service other USB
/// transfers (e.g. a frequency-retune control transfer) until it returns, so
/// it must stay cheap or tuning while playing can stall indefinitely.
extern "C" fn ffi_read_callback(buf: *mut u8, len: u32, ctx: *mut std::os::raw::c_void) {
    if buf.is_null() || len == 0 || ctx.is_null() {
        return;
    }
    let tx = unsafe { &*(ctx as *const std::sync::mpsc::Sender<Vec<u8>>) };
    let bytes = unsafe { std::slice::from_raw_parts(buf, len as usize) }.to_vec();
    let _ = tx.send(bytes);
}

pub fn fm_read(
    callback: AudioCallback,
    output_rate: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let dev = match FFI_DEV.lock().unwrap().as_ref() {
        Some(FfiDevHandle(dev)) => FfiDevHandle(*dev),
        None => return Err("No device available".into()),
    };
    let input_rate = *FFI_SAMPLE_RATE.lock().unwrap();
    let mut demod = FmDemod::new(input_rate, output_rate)?;

    let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();

    // Dedicated DSP thread, off the USB read thread.
    thread::spawn(move || {
        while let Ok(bytes) = rx.recv() {
            let audio = demod.process(&bytes);
            update_rds_state(demod.rds());
            callback(audio);
        }
    });

    let ctx = Box::into_raw(Box::new(tx)) as usize;
    thread::spawn(move || {
        let dev = dev.ptr();
        let ctx_ptr = ctx as *mut std::os::raw::c_void;
        unsafe {
            rtlsdr_reset_buffer(dev);
            // More in-flight buffers than librtlsdr's default (15) so this thread
            // can tolerate longer scheduling delays before libusb has to drop data.
            rtlsdr_read_async(dev, ffi_read_callback, ctx_ptr, 32, 0);
            drop(Box::from_raw(ctx_ptr as *mut std::sync::mpsc::Sender<Vec<u8>>));
        }
    });

    Ok(())
}

pub async fn fm_stop() {
    if let Some(FfiDevHandle(dev)) = FFI_DEV.lock().unwrap().as_ref() {
        unsafe {
            rtlsdr_cancel_async(*dev);
        }
    }

    if let Some(tx) = CMD_TX.lock().unwrap().take() {
        let _ = tx.send(StreamCommand::Stop);
    }
}

#[derive(Clone)]
pub struct RdsInfo {
    pub program_id: u32,
    pub program_type: String,
    pub station_name: Option<String>,
    pub radio_text: Option<String>,
}

struct FmDemod {
    extractor: PhaseExtractor,
    resampler: AdaptiveResampler,
    deemph: DeemphasisFilter,
    volume: f32,
    #[allow(dead_code)]
    mpx_rate: f32,
    #[allow(dead_code)]
    decim_factor: usize,
    mpx_decimator: Decimator,
    stereo: StereoDecoderPLL,
    rds_resampler: RdsResamplerCustom,
    rds: RdsDecoder,
}

impl FmDemod {
    const RDS_TARGET_RATE: f32 = 171_000.0;
    const FM_BANDWIDTH: f32 = 240_000.0;

    fn new_rds_decoder() -> RdsDecoder {
        let mut rds = RdsDecoder::new(Self::RDS_TARGET_RATE, false);
        rds.set_print_json_output(false);
        rds
    }

    fn new(input_rate: u32, output_rate: u32) -> Result<Self, Box<dyn std::error::Error>> {
        let ratio = output_rate as f64 / input_rate as f64;
        let resampler = AdaptiveResampler::new(ratio, 1, 1)?;

        let decim_factor = ((input_rate as f32) / Self::FM_BANDWIDTH).round().max(1.0) as usize;
        let mpx_rate = input_rate as f32 / decim_factor as f32;

        Ok(Self {
            extractor: PhaseExtractor::new(),
            resampler,
            deemph: DeemphasisFilter::new(output_rate as f32, 50e-6),
            volume: 10.0,
            mpx_rate,
            decim_factor,
            mpx_decimator: Decimator::new(decim_factor),
            stereo: StereoDecoderPLL::new(mpx_rate),
            rds_resampler: RdsResamplerCustom::new(mpx_rate, Self::RDS_TARGET_RATE),
            rds: Self::new_rds_decoder(),
        })
    }

    #[allow(dead_code)]
    fn reset_rds(&mut self) {
        self.mpx_decimator = Decimator::new(self.decim_factor);
        self.stereo = StereoDecoderPLL::new(self.mpx_rate);
        self.rds_resampler = RdsResamplerCustom::new(self.mpx_rate, Self::RDS_TARGET_RATE);
        self.rds = Self::new_rds_decoder();
    }

    fn process(&mut self, bytes: &[u8]) -> Vec<f32> {
        let n = bytes.len() / 2;
        let mut iq: Vec<Complex<f32>> = Vec::with_capacity(n);
        for i in 0..n {
            let re = (bytes[2 * i] as f32 - 127.5) / 128.0;
            let im = (bytes[2 * i + 1] as f32 - 127.5) / 128.0;
            iq.push(Complex::new(re, im));
        }

        let phase = self.extractor.process(&iq);

        let phase_complex: Vec<Complex<f32>> =
            phase.iter().map(|&p| Complex::new(p, 0.0)).collect();
        let mpx: Vec<f32> = self
            .mpx_decimator
            .process(&phase_complex)
            .iter()
            .map(|c| c.re)
            .collect();
        let (_left, _right, pilot_phases) = self.stereo.process(&mpx);
        let (rds_i, rds_q) = self.rds_resampler.process_with_pilot(&mpx, &pilot_phases);
        if !rds_i.is_empty() {
            self.rds.process_iq(&rds_i, &rds_q);
        }

        let mut mono = phase;
        for p in mono.iter_mut() {
            *p /= std::f32::consts::PI;
        }

        let resampled = self.resampler.process(&mono);
        let mut audio = self.deemph.process(&resampled);
        for a in audio.iter_mut() {
            *a = (*a * self.volume).clamp(-1.0, 1.0);
        }

        audio
    }

    fn rds(&self) -> RdsInfo {
        RdsInfo {
            program_id: self.rds.program_id() as u32,
            program_type: self.rds.program_type(),
            station_name: self.rds.station_name(),
            radio_text: self.rds.radio_text(),
        }
    }
}
