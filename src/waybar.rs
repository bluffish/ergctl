//! Minimal Waybar custom-module output: power draw + dGPU state, as JSON.
//! Read-only (sysfs only — never wakes the dGPU); no root needed.

use crate::sysfs;
use std::io::Write;
use std::thread;
use std::time::Duration;

// Nerd Font glyphs
const BOLT: &str = "\u{f0e7}"; //
const PLUG: &str = "\u{f1e6}"; //
const CHIP: &str = "\u{f2db}"; //

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

fn line() -> String {
    let ac = sysfs::on_ac();
    let bat = sysfs::bat_dir();
    let cur = bat
        .as_ref()
        .and_then(|d| sysfs::read_u64(&format!("{d}/current_now")))
        .unwrap_or(0) as f64;
    let volt = bat
        .as_ref()
        .and_then(|d| sysfs::read_u64(&format!("{d}/voltage_now")))
        .unwrap_or(0) as f64;
    let draw = cur * volt / 1e12;

    let dgpu = sysfs::dgpu_runtime_status(); // "suspended" = asleep
    let asleep = dgpu == "suspended";
    let gpu_txt = if asleep { "off" } else { "ON" };
    let class = if asleep { "dgpu-off" } else { "dgpu-on" };

    let power = if ac {
        format!("{PLUG} AC")
    } else {
        format!("{BOLT} {draw:.1}W")
    };
    let text = format!("{power} · {CHIP} {gpu_txt}");

    let tooltip = if ac {
        format!("on AC\ndGPU: {dgpu} ({})", if asleep { "asleep" } else { "awake" })
    } else {
        format!(
            "draw {draw:.1} W\ndGPU: {dgpu} ({})",
            if asleep { "asleep" } else { "awake" }
        )
    };

    format!(
        "{{\"text\":\"{}\",\"tooltip\":\"{}\",\"class\":\"{}\"}}",
        json_escape(&text),
        json_escape(&tooltip),
        class
    )
}

/// Emit one JSON object, or (with watch) a line every 4s on stdout.
pub fn emit(watch: bool) {
    if !watch {
        println!("{}", line());
        return;
    }
    loop {
        println!("{}", line());
        let _ = std::io::stdout().flush();
        thread::sleep(Duration::from_secs(4));
    }
}
