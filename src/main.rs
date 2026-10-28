//! matrixstat: iStat-style macOS menu bar monitor for CPU cores, memory, disk and network.

mod config;
mod mac;
mod menu;
mod render;
mod stats;
mod theme;

use std::time::{Duration, Instant};

use tao::event::{Event, StartCause};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::platform::macos::{ActivationPolicy, EventLoopExtMacOS};
use tray_icon::menu::MenuEvent;
use tray_icon::{TrayIcon, TrayIconBuilder};

use config::Config;
use menu::StatsMenu;
use stats::{Sampler, Snapshot};
use theme::Tone;

/// Sampling period; iStat defaults to 2s for CPU.
const INTERVAL: Duration = Duration::from_secs(2);

enum UserEvent {
    Menu(MenuEvent),
}

/// `--once`: sample twice and dump the snapshot as text, for testing without a GUI.
fn print_once() {
    let mut sampler = Sampler::new();
    // Per-process CPU on macOS needs ~1s between refreshes to be non-zero.
    std::thread::sleep(Duration::from_secs(1));
    let s = sampler.sample();
    println!("title: {}", title(&s, &Config::load()));
    println!(
        "cpu total {:.1}%  load {:.2} {:.2} {:.2}",
        s.cpu_total, s.load[0], s.load[1], s.load[2]
    );
    for (i, c) in s.cores.iter().enumerate() {
        println!("  core {:<2} {} {:>3.0}%", i + 1, render::bar(*c, 12), c);
    }
    println!(
        "mem used {} / {} ({:.0}%)  app {} wired {} compressed {} cached {} free {}  pressure {:?}",
        render::bytes(s.vm.used()),
        render::bytes(s.mem_total),
        s.mem_used_pct(),
        render::bytes(s.vm.app),
        render::bytes(s.vm.wired),
        render::bytes(s.vm.compressed),
        render::bytes(s.vm.cached),
        render::bytes(s.vm.free),
        s.pressure.map(|p| p.label())
    );
    for t in &s.top {
        println!(
            "  top {:<24} {:>6.1}%  {}",
            render::short_name(&t.name, 24),
            t.cpu,
            render::bytes(t.mem)
        );
    }
    println!(
        "swap {} / {}",
        render::bytes(s.swap_used),
        render::bytes(s.swap_total)
    );
    for d in &s.disks {
        println!(
            "disk {:<20} {} / {} ({:.0}%)",
            d.name,
            render::bytes(d.used()),
            render::bytes(d.total),
            d.used_pct()
        );
    }
    println!(
        "net ↓ {} ↑ {}  total ↓ {} ↑ {}",
        render::rate(s.net_rx),
        render::rate(s.net_tx),
        render::bytes(s.net_total_rx),
        render::bytes(s.net_total_tx)
    );
    for n in &s.nets {
        println!(
            "  if {:<10} ↓ {:>10} ↑ {:>10}",
            n.name,
            render::rate(n.rx),
            render::rate(n.tx)
        );
    }
    println!("uptime {}", render::uptime(s.uptime_secs));
}

/// Menu bar text next to the icon, one span per enabled part.
fn title_spans(s: &Snapshot, cfg: &Config) -> Vec<(String, Tone)> {
    let mut v: Vec<(String, Tone)> = Vec::new();
    let gap = |v: &mut Vec<(String, Tone)>| {
        if !v.is_empty() {
            v.push(("  ".into(), Tone::Faint));
        }
    };
    if cfg.cpu {
        v.push((format!("{:.0}%", s.cpu_total), theme::pct_tone(s.cpu_total)));
    }
    if cfg.mem {
        gap(&mut v);
        v.push((
            render::bytes_short(s.vm.used()),
            theme::pct_tone(s.mem_used_pct() as f32),
        ));
    }
    if cfg.disk
        && let Some(d) = s.disks.first()
    {
        gap(&mut v);
        let pct = d.used_pct() as f32;
        v.push(("SSD ".into(), Tone::Dim));
        v.push((format!("{pct:.0}%"), theme::pct_tone(pct)));
    }
    if cfg.net {
        gap(&mut v);
        v.push(("↓".into(), Tone::Dim));
        // Fixed-width MB fields keep the status item from jittering in width.
        v.push((format!("{}M", render::mb(s.net_rx)), Tone::Bright));
        v.push((" ↑".into(), Tone::Dim));
        v.push((format!("{}M", render::mb(s.net_tx)), Tone::Bright));
    }
    v
}

