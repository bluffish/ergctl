//! proart-power — single owner of the dynamic power "mode" on the ASUS ProArt P16.
//!
//! It reacts to one trigger (a udev power-source event / boot / resume), reads a
//! persisted override (auto|turbo), and applies one coherent state by driving
//! cardwire (GPU), the ACPI platform profile, CPU boost, EPP and charge limit.

mod apply;
mod config;
mod sysfs;

use std::process::{exit, Command};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("status");

    match cmd {
        "status" => apply::status(),
        "auto" => {
            ensure_root();
            apply::set_mode_and_apply("auto");
        }
        "turbo" => {
            ensure_root();
            apply::set_mode_and_apply("turbo");
        }
        "apply" => {
            ensure_root();
            apply::apply_current();
        }
        "-h" | "--help" | "help" => print_help(),
        "-V" | "--version" => println!("proart-power {}", env!("CARGO_PKG_VERSION")),
        other => {
            eprintln!("proart-power: unknown command '{other}'\n");
            print_help();
            exit(2);
        }
    }
}

fn print_help() {
    println!(
        "proart-power {}\n\n\
         USAGE:\n  proart-power <command>\n\n\
         COMMANDS:\n\
         \x20 auto     Hand control to automatic battery/AC switching (default)\n\
         \x20 turbo    Force max performance + dGPU, even on battery (sticky until 'auto')\n\
         \x20 status   Show current mode and live power state\n\
         \x20 apply    Re-apply the correct state for the current override + power source\n\
         \x20          (used by the systemd service on boot/resume/power events)\n\n\
         CONFIG:\n  /etc/proart-power.conf",
        env!("CARGO_PKG_VERSION")
    );
}

/// Effective UID from /proc/self/status (no libc dependency).
fn euid() -> u32 {
    if let Ok(s) = std::fs::read_to_string("/proc/self/status") {
        for line in s.lines() {
            if let Some(rest) = line.strip_prefix("Uid:") {
                if let Some(e) = rest.split_whitespace().nth(1) {
                    return e.parse().unwrap_or(1);
                }
            }
        }
    }
    1
}

/// Mutating commands need root. If we aren't root, re-exec the same invocation
/// under sudo so the user just gets a password prompt.
fn ensure_root() {
    if euid() == 0 {
        return;
    }
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("proart-power: cannot find own path: {e}");
            exit(1);
        }
    };
    let rest: Vec<String> = std::env::args().skip(1).collect();
    match Command::new("sudo").arg(exe).args(&rest).status() {
        Ok(s) => exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("proart-power: failed to re-exec under sudo: {e}");
            exit(1);
        }
    }
}
