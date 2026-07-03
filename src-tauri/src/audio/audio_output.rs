use std::cmp::max;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::audio::gstreamer::{
    audio_device_prop, audio_sink_element, gst_env, resolve_gstreamer_root,
};

pub enum AudioOutputMode {
    Music,
    Realtime,
}

pub struct AudioOutputOptions {
    pub sample_rate: i32,
    pub channels: i32,
    pub mode: Option<AudioOutputMode>,
    pub device: Option<String>,
}

pub struct AudioOutput {
    stop_grace_ms: u64,
    process: Option<tokio::process::Child>,
    stdin: Option<tokio::process::ChildStdin>,
    sample_rate: i32,
    channels: i32,
    mode: AudioOutputMode,
    bytes_written: usize,
    queue: Vec<Vec<u8>>,
    writing: bool,
    write_seq: u64,
    device: Option<String>,
}

impl AudioOutput {
    pub fn new(opts: AudioOutputOptions) -> Self {
        let channels = max(1, opts.channels);
        AudioOutput {
            stop_grace_ms: 500,
            channels: channels,
            mode: opts
                .mode
                .unwrap_or(Self::infer_mode(opts.sample_rate, channels)),
            sample_rate: opts.sample_rate,
            device: opts.device,
            process: None,
            stdin: None,
            bytes_written: 0,
            queue: vec![],
            writing: false,
            write_seq: 0,
        }
    }

    fn set_device(&mut self, device: Option<String>) {
        self.device = device;
    }

