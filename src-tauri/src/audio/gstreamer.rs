use std::path::PathBuf;

use tauri::{path::BaseDirectory, Manager};

pub enum VideoCodec {
    H264,
    H265,
}

fn platform_dir() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("macos-arm64"),
        ("linux", "aarch64") => Some("linux-arm64"),
        ("linux", "x86_64") => Some("linux-x64"),
        ("windows", "x86_64") => Some("windows-x64"),
        _ => None,
    }
}

pub fn resolve_gstreamer_root(app: &tauri::AppHandle) -> Option<PathBuf> {
    let dir = platform_dir()?;
    let base = app.path().resolve("assets", BaseDirectory::Resource);
    if base.is_err() {
        return None;
    }
    let bundles = base.unwrap().join("gstreamer").join(dir);
    Some(bundles)
}

pub fn resolve_binary(name: &str, app: &tauri::AppHandle) -> Option<PathBuf> {
    let root = resolve_gstreamer_root(app)?;
    let binary = root.join("bin").join(name);
    if binary.exists() {
        if cfg!(target_os = "windows") {
            let binary = binary.with_extension("exe");
            if binary.exists() {
                return Some(binary);
            }
        }
        Some(binary)
    } else {
        None
    }
}

/// Environment for spawning gst-device-monitor during device enumeration.
/// The bundled binary locates its own libs and plugins via RPATH + baked-in paths, so
/// we must not override GST_PLUGIN_PATH / GST_PLUGIN_SYSTEM_PATH / LD_LIBRARY_PATH here —
/// those overrides break the device providers (PipeWire/PulseAudio) that need system libs.
/// On Windows the binary still needs its bin dir on PATH to find DLLs.
pub fn gst_env_for_enum(gst_root: &PathBuf) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = std::env::vars().collect();

    let scanner = if std::env::consts::OS == "windows" {
        gst_root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner.exe")
    } else {
        gst_root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner")
    };
    if scanner.exists() {
        env.push((
            "GST_PLUGIN_SCANNER".to_owned(),
            scanner.to_str().unwrap_or_default().to_owned(),
        ));
    }

    if std::env::consts::OS == "windows" {
        env.push((
            "PATH".to_owned(),
            gst_root.join("bin").to_str().unwrap_or_default().to_owned(),
        ));
    }

    env
}

pub fn gst_env(gst_root: &PathBuf) -> Vec<(String, String)> {
    let mut env = vec![];
    let plugin_path = gst_root.join("lib").join("gstreamer-1.0");
    let plugin_scanner = if std::env::consts::OS == "windows" {
        gst_root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner.exe")
    } else {
        gst_root
            .join("libexec")
            .join("gstreamer-1.0")
            .join("gst-plugin-scanner")
    };

    let current_env = std::env::vars().collect::<Vec<(String, String)>>();

    env.extend(current_env);

    let registry = std::env::temp_dir().join("avio-gstreamer-registry.bin");

    env.push(("GST_PLUGIN_SYSTEM_PATH".to_owned(), "".to_owned()));
    env.push((
        "GST_REGISTRY".to_owned(),
        registry.to_str().unwrap_or_default().to_owned(),
    ));
    env.push((
        "GST_PLUGIN_PATH".to_owned(),
        plugin_path.to_str().unwrap_or_default().to_owned(),
    ));
    env.push((
        "GST_PLUGIN_SCANNER".to_owned(),
        plugin_scanner.to_str().unwrap_or_default().to_owned(),
    ));

    if std::env::consts::OS == "macos" {
        env.push((
            "DYLD_LIBRARY_PATH".to_owned(),
            gst_root.join("lib").to_str().unwrap_or_default().to_owned(),
        ));
    } else if std::env::consts::OS == "linux" {
        env.push((
            "LD_LIBRARY_PATH".to_owned(),
            gst_root.join("lib").to_str().unwrap_or_default().to_owned(),
        ));
    } else if std::env::consts::OS == "windows" {
        env.push((
            "PATH".to_owned(),
            gst_root.join("bin").to_str().unwrap_or_default().to_owned(),
        ));
    }

    env
}

pub fn audio_sink_element() -> &'static str {
    if std::env::consts::OS == "macos" {
        "osxaudiosink"
    } else if std::env::consts::OS == "linux" {
        "pulsesink"
    } else if std::env::consts::OS == "windows" {
        "wasapisink"
    } else {
        "autoaudiosink"
    }
}

pub fn audio_source_element() -> &'static str {
    if std::env::consts::OS == "macos" {
        "osxaudiosrc"
    } else if std::env::consts::OS == "linux" {
        "pulsesrc"
    } else if std::env::consts::OS == "windows" {
        "wasapisrc"
    } else {
        "autoaudiosrc"
    }
}

pub fn audio_device_prop() -> &'static str {
    if std::env::consts::OS == "macos" {
        "unique-id"
    } else if std::env::consts::OS == "linux" {
        "device"
    } else if std::env::consts::OS == "windows" {
        "device-name"
    } else {
        "device"
    }
}

pub fn video_parse_element(codec: VideoCodec) -> &'static str {
    match codec {
        VideoCodec::H264 => "h264parse",
        VideoCodec::H265 => "h265parse",
    }
}

pub fn video_decoder_element(codec: VideoCodec) -> &'static str {
    if std::env::consts::OS == "macos" {
        return "vtdec";
    } else if std::env::consts::OS == "windows" {
        return match codec {
            VideoCodec::H264 => "d3d11h264dec",
            VideoCodec::H265 => "d3d11h265dec",
        };
    } else {
        return match codec {
            VideoCodec::H264 => "v4l2slh264dec",
            VideoCodec::H265 => "v4l2slh265dec",
        };
    }
}

pub fn video_sink_element() -> &'static str {
    if std::env::consts::OS == "macos" {
        "glimagesink"
    } else if std::env::consts::OS == "linux" {
        "glimagesink"
    } else if std::env::consts::OS == "windows" {
        "d3d11videosink"
    } else {
        "glimagesink"
    }
}
