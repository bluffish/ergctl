//! Flat `key = value` config (no TOML dependency). One file, declarative.

use std::collections::HashMap;
use std::fs;

/// The knobs that make up one power state. (GPU power is handled by NVIDIA RTD3 +
/// the audio/gpu guards, not by ergctl switching a GPU mode — so no gpu field.)
pub struct StateCfg {
    pub profile: String,                  // ACPI platform_profile: quiet|balanced|performance
    pub boost: bool,                      // CPU turbo
    pub epp: String,                      // energy_performance_preference
    pub ppt: Option<(u32, u32, u32)>,     // asus-armoury SoC power: (SPL, SPPT, FPPT) watts; None = leave
    pub brightness: Option<u32>,          // OLED backlight %, None = leave alone
}

pub struct Config {
    pub charge_limit: Option<u32>,
    pub battery: StateCfg,
    pub ac: StateCfg,
    pub turbo: StateCfg,
}

pub const DEFAULT_PATH: &str = "/etc/ergctl.conf";

/// Active config path (overridable via ERGCTL_CONFIG, for testing).
pub fn path() -> String {
    std::env::var("ERGCTL_CONFIG").unwrap_or_else(|_| DEFAULT_PATH.to_string())
}

impl Config {
    pub fn load(path: &str) -> Config {
        let mut m: HashMap<String, String> = HashMap::new();
        if let Ok(text) = fs::read_to_string(path) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    m.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
        }

        let s = |k: &str, d: &str| m.get(k).cloned().unwrap_or_else(|| d.to_string());
        let b = |k: &str, d: bool| {
            m.get(k)
                .map(|v| matches!(v.as_str(), "true" | "1" | "on" | "yes"))
                .unwrap_or(d)
        };
        // PPT: "SPL SPPT FPPT" (3 ints) -> Some((..)); empty/"off"/absent uses default.
        let ppt = |k: &str, d: Option<(u32, u32, u32)>| match m.get(k).map(|v| v.trim()) {
            Some("") | Some("off") | Some("none") => None,
            Some(v) => {
                let p: Vec<u32> = v.split_whitespace().filter_map(|x| x.parse().ok()).collect();
                if p.len() == 3 {
                    Some((p[0], p[1], p[2]))
                } else {
                    d
                }
            }
            None => d,
        };
        // brightness %: int -> Some; empty/"off"/absent uses default.
        let pct = |k: &str, d: Option<u32>| match m.get(k).map(|v| v.trim()) {
            Some("") | Some("off") | Some("none") => None,
            Some(v) => v.parse().ok().map(Some).unwrap_or(d),
            None => d,
        };

        Config {
            charge_limit: m.get("charge_limit").and_then(|v| v.parse().ok()),
            battery: StateCfg {
                profile: s("battery_profile", "quiet"),
                boost: b("battery_boost", false),
                epp: s("battery_epp", "power"),
                ppt: ppt("battery_ppt", Some((15, 35, 35))),
                brightness: pct("battery_brightness", Some(40)),
            },
            ac: StateCfg {
                profile: s("ac_profile", "balanced"),
                boost: b("ac_boost", true),
                epp: s("ac_epp", "balance_performance"),
                ppt: ppt("ac_ppt", Some((80, 80, 80))),
                brightness: pct("ac_brightness", None),
            },
            turbo: StateCfg {
                profile: s("turbo_profile", "performance"),
                boost: b("turbo_boost", true),
                epp: s("turbo_epp", "performance"),
                ppt: ppt("turbo_ppt", Some((80, 80, 80))),
                brightness: pct("turbo_brightness", None),
            },
        }
    }
}

/// Update (or append) a single `key = value` line in the config file, preserving
/// comments and other keys. Used by the TUI to persist e.g. the charge limit.
pub fn set_key(path: &str, key: &str, val: &str) -> std::io::Result<()> {
    let mut lines: Vec<String> = fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let mut found = false;
    for line in lines.iter_mut() {
        if line.trim_start().starts_with('#') {
            continue;
        }
        if let Some((k, _)) = line.split_once('=') {
            if k.trim() == key {
                *line = format!("{key} = {val}");
                found = true;
                break;
            }
        }
    }
    if !found {
        lines.push(format!("{key} = {val}"));
    }
    fs::write(path, lines.join("\n") + "\n")
}