    pub async fn start(&mut self, app: &tauri::AppHandle) {
        self.kill_immediate().await;

        let os = std::env::consts::OS;
        if os != "macos" && os != "linux" && os != "windows" {
            eprintln!("[AudioOutput] Unsupported platform");
            return;
        }

        let gst_root = match resolve_gstreamer_root(app) {
            Some(r) => r,
            None => {
                eprintln!("[AudioOutput] Bundled GStreamer not found");
                return;
            }
        };

        let exe = if os == "windows" {
            "gst-launch-1.0.exe"
        } else {
            "gst-launch-1.0"
        };
        let cmd = gst_root.join("bin").join(exe);
        let args = self.build_args();
        let env = gst_env(&gst_root);

        self.bytes_written = 0;
        self.queue.clear();
        self.writing = false;
        self.write_seq = 0;

        let mut command = tokio::process::Command::new(&cmd);
        command
            .args(&args)
            .envs(env)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[AudioOutput] Failed to spawn: {e}");
                return;
            }
        };

        let stdin = child.stdin.take();
        let stderr = child.stderr.take();

        self.process = Some(child);
        self.stdin = stdin;

        if let Some(stderr) = stderr {
            tauri::async_runtime::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    eprintln!("[AudioOutput] STDERR: {}", line.trim());
                }
            });
        }
    }

    pub async fn write(&mut self, chunk: Vec<i16>) {
        let proc = self.process.as_mut();
        if proc.is_none() {
            return;
        }

        self.queue
            .push(chunk.iter().flat_map(|&x| x.to_le_bytes()).collect());

        if !self.writing {
            self.flush_queue().await;
        }
    }

    pub async fn stop(&mut self) {
        if self.process.is_none() {
            return;
        }

        let mut proc = self.process.take().unwrap();
        self.end_stdin().await;

        let result = tokio::time::timeout(
            std::time::Duration::from_millis(self.stop_grace_ms),
            proc.wait(),
        )
        .await;

        match result {
            Ok(Ok(_)) => {
                // Process exited gracefully
            }
            Ok(Err(e)) => {
                eprintln!("Error waiting for audio output process: {:?}", e);
            }
            Err(_) => {
                // Timeout reached, kill the process
                let _ = proc.kill().await;
            }
        }

        self.cleanup();
    }

    async fn dispose(&mut self) {
        self.kill_immediate().await;
    }

    async fn flush_queue(&mut self) {
        if self.process.is_none() {
            self.queue.clear();
            self.writing = false;
            return;
        }
        self.writing = true;

        let chunks: Vec<Vec<u8>> = self.queue.drain(..).collect();
        for buf in chunks {
            let Some(stdin) = self.stdin.as_mut() else {
                break;
            };
            self.bytes_written += buf.len();
            if stdin.write_all(&buf).await.is_err() {
                self.writing = false;
                return;
            }
        }
        self.writing = false;
    }

    fn build_args(&self) -> Vec<String> {
        let is_realtime = matches!(self.mode, AudioOutputMode::Realtime);

        let input_queue_args = if is_realtime {
            vec![
                "queue",
                "max-size-time=40000000",
                "max-size-bytes=0",
                "max-size-buffers=0",
            ]
        } else {
            vec![
                "queue",
                "max-size-time=350000000",
                "max-size-bytes=0",
                "max-size-buffers=0",
            ]
        };

        let output_queue_args = if is_realtime {
            vec![
                "queue",
                "max-size-time=20000000",
                "max-size-bytes=0",
                "max-size-buffers=0",
            ]
        } else {
            vec![
                "queue",
                "max-size-time=200000000",
                "max-size-bytes=0",
                "max-size-buffers=0",
            ]
        };

        let sink = audio_sink_element();

        let mut sink_args: Vec<String> = if is_realtime {
            vec![sink.to_owned(), "sync=false".to_string()]
        } else if sink == "osxaudiosink" {
            vec![sink.to_owned()]
        } else {
            vec![
                sink.to_owned(),
                "buffer-time=300000".to_string(),
                "latency-time=30000".to_string(),
            ]
        };

        if let Some(device) = &self.device {
            sink_args.push(format!("{}={}", audio_device_prop(), device));
        }

        fn p(args: &mut Vec<String>, s: &str) {
            args.push(s.to_string());
        }

        let mut args: Vec<String> = Vec::new();

        p(&mut args, "fdsrc");
        p(&mut args, "fd=0");
        p(&mut args, "!");
        args.extend(input_queue_args.iter().map(|s| s.to_string()));
        p(&mut args, "!");
        p(&mut args, "rawaudioparse");
        p(&mut args, "format=pcm");
        p(&mut args, "pcm-format=s16le");
        args.push(format!("sample-rate={}", self.sample_rate));
        args.push(format!("num-channels={}", self.channels));
        p(&mut args, "!");
        p(&mut args, "audioconvert");
        p(&mut args, "!");
        p(&mut args, "audioresample");
        if std::env::consts::OS != "windows" {
            p(&mut args, "!");
            p(&mut args, "audio/x-raw,format=S16LE,rate=48000,channels=2");
        }
        p(&mut args, "!");
        args.extend(output_queue_args.iter().map(|s| s.to_string()));
        p(&mut args, "!");
        args.extend(sink_args);

        args
    }

    fn cleanup(&mut self) {
        self.process = None;
        self.bytes_written = 0;
        self.queue.clear();
        self.writing = false;
        self.write_seq = 0;
    }

    async fn kill_immediate(&mut self) {
        if self.process.is_none() {
            return;
        }

        let mut proc = self.process.take().unwrap();
        self.end_stdin().await;

        let result = proc.kill().await;

        if result.is_err() {
            eprintln!("Failed to kill audio output process: {:?}", result.err());
        }

        self.cleanup();
    }

    async fn end_stdin(&mut self) {
        if let Some(stdin) = self.stdin.as_mut() {
            let _ = stdin.flush().await;
            let _ = stdin.shutdown().await;
        }
        self.stdin = None;
    }

    fn infer_mode(sample_rate: i32, channels: i32) -> AudioOutputMode {
        if channels == 1 {
            return AudioOutputMode::Realtime;
        } else if sample_rate <= 24000 {
            return AudioOutputMode::Realtime;
        } else {
            return AudioOutputMode::Music;
        }
    }
}
