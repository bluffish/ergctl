//! ratatui cockpit: live draw + status, with one-key control of the core knobs.

use crate::{apply, audioguard, config, readers, readers::Snapshot, readers::Services, sysfs};
use std::io;
use std::time::{Duration, Instant};

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Sparkline},
    DefaultTerminal, Frame,
};

const HISTORY: usize = 240;

pub fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new();
    let res = app.main_loop(&mut terminal);
    ratatui::restore();
    res
}

struct App {
    snap: Snapshot,
    svc: Services,   // refreshed every ~10 ticks (systemctl forks are costly)
    svc_ticks: u32,
    history: Vec<u64>, // draw in centi-watts, for the sparkline
    soc_w: f64,
    last_rapl: u64,
    last_tick: Instant,
    quit: bool,
}

impl App {
    fn new() -> App {
        let snap = readers::read();
        let last_rapl = snap.rapl_uj;
        App {
            snap,
            svc: readers::read_services(),
            svc_ticks: 0,
            history: Vec::new(),
            soc_w: 0.0,
            last_rapl,
            last_tick: Instant::now(),
            quit: false,
        }
    }

    fn main_loop(&mut self, term: &mut DefaultTerminal) -> io::Result<()> {
        loop {
            term.draw(|f| self.ui(f))?;
            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(k) = event::read()? {
                    if k.kind == KeyEventKind::Press {
                        self.on_key(k.code);
                    }
                }
            }
            if self.last_tick.elapsed() >= Duration::from_secs(1) {
                self.tick();
            }
            if self.quit {
                return Ok(());
            }
        }
    }

    fn tick(&mut self) {
        let elapsed = self.last_tick.elapsed().as_secs_f64();
        self.last_tick = Instant::now();
        let snap = readers::read();
        if snap.rapl_uj >= self.last_rapl && elapsed > 0.0 {
            self.soc_w = (snap.rapl_uj - self.last_rapl) as f64 / 1e6 / elapsed;
        }
        self.last_rapl = snap.rapl_uj;
        self.history.push((snap.draw_w * 100.0) as u64);
        if self.history.len() > HISTORY {
            self.history.remove(0);
        }
        self.snap = snap;
        // Refresh service health roughly every 10s, not every tick.
        self.svc_ticks = self.svc_ticks.wrapping_add(1);
        if self.svc_ticks % 10 == 0 {
            self.svc = readers::read_services();
        }
    }

    fn refresh(&mut self) {
        self.snap = readers::read();
    }

    fn on_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('a') => {
                apply::set_mode_and_apply("auto");
                self.refresh();
            }
            KeyCode::Char('t') => {
                apply::set_mode_and_apply("turbo");
                self.refresh();
            }
            KeyCode::Char('b') => {
                sysfs::set_boost(!self.snap.boost);
                self.refresh();
            }
            KeyCode::Char('p') => {
                let next = match self.snap.profile.as_str() {
                    "quiet" => "balanced",
                    "balanced" => "performance",
                    _ => "quiet",
                };
                sysfs::set_platform_profile(next);
                self.refresh();
            }
            KeyCode::Char('g') => {
                // Toggle audio-guard. Turning it OFF rescans the bus, which wakes
                // the dGPU — fine for an explicit keypress.
                if self.snap.audio_guard {
                    audioguard::off();
                } else {
                    audioguard::on();
                }
                self.refresh();
            }
            KeyCode::Char(']') => self.bump_charge(5),
            KeyCode::Char('[') => self.bump_charge(-5),
            KeyCode::Char('=') | KeyCode::Char('+') => {
                sysfs::set_brightness_pct((self.snap.brightness_pct + 10).min(100));
                self.refresh();
            }
            KeyCode::Char('-') | KeyCode::Char('_') => {
                sysfs::set_brightness_pct(self.snap.brightness_pct.saturating_sub(10).max(5));
                self.refresh();
            }
            _ => {}
        }
    }

    fn bump_charge(&mut self, delta: i32) {
        let v = (self.snap.charge_limit as i32 + delta).clamp(20, 100) as u32;
        let _ = config::set_key(&config::path(), "charge_limit", &v.to_string());
        sysfs::set_charge_limit(v);
        self.refresh();
    }

    // --------------------------------------------------------------------- UI

    fn ui(&self, f: &mut Frame) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(4),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(f.area());

        self.header(f, rows[0]);

        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(rows[1]);
        let left = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body[0]);
        let right = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(body[1]);

        self.cpu_panel(f, left[0]);
        self.battery_panel(f, left[1]);
        self.gpu_panel(f, right[0]);
        self.system_panel(f, right[1]);
        self.footer(f, rows[2]);
    }

    fn header(&self, f: &mut Frame, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
            .split(area);

        let s = &self.snap;
        let turbo = s.override_mode == "turbo";
        let auto_style = mode_style(!turbo);
        let turbo_style = mode_style(turbo);
        let src = if s.ac_online { "AC" } else { "battery" };

        let left = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    "ergctl",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("  power cockpit"),
            ]),
            Line::from(vec![
                Span::raw("mode  "),
                Span::styled(" AUTO ", auto_style),
                Span::raw(" "),
                Span::styled(" TURBO ", turbo_style),
            ]),
            Line::from(format!("{src}   {}%  {}", s.capacity, s.bat_status)),
        ])
        .block(Block::bordered());
        f.render_widget(left, cols[0]);

        let title = format!("draw {:.1} W    SoC {:.1} W", s.draw_w, self.soc_w);
        let spark = Sparkline::default()
            .block(Block::bordered().title(title))
            .data(&self.history)
            .style(Style::default().fg(Color::Green));
        f.render_widget(spark, cols[1]);
    }

    fn cpu_panel(&self, f: &mut Frame, area: Rect) {
        let s = &self.snap;
        let p = Paragraph::new(vec![
            kv("EPP", &s.epp),
            kv("boost", if s.boost { "on" } else { "off" }),
            kv("governor", &s.governor),
            kv("freq", &format!("{} MHz", s.cpu_mhz)),
            kv("temp", &format!("{:.0} °C", s.cpu_temp)),
            kv("SoC pkg", &format!("{:.1} W", self.soc_w)),
        ])
        .block(Block::bordered().title("CPU"));
        f.render_widget(p, area);
    }

    fn gpu_panel(&self, f: &mut Frame, area: Rect) {
        let s = &self.snap;
        let dgpu = if s.dgpu_awake() {
            format!("{} (awake)", s.dgpu_state)
        } else {
            format!("{} (asleep)", s.dgpu_state)
        };
        let p = Paragraph::new(vec![
            kv("dGPU", &dgpu),
            kv("RTX 4070", if s.dgpu_awake() { "drawing power" } else { "D3cold (off)" }),
            kv("cardwire", &s.cardwire_mode),
            kv("last wake", s.dgpu_waker.as_deref().unwrap_or("— (watcher off)")),
            kv("pm", "RTD3 + guards"),
        ])
        .block(Block::bordered().title("GPU"));
        f.render_widget(p, area);
    }

    fn battery_panel(&self, f: &mut Frame, area: Rect) {
        let s = &self.snap;
        let left = if s.time_left_h > 0.0 {
            let h = s.time_left_h as u32;
            let m = ((s.time_left_h - h as f64) * 60.0) as u32;
            format!("~{h}h{m:02}m left")
        } else {
            "—".to_string()
        };
        let p = Paragraph::new(vec![
            Line::from(vec![
                Span::raw("remaining  "),
                Span::styled(
                    format!("{:.1} Wh", s.energy_now),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("  ({}%)", s.capacity)),
            ]),
            Line::from(vec![
                Span::raw("charge limit  "),
                Span::styled(
                    format!("{}%", s.charge_limit),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("   [ / ] adjust"),
            ]),
            kv(
                "health",
                &format!(
                    "{:.0}% ({:.1}/{:.1} Wh)",
                    s.health_pct(),
                    s.energy_full,
                    s.energy_design
                ),
            ),
            kv("cycles", &s.cycles.to_string()),
            kv("estimate", &left),
        ])
        .block(Block::bordered().title("BATTERY"));
        f.render_widget(p, area);
    }

    fn system_panel(&self, f: &mut Frame, area: Rect) {
        let s = &self.snap;
        let stack = Line::from(vec![
            Span::raw("stack  "),
            ok_span("tlp", self.svc.tlp),
            Span::raw(" "),
            ok_span("asusd", self.svc.asusd),
        ]);
        let p = Paragraph::new(vec![
            kv("profile", &s.profile),
            kv("fan", &format!("{} rpm", s.fan_rpm)),
            kv("backlight", &format!("{}%", s.brightness_pct)),
            kv(
                "guards",
                &format!(
                    "gpu={} audio={}",
                    if s.gpu_guard { "on" } else { "off" },
                    if s.audio_guard { "on" } else { "off" }
                ),
            ),
            stack,
        ])
        .block(Block::bordered().title("SYSTEM"));
        f.render_widget(p, area);
    }

    fn footer(&self, f: &mut Frame, area: Rect) {
        let help = "[a]uto [t]urbo  [p]rofile [b]oost  [g] audio-guard  [ ] charge  -/= bright  [q]uit";
        let p = Paragraph::new(Line::from(help))
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::bordered());
        f.render_widget(p, area);
    }
}

fn kv<'a>(k: &'a str, v: &str) -> Line<'a> {
    Line::from(format!("{k:<10} {v}"))
}

fn mode_style(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn ok_span(label: &str, ok: bool) -> Span<'_> {
    let (mark, color) = if ok { ("✓", Color::Green) } else { ("✗", Color::Red) };
    Span::styled(format!("{label}{mark}"), Style::default().fg(color))
}
