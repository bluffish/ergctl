//! Thin helpers for reading/writing the relevant sysfs knobs and driving cardwire.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub const DGPU_PCI: &str = "/sys/bus/pci/devices/0000:64:00.0";

pub fn read_trim(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn write_file(path: &str, val: &str) {
    match fs::OpenOptions::new().write(true).open(path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(val.as_bytes()) {
                eprintln!("proart-power: write {path} = {val}: {e}");
            }
        }
        Err(e) => eprintln!("proart-power: open {path}: {e}"),
    }
}

/// True if any "Mains" power supply reports online.
pub fn on_ac() -> bool {
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for e in entries.flatten() {
            let p = e.path();
            if read_trim(&p.join("type").to_string_lossy()).as_deref() == Some("Mains")
                && read_trim(&p.join("online").to_string_lossy()).as_deref() == Some("1")
            {
                return true;
            }
        }
    }
    false
}

pub fn set_platform_profile(p: &str) {
    let path = "/sys/firmware/acpi/platform_profile";
    if Path::new(path).exists() {
        write_file(path, p);
    }
}

pub fn set_boost(on: bool) {
    let path = "/sys/devices/system/cpu/cpufreq/boost";
    if Path::new(path).exists() {
        write_file(path, if on { "1" } else { "0" });
    }
}

/// Apply EPP to every cpuN core that exposes the attribute.
pub fn set_epp(epp: &str) {
    if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            let is_core = name
                .strip_prefix("cpu")
                .map(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            if !is_core {
                continue;
            }
            let path = format!(
                "/sys/devices/system/cpu/{name}/cpufreq/energy_performance_preference"
            );
            if Path::new(&path).exists() {
                write_file(&path, epp);
            }
        }
    }
}

pub fn set_charge_limit(v: u32) {
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("BAT") {
                continue;
            }
            let path = format!(
                "/sys/class/power_supply/{name}/charge_control_end_threshold"
            );
            if Path::new(&path).exists() {
                write_file(&path, &v.to_string());
            }
        }
    }
}

pub fn cardwire_set(mode: &str) {
    if let Err(e) = Command::new("cardwire").arg("set").arg(mode).status() {
        eprintln!("proart-power: cardwire set {mode}: {e}");
    }
}

/// Start/stop a systemd unit. Used for nvidia-powerd, which otherwise holds the
/// dGPU at D0 even with no clients (defeats D3cold on battery).
pub fn set_service(name: &str, want_active: bool) {
    let action = if want_active { "start" } else { "stop" };
    if let Err(e) = Command::new("systemctl").arg(action).arg(name).status() {
        eprintln!("proart-power: systemctl {action} {name}: {e}");
    }
}

pub fn cardwire_get() -> String {
    Command::new("cardwire")
        .arg("get")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(str::to_string))
        .map(|l| l.replace("Current Mode:", "").trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
