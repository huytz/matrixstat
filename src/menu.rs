//! Dropdown laid out like iStat: section header with headline value, graph rows, aligned columns.

use tray_icon::menu::{
    CheckMenuItem, ContextMenu, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu,
    accelerator::{Accelerator, Code, Modifiers},
};

use crate::config::{Config, KEYS};

use crate::render;
use crate::stats::{HIST, Snapshot, TOP_N};
use crate::theme::{self, GraphRow, Span, Tone};
use crate::weather::Weather;

/// Characters per text row; every row is padded to this so right columns line up.
const COLS: usize = 40;
/// Menlo 12pt advance ≈ 7.22pt; graphs match the text width plus the menu insets.
const GRAPH_W: u32 = (COLS as f64 * 7.22) as u32 + 2 * render::INSET_PT;
const HIST_H: u32 = 44;
const CORES_H: u32 = 26;
/// Width in cells of the text bars used for per-core and disk gauges.
const BAR_CELLS: usize = 10;

/// Placeholder titles that `GraphRow::attach` swaps for image views.
const PH_CPU: &str = "graph:cpu";
const PH_CORES: &str = "graph:cores";
const PH_MEM: &str = "graph:mem";
const PH_NET: &str = "graph:net";

/// Series colours for the stacked memory graph: app, wired, compressed.
const MEM_SERIES: [Tone; 3] = [Tone::Bright, Tone::Mid, Tone::Dim];
/// Series colours for the network graph: download, upload.
const NET_SERIES: [Tone; 2] = [Tone::Bright, Tone::Dim];

type Spans = Vec<(String, Tone)>;

