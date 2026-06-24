//! Thin helpers for reading/writing the relevant sysfs knobs and driving cardwire.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;

pub const DGPU_PCI: &str = "/sys/bus/pci/devices/0000:64:00.0";
pub const RAPL_ENERGY: &str = "/sys/class/powercap/intel-rapl:0/energy_uj";

pub fn read_trim(path: &str) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn read_u64(path: &str) -> Option<u64> {
    read_trim(path).and_then(|s| s.parse().ok())
}

fn write_file(path: &str, val: &str) {
    match fs::OpenOptions::new().write(true).open(path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(val.as_bytes()) {
                eprintln!("ergctl: write {path} = {val}: {e}");
            }
        }
        Err(e) => eprintln!("ergctl: open {path}: {e}"),
    }
}

// ---------------------------------------------------------------- power source

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

/// First BAT* power supply directory.
pub fn bat_dir() -> Option<String> {
    fs::read_dir("/sys/class/power_supply")
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().starts_with("BAT"))
                .unwrap_or(false)
        })
        .map(|p| p.to_string_lossy().to_string())
}

// ------------------------------------------------------------------- CPU knobs

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
    for_each_cpu(|name| {
        let path =
            format!("/sys/devices/system/cpu/{name}/cpufreq/energy_performance_preference");
        if Path::new(&path).exists() {
            write_file(&path, epp);
        }
    });
}

fn for_each_cpu(mut f: impl FnMut(&str)) {
    if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            let is_core = name
                .strip_prefix("cpu")
                .map(|r| !r.is_empty() && r.chars().all(|c| c.is_ascii_digit()))
                .unwrap_or(false);
            if is_core {
                f(&name);
            }
        }
    }
}

/// Average current CPU frequency across cores, in MHz.
pub fn avg_cpu_mhz() -> u32 {
    let mut sum = 0u64;
    let mut n = 0u64;
    for_each_cpu(|name| {
        if let Some(khz) =
            read_u64(&format!("/sys/devices/system/cpu/{name}/cpufreq/scaling_cur_freq"))
        {
            sum += khz;
            n += 1;
        }
    });
    if n > 0 {
        (sum / n / 1000) as u32
    } else {
        0
    }
}

/// CPU package temperature (k10temp Tctl/Tccd), in °C.
pub fn cpu_temp_c() -> f64 {
    if let Some(d) = hwmon_by_name("k10temp") {
        for f in ["temp1_input", "temp2_input"] {
            if let Some(m) = read_u64(&format!("{d}/{f}")) {
                return m as f64 / 1000.0;
            }
        }
    }
    0.0
}

// ----------------------------------------------------------------------- hwmon

fn hwmon_by_name(target: &str) -> Option<String> {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for e in entries.flatten() {
            let p = e.path();
            if read_trim(&p.join("name").to_string_lossy()).as_deref() == Some(target) {
                return Some(p.to_string_lossy().to_string());
            }
        }
    }
    None
}

/// First nonzero fan reading across all hwmon nodes, in RPM.
pub fn fan_rpm() -> u32 {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for e in entries.flatten() {
            let p = e.path();
            for f in ["fan1_input", "fan2_input"] {
                if let Some(r) = read_u64(&p.join(f).to_string_lossy()) {
                    if r > 0 {
                        return r as u32;
                    }
                }
            }
        }
    }
    0
}

// ------------------------------------------------------------------ backlight

fn backlight_dir() -> Option<String> {
    let preferred = "/sys/class/backlight/amdgpu_bl1";
    if Path::new(preferred).exists() {
        return Some(preferred.to_string());
    }
    fs::read_dir("/sys/class/backlight")
        .ok()?
        .flatten()
        .next()
        .map(|e| e.path().to_string_lossy().to_string())
}

pub fn brightness_pct() -> u32 {
    if let Some(d) = backlight_dir() {
        let cur = read_u64(&format!("{d}/brightness")).unwrap_or(0) as f64;
        let max = read_u64(&format!("{d}/max_brightness")).unwrap_or(1).max(1) as f64;
        return ((cur / max) * 100.0).round() as u32;
    }
    0
}

pub fn set_brightness_pct(pct: u32) {
    if let Some(d) = backlight_dir() {
        let max = read_u64(&format!("{d}/max_brightness")).unwrap_or(1).max(1) as f64;
        let v = ((pct.min(100) as f64 / 100.0) * max).round() as u64;
        write_file(&format!("{d}/brightness"), &v.to_string());
    }
}

// ----------------------------------------------------------------- charge / EC

pub fn set_charge_limit(v: u32) {
    if let Ok(entries) = fs::read_dir("/sys/class/power_supply") {
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy();
            if !name.starts_with("BAT") {
                continue;
            }
            let path =
                format!("/sys/class/power_supply/{name}/charge_control_end_threshold");
            if Path::new(&path).exists() {
                write_file(&path, &v.to_string());
            }
        }
    }
}

// ----------------------------------------------------------------- services

pub fn service_active(name: &str) -> bool {
    Command::new("systemctl")
        .args(["is-active", "--quiet", name])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn dgpu_runtime_status() -> String {
    read_trim(&format!("{DGPU_PCI}/power/runtime_status")).unwrap_or_else(|| "?".into())
}
