#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("shim.h");

        type WelleIoBridge;
        type RadioReceiver;
        type mot_file_t;

        fn new_welle_io_bridge() -> UniquePtr<WelleIoBridge>;
        fn ping(self: &WelleIoBridge) -> i32;
        unsafe fn setReceiver(self: Pin<&mut WelleIoBridge>, receiver: *mut RadioReceiver);
        fn setSnrCallback(self: Pin<&mut WelleIoBridge>, callback: extern "C" fn(f32));
        fn setSignalPresenceCallback(self: Pin<&mut WelleIoBridge>, callback: extern "C" fn(bool));
        fn setServiceDetectedCallback(self: Pin<&mut WelleIoBridge>, callback: extern "C" fn(u32, String));
        fn setNewAudioCallback(self: Pin<&mut WelleIoBridge>, callback: extern "C" fn(Vec<i16>, i32, bool));
        fn setNewDynamicLabelCallback(self: Pin<&mut WelleIoBridge>, callback: extern "C" fn(String));
        fn setMotCallback(self: Pin<&mut WelleIoBridge>, callback: extern "C" fn(motfile: &mot_file_t));
    }
}

pub fn demo_ping() -> i32 {
    let bridge = ffi::new_welle_io_bridge();
    bridge.ping()
}