pub struct StatsMenu {
    pub menu: Menu,
    pub quit_id: MenuId,
    cpu_hdr: MenuItem,
    cpu_graph: Option<GraphRow>,
    cores_graph: Option<GraphRow>,
    load: MenuItem,
    cores_menu: Submenu,
    cores: Vec<MenuItem>,
    top: Vec<MenuItem>,
    mem_hdr: MenuItem,
    mem_graph: Option<GraphRow>,
    mem_rows: [MenuItem; 5],
    swap: MenuItem,
    pressure: MenuItem,
    disk_pos: usize,
    disks: Vec<MenuItem>,
    disk_names: Vec<String>,
    net_hdr: MenuItem,
    net_graph: Option<GraphRow>,
    net_base: usize,
    nets: Vec<MenuItem>,
    net_names: Vec<String>,
    net_total: MenuItem,
    wx_hdr: MenuItem,
    wx_rows: [MenuItem; 5],
    uptime: MenuItem,
    toggles: Vec<(MenuId, &'static str, CheckMenuItem)>,
}

/// Labels for the menu-bar toggles, same order as config::KEYS.
const TOGGLE_LABELS: [&str; 6] = [
    "Core bars",
    "CPU %",
    "Memory used",
    "Disk used",
    "Network ↓↑",
    "Weather",
];

/// Info rows are disabled so they neither highlight on hover nor react to clicks.
fn row() -> MenuItem {
    MenuItem::new("", false, None)
}

fn apply(item: &MenuItem, spans: &Spans) {
    let refs: Vec<Span> = spans.iter().map(|(s, t)| (s.as_str(), *t)).collect();
    theme::style_row(item, &refs);
}

/// Pad between left and right spans so the row is exactly COLS wide.
fn lr(left: Spans, right: Spans) -> Spans {
    let used: usize = left
        .iter()
        .chain(&right)
        .map(|(s, _)| {
            // An SF Symbol attachment is about two Menlo cells wide.
            if s.starts_with(theme::SYMBOL) {
                2
            } else {
                s.chars().count()
            }
        })
        .sum();
    let pad = COLS.saturating_sub(used).max(1);
    let mut out = left;
    out.push((" ".repeat(pad), Tone::Faint));
    out.extend(right);
    out
}

/// "↓ 1.2 MB/s  ↑ 30.0 KB/s" with arrows in the graph series colours.
fn updown(rx: &str, tx: &str) -> Spans {
    vec![
        span("↓ ", NET_SERIES[0]),
        span(format!("{rx:>9}"), Tone::Bright),
        span("  ↑ ", NET_SERIES[1]),
        span(format!("{tx:>9}"), Tone::Bright),
    ]
}

fn span(s: impl Into<String>, t: Tone) -> (String, Tone) {
    (s.into(), t)
}

/// Section header: title in mid green, headline value bright at the right edge.
fn header(item: &MenuItem, title: &str, value: &str, tone: Tone) {
    apply(
        item,
        &lr(vec![span(title, Tone::Mid)], vec![span(value, tone)]),
    );
}

/// "label ........ value" row.
fn kv(item: &MenuItem, label: &str, value: &str, tone: Tone) {
    apply(
        item,
        &lr(vec![span(label, Tone::Dim)], vec![span(value, tone)]),
    );
}

/// Bar split into bright filled cells and faint empty cells.
fn bar_spans(pct: f32) -> Spans {
    let full = render::bar(pct, BAR_CELLS);
    let filled: String = full.chars().filter(|c| *c == '█').collect();
    let empty: String = full.chars().filter(|c| *c == '░').collect();
    vec![span(filled, theme::pct_tone(pct)), span(empty, Tone::Faint)]
}

/// "label  ████░░░░  NN%" row with the gauge hugging the right edge.
fn gauge(item: &MenuItem, label: &str, pct: f32) {
    let mut right = bar_spans(pct);
    right.push(span(format!(" {pct:>3.0}%"), theme::pct_tone(pct)));
    apply(item, &lr(vec![span(label, Tone::Dim)], right));
}

impl StatsMenu {
    pub fn build(snap: &Snapshot, cfg: &Config) -> Self {
        let menu = Menu::new();
        let cpu_hdr = row();
        let ph_cpu = MenuItem::new(PH_CPU, false, None);
        let ph_cores = MenuItem::new(PH_CORES, false, None);
        let load = row();
        let cores: Vec<MenuItem> = snap.cores.iter().map(|_| row()).collect();
        let cores_menu = Submenu::new("Per core", true);
        for c in &cores {
            cores_menu.append(c).expect("core row");
        }
        theme::style_submenu(&cores_menu, &[("Per core", Tone::Dim)]);
        let top: Vec<MenuItem> = (0..TOP_N).map(|_| row()).collect();
        let mem_hdr = row();
        let ph_mem = MenuItem::new(PH_MEM, false, None);
        let mem_rows = [row(), row(), row(), row(), row()];
        let swap = row();
        let pressure = row();
        let net_hdr = row();
        let ph_net = MenuItem::new(PH_NET, false, None);
        let net_total = row();
        let wx_hdr = row();
        let wx_rows = [row(), row(), row(), row(), row()];
        let uptime = row();
        let quit = MenuItem::new(
            "Exit the Matrix",
            true,
            Some(Accelerator::new(Modifiers::META, Code::KeyQ)),
        );
        theme::style_row(&quit, &[("Exit the Matrix", Tone::Mid)]);
        let bar_menu = Submenu::new("Menu bar", true);
        theme::style_submenu(&bar_menu, &[("Show in menu bar", Tone::Dim)]);
        let mut toggles = Vec::new();
        for (key, label) in KEYS.iter().zip(TOGGLE_LABELS) {
            let item = CheckMenuItem::new(label, true, cfg.get(key), None);
            theme::style_check(&item, &[(label, Tone::Dim)]);
            bar_menu.append(&item).expect("toggle row");
            toggles.push((item.id().clone(), *key, item));
        }
        let top_hdr = row();
        header(&top_hdr, "PROCESSES", "CPU     MEM", Tone::Dim);
        let disk_hdr = row();
        header(&disk_hdr, "DISKS", "", Tone::Dim);
        let seps: Vec<PredefinedMenuItem> =
            (0..7).map(|_| PredefinedMenuItem::separator()).collect();

        let mut items: Vec<&dyn IsMenuItem> =
            vec![&cpu_hdr, &ph_cpu, &ph_cores, &load, &cores_menu, &seps[0]];
        items.push(&top_hdr);
        for t in &top {
            items.push(t);
        }
        items.push(&seps[1]);
        items.push(&mem_hdr);
        items.push(&ph_mem);
        for r in &mem_rows {
            items.push(r);
        }
        items.push(&swap);
        items.push(&pressure);
        items.push(&seps[2]);
        items.push(&disk_hdr);
        let disk_pos = items.len();
        items.push(&seps[3]);
        items.push(&net_hdr);
        items.push(&ph_net);
        let net_base = items.len();
        items.push(&net_total);
        items.push(&seps[4]);
        items.push(&wx_hdr);
        for r in &wx_rows {
            items.push(r);
        }
        items.push(&seps[5]);
        items.push(&uptime);
        items.push(&bar_menu);
        items.push(&seps[6]);
        items.push(&quit);
        menu.append_items(&items).expect("build menu");

        let ns = menu.ns_menu();
        theme::force_dark(ns);
        let cpu_graph = GraphRow::attach(ns, PH_CPU, GRAPH_W, HIST_H);
        let cores_graph = GraphRow::attach(ns, PH_CORES, GRAPH_W, CORES_H);
        let mem_graph = GraphRow::attach(ns, PH_MEM, GRAPH_W, HIST_H);
        let net_graph = GraphRow::attach(ns, PH_NET, GRAPH_W, HIST_H);

        let mut m = Self {
            menu,
            quit_id: quit.id().clone(),
            cpu_hdr,
            cpu_graph,
            cores_graph,
            load,
            cores_menu,
            cores,
            top,
            mem_hdr,
            mem_graph,
            mem_rows,
            swap,
            pressure,
            disk_pos,
            disks: Vec::new(),
            disk_names: Vec::new(),
            net_hdr,
            net_graph,
            net_base,
            nets: Vec::new(),
            net_names: Vec::new(),
            net_total,
            wx_hdr,
            wx_rows,
            uptime,
            toggles,
        };
        m.update(snap);
        m.update_weather(None);
        m
    }

