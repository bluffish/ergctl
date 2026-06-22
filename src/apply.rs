//! The decision + apply engine and the persisted override flag.

use crate::config::{Config, StateCfg, DEFAULT_PATH};
use crate::sysfs;
use std::fs;

const STATE_DIR: &str = "/run/proart-power";
const STATE_FILE: &str = "/run/proart-power/mode";

fn config_path() -> String {
    std::env::var("PROART_POWER_CONFIG").unwrap_or_else(|_| DEFAULT_PATH.to_string())
}

/// Persisted override: "turbo" if explicitly set, otherwise "auto".
pub fn read_override() -> String {
    fs::read_to_string(STATE_FILE)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| s == "turbo")
        .unwrap_or_else(|| "auto".to_string())
}

fn write_override(mode: &str) {
    let _ = fs::create_dir_all(STATE_DIR);
    if let Err(e) = fs::write(STATE_FILE, mode) {
        eprintln!("proart-power: persist override: {e}");
    }
}

fn apply_state(label: &str, s: &StateCfg) {
    sysfs::set_platform_profile(&s.profile);
    sysfs::set_boost(s.boost);
    sysfs::set_epp(&s.epp);
    sysfs::cardwire_set(&s.gpu);
    // nvidia-powerd (Dynamic Boost) keeps the dGPU at D0; only run it when the
    // dGPU is actually available, so Integrated mode can reach D3cold.
    sysfs::set_service("nvidia-powerd", s.gpu != "integrated");
    println!(
        "[proart-power] {label}: profile={} gpu={} boost={} epp={}",
        s.profile, s.gpu, s.boost, s.epp
    );
}

/// Apply the coherent state for (override, power source). Idempotent.
pub fn apply_current() {
    let cfg = Config::load(&config_path());
    if let Some(cl) = cfg.charge_limit {
        sysfs::set_charge_limit(cl);
    }
    match read_override().as_str() {
        "turbo" => apply_state("turbo", &cfg.turbo),
        _ if sysfs::on_ac() => apply_state("ac", &cfg.ac),
        _ => apply_state("battery", &cfg.battery),
    }
}

pub fn set_mode_and_apply(mode: &str) {
    write_override(mode);
    apply_current();
}

pub fn status() {
    let dgpu = format!("{}/power/runtime_status", sysfs::DGPU_PCI);
    println!("override         : {}", read_override());
    println!(
        "AC online        : {}",
        if sysfs::on_ac() { "yes" } else { "no" }
    );
    println!(
        "platform_profile : {}",
        sysfs::read_trim("/sys/firmware/acpi/platform_profile").unwrap_or_default()
    );
    println!(
        "cpu boost        : {}",
        sysfs::read_trim("/sys/devices/system/cpu/cpufreq/boost").unwrap_or_default()
    );
    println!(
        "EPP (cpu0)       : {}",
        sysfs::read_trim("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference")
            .unwrap_or_default()
    );
    println!("GPU mode         : {}", sysfs::cardwire_get());
    println!(
        "dGPU power       : {}",
        sysfs::read_trim(&dgpu).unwrap_or_else(|| "?".into())
    );
    println!(
        "charge limit     : {}",
        sysfs::read_trim("/sys/class/power_supply/BAT1/charge_control_end_threshold")
            .unwrap_or_default()
    );
}
