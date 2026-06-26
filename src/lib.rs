//! ergctl core — shared by the CLI and the TUI frontends.
//!
//! ergctl is the single owner of the dynamic power "mode" on the ASUS ProArt P16.
//! It reacts to one trigger (udev power event / boot / resume), reads a persisted
//! override (auto|turbo), and applies one coherent state by driving cardwire
//! (GPU), the ACPI platform profile, CPU boost, EPP and charge limit.

pub mod apply;
pub mod audioguard;
pub mod config;
pub mod dgpuwatch;
pub mod gpuguard;
pub mod readers;
pub mod sysfs;
pub mod tui;
pub mod waybar;