    /// WEATHER section; rows show a placeholder until the first fetch lands.
    pub fn update_weather(&self, w: Option<&Weather>) {
        let Some(w) = w else {
            header(&self.wx_hdr, "WEATHER", "locating…", Tone::Faint);
            for r in &self.wx_rows {
                apply(r, &vec![span("·", Tone::Faint)]);
            }
            return;
        };
        apply(
            &self.wx_hdr,
            &lr(
                vec![span("WEATHER", Tone::Mid)],
                vec![
                    span(w.symbol(), Tone::Bright),
                    span(format!(" {:.0}", w.temp), Tone::Bright),
                    span(Weather::CELSIUS, Tone::Bright),
                ],
            ),
        );
        kv(
            &self.wx_rows[0],
            &render::short_name(&w.place, 24),
            w.label(),
            Tone::Bright,
        );
        kv(
            &self.wx_rows[1],
            "Feels like",
            &format!("{:.0}°C", w.feels),
            Tone::Bright,
        );
        kv(
            &self.wx_rows[2],
            "High / Low",
            &format!("{:.0}° / {:.0}°", w.hi, w.lo),
            Tone::Bright,
        );
        kv(
            &self.wx_rows[3],
            "Humidity  Wind",
            &format!("{:.0}%  {:.0} km/h", w.humidity, w.wind_kmh),
            Tone::Bright,
        );
        let mins = w.at.elapsed().as_secs() / 60;
        kv(
            &self.wx_rows[4],
            "Updated",
            &format!("{mins} min ago · Open-Meteo"),
            Tone::Faint,
        );
    }

