use std::{
    collections::HashSet,
    fmt::format,
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use regex::Regex;
use tauri::{path::BaseDirectory, utils::config, AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

const RULE_FILE: &str = "/etc/udev/rules.d/99-AVIO.rules";

const TEMPLATE_FILENAME: &str = "99-AVIO.rules.template";

fn resolve_template_path(app: tauri::AppHandle) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let base = app.path().resolve("assets", BaseDirectory::Resource);
    if base.is_err() {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not resolve base path",
        )));
    }

    let template_path = base.unwrap().join("linux").join(TEMPLATE_FILENAME);

    Ok(template_path)
}

static CACHED_PHONE_VENDOR_IDS: Mutex<Option<HashSet<u16>>> = Mutex::new(None);

fn load_template(app: tauri::AppHandle) -> Result<String, Box<dyn std::error::Error>> {
    let template_path = resolve_template_path(app)?;
    let template_content = std::fs::read_to_string(template_path)?;
    Ok(template_content)
}

// Android vendor allowlist parsed from the udev template.
// Lines that also match an idProduct (dongle) are skipped.
pub fn phone_vendor_ids_from_udev_template(
    app: AppHandle,
) -> Result<HashSet<u16>, Box<dyn std::error::Error>> {
    {
        let cache = CACHED_PHONE_VENDOR_IDS.lock().unwrap();
        if let Some(ids) = &*cache {
            return Ok(ids.clone());
        }
    }

    let regex = Regex::new(r#"ATTR\{idVendor\}==\"([0-9a-fA-F]{4})\""#)?;
    let mut ids: HashSet<u16> = HashSet::new();

    let template = load_template(app)?;
    for line in template.lines() {
        if line.contains("ATTR{idProduct}") {
            continue;
        }
        if let Some(captures) = regex.captures(line) {
            if let Some(vendor_id_str) = captures.get(1) {
                if let Ok(vendor_id) = u16::from_str_radix(vendor_id_str.as_str(), 16) {
                    ids.insert(vendor_id);
                }
            }
        }
    }

    *CACHED_PHONE_VENDOR_IDS.lock().unwrap() = Some(ids.clone());

    Ok(ids)
}

fn template_marker(template: &str) -> String {
    let re = Regex::new(r"(?m)^# LIVI-RULE-VERSION=\d+$").unwrap();
    re.find(template)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "# LIVI-RULE-VERSION=0".to_string())
}

fn resolve_username() -> Result<String, Box<dyn std::error::Error>> {
    // Falls back through env vars first since they're cheap,
    // then asks the OS directly via libc/uid lookup.
    if let Ok(username) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        return Ok(username);
    }

    whoami::username().map_err(|e| e.into())
}

fn build_rule_content(app: &AppHandle) -> Result<String, Box<dyn std::error::Error>> {
    let template = load_template(app.clone())?;
    let username = resolve_username()?;
    let content = template.replace("__USERNAME__", &username);
    Ok(content)
}

pub fn udev_rule_exists() -> bool {
    std::path::Path::new(RULE_FILE).exists()
}

fn udev_rule_is_current(app: &AppHandle) -> Result<bool, Box<dyn std::error::Error>> {
    if !std::path::Path::new(RULE_FILE).exists() {
        return Ok(false);
    }

    let current_content = std::fs::read_to_string(RULE_FILE)?;
    let expected_content = template_marker(&load_template(app.clone())?);
    Ok(current_content.contains(&expected_content))
}

fn pkexec_available() -> bool {
    std::process::Command::new("which")
        .arg("pkexec")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

async fn install_rule(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let content = build_rule_content(app)?;
    let script = [
        format!("echo \"{}\" > {}", content.trim(), RULE_FILE),
        format!("udevadm control --reload-rules"),
        format!("udevadm trigger"),
    ]
    .join(" && ");

    let mut proc = std::process::Command::new("pkexec")
        .arg("bash")
        .arg("-c")
        .arg(script)
        .spawn()?;

    let code = proc.wait()?.code().unwrap_or(-1);
    if code != 0 {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("pkexec command failed with exit code {}", code),
        )));
    }

    Ok(())
}

pub async fn check_and_install_udev_rule(app: &AppHandle) -> bool {
    if std::env::consts::OS != "linux" {
        return false;
    }

    let exists = udev_rule_exists();
    let is_current = udev_rule_is_current(app).unwrap_or_else(|e| {
        eprintln!("Failed to check udev rule version: {:?}", e);
        false
    });

    if exists && is_current {
        return false;
    }

    if !pkexec_available() {
        eprintln!("pkexec is not available. Cannot install udev rule.");
        return false;
    }

    let is_upgrade = exists && !is_current;
    let title = if is_upgrade {
        "USB Permission Update"
    } else {
        "USB Permission Required"
    };
    let message = if is_upgrade {
        format!(
        "AVIO needs to update its udev rule for USB device access.\n\nThe existing rule at {} is outdated (wired Android Auto needs additional phone vendor entries). It will be replaced.",
        RULE_FILE
    )
    } else {
        format!(
            "AVIO needs permission to access USB devices.\n\nA udev rule will be installed to {}.",
            RULE_FILE
        )
    };

    let (button_a, button_b) = if is_upgrade {
        ("Update", "Skip")
    } else {
        ("Install", "Skip")
    };

    let (tx, rx) = oneshot::channel();

    app.dialog()
        .message(message)
        .kind(tauri_plugin_dialog::MessageDialogKind::Info)
        .title(title)
        .buttons(tauri_plugin_dialog::MessageDialogButtons::OkCancelCustom(
            button_a.to_string(),
            button_b.to_string(),
        ))
        .show(move |confirmed| {
            let _ = tx.send(confirmed);
        });

    let confirmed = rx.await.unwrap_or(false);

    if !confirmed {
        return false;
    }

    let mut installed = false;
    while !installed {
        let result = install_rule(app).await;
        if result.is_ok() {
            installed = true;
        }

        let (tx, rx) = oneshot::channel();
        app.dialog()
            .message("Could not install the udev rule. \n The authorization was cancelled or the password was wrong.")
            .kind(tauri_plugin_dialog::MessageDialogKind::Error)
            .title("Installation Failed")
            .buttons(tauri_plugin_dialog::MessageDialogButtons::OkCancelCustom(
                "Retry".to_string(),
                "Skip".to_string(),
            ))
            .show(move |confirmed| {
                let _ = tx.send(confirmed);
            });

        let confirmed = rx.await.unwrap_or(false);
        if !confirmed {
            break;
        }
    }

    let app_clone = app.clone();
    app.dialog()
        .message("udev rule installed. AVIO will now restart to apply it.")
        .kind(tauri_plugin_dialog::MessageDialogKind::Info)
        .title("Installation Complete")
        .buttons(tauri_plugin_dialog::MessageDialogButtons::Ok)
        .show(move |_| {
            app_clone.exit(0);
            app_clone.restart();
        });
    true
}