/// Plain-text title for `--once`.
fn title(s: &Snapshot, cfg: &Config) -> String {
    title_spans(s, cfg)
        .iter()
        .map(|(t, _)| t.as_str())
        .collect()
}

/// Push icon and styled title onto the status item according to the toggles.
fn set_status(t: &TrayIcon, s: &Snapshot, cfg: &Config) {
    let cfg = cfg.effective();
    let icon = cfg
        .icon
        .then(|| render::core_bars(&s.cores, theme::ICON_RGB));
    let _ = t.set_icon(icon);
    if let Some(item) = t.ns_status_item() {
        let spans = title_spans(s, &cfg);
        let mut refs: Vec<theme::Span> = Vec::new();
        if cfg.icon && !spans.is_empty() {
            refs.push((" ", Tone::Faint));
        }
        refs.extend(spans.iter().map(|(t, tone)| (t.as_str(), *tone)));
        theme::style_status(&item, &refs);
    }
}

fn main() {
    if std::env::args().any(|a| a == "--once") {
        print_once();
        return;
    }
    let mut event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    event_loop.set_activation_policy(ActivationPolicy::Accessory);

    let proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |e| {
        let _ = proxy.send_event(UserEvent::Menu(e));
    }));

    let mut sampler = Sampler::new();
    let mut cfg = Config::load();
    // sysinfo needs two CPU refreshes spaced apart before usage is meaningful.
    std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);

    let mut tray: Option<TrayIcon> = None;
    let mut stats_menu: Option<StatsMenu> = None;
    let mut next = Instant::now();
    let mut last: Option<Snapshot> = None;
    // MATRIXSTAT_DEMO=1 pops the dropdown open on the 3rd tick, for screenshots.
    let demo = std::env::var_os("MATRIXSTAT_DEMO").is_some();
    let mut ticks = 0u32;

    event_loop.run(move |event, _, control_flow| {
        match event {
            // macOS requires the tray to be created once the loop is live.
            Event::NewEvents(StartCause::Init) => {
                let snap = sampler.sample();
                let m = StatsMenu::build(&snap, &cfg);
                let t = TrayIconBuilder::new()
                    .with_menu(Box::new(m.menu.clone()))
                    .with_icon(render::core_bars(&snap.cores, theme::ICON_RGB))
                    .with_tooltip("matrixstat")
                    .build()
                    .expect("create tray icon");
                set_status(&t, &snap, &cfg);
                last = Some(snap);
                tray = Some(t);
                stats_menu = Some(m);
                next = Instant::now() + INTERVAL;
            }
            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let snap = sampler.sample();
                if let (Some(t), Some(m)) = (tray.as_ref(), stats_menu.as_mut()) {
                    m.update(&snap);
                    set_status(t, &snap, &cfg);
                    ticks += 1;
                    if demo && ticks == 3 {
                        t.show_menu();
                    }
                }
                last = Some(snap);
                next = Instant::now() + INTERVAL;
            }
            Event::UserEvent(UserEvent::Menu(e))
                if stats_menu
                    .as_ref()
                    .is_some_and(|m| m.handle_toggle(&e.id, &mut cfg)) =>
            {
                cfg.save();
                if let (Some(t), Some(s)) = (tray.as_ref(), last.as_ref()) {
                    set_status(t, s, &cfg);
                }
            }
            Event::UserEvent(UserEvent::Menu(e))
                if stats_menu.as_ref().is_some_and(|m| e.id == m.quit_id) =>
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
            _ => {}
        }
        *control_flow = ControlFlow::WaitUntil(next);
    });
}