    /// If `id` is a menu-bar toggle, fold its new state into `cfg` and report true.
    pub fn handle_toggle(&self, id: &MenuId, cfg: &mut Config) -> bool {
        match self.toggles.iter().find(|(tid, _, _)| tid == id) {
            Some((_, key, item)) => {
                cfg.set(key, item.is_checked());
                true
            }
            None => false,
        }
    }

    pub fn update(&mut self, s: &Snapshot) {
        // CPU
        header(
            &self.cpu_hdr,
            "CPU",
            &format!("{:.0}%", s.cpu_total),
            theme::pct_tone(s.cpu_total),
        );
        if let Some(g) = &self.cpu_graph {
            g.set(&render::graph_history(
                &s.cpu_hist,
                HIST,
                GRAPH_W,
                HIST_H,
                theme::rgb8(Tone::Bright),
            ));
        }
        if let Some(g) = &self.cores_graph {
            g.set(&render::graph_cores(
                &s.cores,
                GRAPH_W,
                CORES_H,
                theme::rgb8(Tone::Mid),
            ));
        }
        kv(
            &self.load,
            "Load",
            &format!("{:.2}  {:.2}  {:.2}", s.load[0], s.load[1], s.load[2]),
            Tone::Bright,
        );
        let hot = s.cores.iter().filter(|c| **c >= 90.0).count();
        let label = if hot > 0 {
            format!("Per core  ({hot} hot)")
        } else {
            "Per core".to_string()
        };
        theme::style_submenu(&self.cores_menu, &[(&label, Tone::Dim)]);
        for (i, (item, pct)) in self.cores.iter().zip(&s.cores).enumerate() {
            gauge(item, &format!("Core {:02}", i + 1), *pct);
        }

        // Processes
        for (i, item) in self.top.iter().enumerate() {
            match s.top.get(i) {
                Some(t) => apply(
                    item,
                    &lr(
                        vec![span(render::short_name(&t.name, 22), Tone::Dim)],
                        vec![
                            span(format!("{:>5.1}%", t.cpu), theme::pct_tone(t.cpu)),
                            span(format!(" {:>7}", render::bytes(t.mem)), Tone::Dim),
                        ],
                    ),
                ),
                None => apply(item, &vec![span("·", Tone::Faint)]),
            }
        }

        // Memory
        let pct = s.mem_used_pct() as f32;
        header(
            &self.mem_hdr,
            "MEMORY",
            &format!(
                "{} / {}   {pct:.0}%",
                render::bytes(s.vm.used()),
                render::bytes(s.mem_total)
            ),
            theme::pct_tone(pct),
        );
        if let Some(g) = &self.mem_graph {
            g.set(&render::graph_stacked(
                &s.mem_hist,
                HIST,
                GRAPH_W,
                HIST_H,
                [
                    theme::rgb8(MEM_SERIES[0]),
                    theme::rgb8(MEM_SERIES[1]),
                    theme::rgb8(MEM_SERIES[2]),
                ],
            ));
        }
        let legend: [(&str, u64, Tone); 5] = [
            ("App", s.vm.app, MEM_SERIES[0]),
            ("Wired", s.vm.wired, MEM_SERIES[1]),
            ("Compressed", s.vm.compressed, MEM_SERIES[2]),
            ("Cached files", s.vm.cached, Tone::Faint),
            ("Free", s.vm.free, Tone::Faint),
        ];
        for (item, (name, val, dot)) in self.mem_rows.iter().zip(legend) {
            apply(
                item,
                &lr(
                    vec![span("● ", dot), span(name, Tone::Dim)],
                    vec![span(render::bytes(val), Tone::Bright)],
                ),
            );
        }
        kv(
            &self.swap,
            "Swap",
            &format!(
                "{} / {}",
                render::bytes(s.swap_used),
                render::bytes(s.swap_total)
            ),
            Tone::Bright,
        );
        let (ptxt, ptone) = match s.pressure {
            Some(crate::mac::Pressure::Normal) => ("Normal", Tone::Bright),
            Some(crate::mac::Pressure::Warning) => ("Warning", Tone::Alert),
            Some(crate::mac::Pressure::Critical) => ("Critical", Tone::Alert),
            _ => ("Unknown", Tone::Faint),
        };
        kv(&self.pressure, "Pressure", ptxt, ptone);

        // Disks + system
        self.sync_disks(s);

        // Network
        apply(
            &self.net_hdr,
            &lr(
                vec![span("NETWORK", Tone::Mid)],
                updown(&render::rate(s.net_rx), &render::rate(s.net_tx)),
            ),
        );
        if let Some(g) = &self.net_graph {
            g.set(&render::graph_updown(
                &s.net_hist,
                HIST,
                GRAPH_W,
                HIST_H,
                [theme::rgb8(NET_SERIES[0]), theme::rgb8(NET_SERIES[1])],
            ));
        }
        self.sync_nets(s);
        apply(
            &self.net_total,
            &lr(
                vec![span("Since boot", Tone::Faint)],
                updown(
                    &render::bytes(s.net_total_rx),
                    &render::bytes(s.net_total_tx),
                ),
            ),
        );

        kv(
            &self.uptime,
            "Uptime",
            &render::uptime(s.uptime_secs),
            Tone::Bright,
        );
    }

    /// One row per interface; rebuilt only when the set of interfaces changes.
    fn sync_nets(&mut self, s: &Snapshot) {
        let mut names: Vec<String> = s.nets.iter().map(|n| n.name.clone()).collect();
        names.sort();
        if names != self.net_names {
            for item in &self.nets {
                let _ = self.menu.remove(item);
            }
            self.nets = s.nets.iter().map(|_| row()).collect();
            let refs: Vec<&dyn IsMenuItem> =
                self.nets.iter().map(|i| i as &dyn IsMenuItem).collect();
            // Disk rows sit above, so the slot shifts by however many there are now.
            let _ = self
                .menu
                .insert_items(&refs, self.net_base + self.disks.len());
            self.net_names = names;
        }
        for (item, n) in self.nets.iter().zip(&s.nets) {
            apply(
                item,
                &lr(
                    vec![span(render::short_name(&n.name, 12), Tone::Dim)],
                    updown(&render::rate(n.rx), &render::rate(n.tx)),
                ),
            );
        }
    }

    /// Two rows per disk; rebuilt only when the set of mounts changes.
    fn sync_disks(&mut self, s: &Snapshot) {
        let names: Vec<String> = s.disks.iter().map(|d| d.name.clone()).collect();
        if names != self.disk_names {
            for item in &self.disks {
                let _ = self.menu.remove(item);
            }
            self.disks = s.disks.iter().flat_map(|_| [row(), row()]).collect();
            let refs: Vec<&dyn IsMenuItem> =
                self.disks.iter().map(|i| i as &dyn IsMenuItem).collect();
            let _ = self.menu.insert_items(&refs, self.disk_pos);
            self.disk_names = names;
        }
        for (pair, d) in self.disks.chunks(2).zip(&s.disks) {
            let pct = d.used_pct() as f32;
            apply(
                &pair[0],
                &lr(
                    vec![span(render::short_name(&d.name, 20), Tone::Dim)],
                    vec![span(
                        format!("{} available", render::bytes(d.available)),
                        Tone::Bright,
                    )],
                ),
            );
            let mut right = bar_spans(pct);
            right.push(span(format!(" {pct:>3.0}%"), theme::pct_tone(pct)));
            apply(
                &pair[1],
                &lr(
                    vec![span(
                        format!(
                            "{} of {} used",
                            render::bytes(d.used()),
                            render::bytes(d.total)
                        ),
                        Tone::Faint,
                    )],
                    right,
                ),
            );
        }
    }
}
