//! Text and icon rendering helpers for the menu bar.

/// Human-readable bytes with one decimal, binary units like Activity Monitor.
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n} B")
    } else if v >= 100.0 {
        format!("{v:.0} {}", UNITS[i])
    } else {
        format!("{v:.1} {}", UNITS[i])
    }
}

/// Compact bytes for the menu bar: "12.9G", "512M".
pub fn bytes_short(n: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 {
        format!("{n}B")
    } else if v >= 10.0 {
        format!("{v:.0}{}", UNITS[i])
    } else {
        format!("{v:.1}{}", UNITS[i])
    }
}

/// Bytes/s as a fixed-MB number, always 4 chars: "0.01", "12.3", " 123".
pub fn mb(n: u64) -> String {
    let v = n as f64 / (1024.0 * 1024.0);
    if v < 10.0 {
        format!("{v:.2}")
    } else if v < 100.0 {
        format!("{v:.1}")
    } else {
        format!("{v:>4.0}")
    }
}

/// Throughput always in MB/s like "0.01 MB/s" so the unit never flips.
pub fn rate(n: u64) -> String {
    format!("{} MB/s", mb(n))
}

/// Text bar of `cells` block characters filled to `pct` percent.
pub fn bar(pct: f32, cells: usize) -> String {
    let filled = ((pct.clamp(0.0, 100.0) / 100.0) * cells as f32).round() as usize;
    let mut s = String::with_capacity(cells * 3);
    for i in 0..cells {
        s.push(if i < filled { '█' } else { '░' });
    }
    s
}

/// Uptime as "3d 4h 12m" (or "4h 12m" under a day).
pub fn uptime(secs: u64) -> String {
    let d = secs / 86_400;
    let h = (secs % 86_400) / 3_600;
    let m = (secs % 3_600) / 60;
    if d > 0 {
        format!("{d}d {h}h {m}m")
    } else {
        format!("{h}h {m}m")
    }
}

/// Cut long process names so dropdown rows stay one width.
pub fn short_name(name: &str, max: usize) -> String {
    if name.chars().count() <= max {
        return name.to_string();
    }
    let cut: String = name.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
}

/// Bars are 32px (16pt at 2x) tall so they sit like iStat's graphs.
const BAR_H: u32 = 32;
const BAR_W: u32 = 3;
const GAP: u32 = 1;
/// Faint track alpha behind each bar; fill is fully opaque.
const TRACK_ALPHA: u8 = 64;

/// Per-core vertical bars in one solid colour on a transparent 2x canvas.
pub fn core_bars(cores: &[f32], rgb: [u8; 3]) -> Canvas {
    let n = cores.len().max(1) as u32;
    let w = n * BAR_W + (n - 1) * GAP;
    let mut px = vec![0u8; (w * BAR_H * 4) as usize];
    let mut put = |x: u32, y: u32, a: u8| {
        let i = ((y * w + x) * 4) as usize;
        px[i..i + 3].copy_from_slice(&rgb);
        px[i + 3] = a;
    };
    for (ci, pct) in cores.iter().enumerate() {
        let x0 = ci as u32 * (BAR_W + GAP);
        let fill = ((pct.clamp(0.0, 100.0) / 100.0) * BAR_H as f32).round() as u32;
        for y in 0..BAR_H {
            let from_bottom = BAR_H - y;
            let a = if from_bottom <= fill {
                255
            } else {
                TRACK_ALPHA
            };
            for x in x0..x0 + BAR_W {
                put(x, y, a);
            }
        }
    }
    Canvas { w, h: BAR_H, px }
}

/// 8-bit RGBA canvas at 2x scale; everything in it is drawn in pixels, not points.
pub struct Canvas {
    pub w: u32,
    pub h: u32,
    pub px: Vec<u8>,
}

/// Graph images are 2x so they stay crisp on Retina; sizes below are in points.
pub const SCALE: u32 = 2;

impl Canvas {
    pub fn new(w_pt: u32, h_pt: u32) -> Self {
        let (w, h) = (w_pt * SCALE, h_pt * SCALE);
        Self {
            w,
            h,
            px: vec![0; (w * h * 4) as usize],
        }
    }

    /// Fill a rectangle; coordinates are pixels, y grows downward, clipped to the canvas.
    pub fn rect(&mut self, x: i32, y: i32, w: u32, h: u32, rgb: [u8; 3], a: u8) {
        for yy in y.max(0)..(y + h as i32).min(self.h as i32) {
            for xx in x.max(0)..(x + w as i32).min(self.w as i32) {
                let i = ((yy as u32 * self.w + xx as u32) * 4) as usize;
                self.px[i..i + 3].copy_from_slice(&rgb);
                self.px[i + 3] = a;
            }
        }
    }

    pub fn to_png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, self.w, self.h);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().expect("png header");
            w.write_image_data(&self.px).expect("png data");
            w.finish().expect("png finish");
        }
        out
    }
}

/// Horizontal inset of graph content so it lines up with menu text.
pub const INSET_PT: u32 = 14;

/// Faint baseline alpha under every graph.
const BASE_ALPHA: u8 = 70;

/// iStat-style history bars: newest sample at the right, one bar per slot.
pub fn graph_history(hist: &[f32], slots: usize, w_pt: u32, h_pt: u32, rgb: [u8; 3]) -> Canvas {
    let mut c = Canvas::new(w_pt, h_pt);
    let inset = INSET_PT * SCALE;
    let plot_w = c.w - 2 * inset;
    let slot_w = plot_w / slots as u32;
    let gap = SCALE;
    let base_y = c.h as i32 - 2 * SCALE as i32;
    c.rect(inset as i32, base_y, plot_w, SCALE, rgb, BASE_ALPHA);
    let start = slots.saturating_sub(hist.len());
    for (i, v) in hist.iter().enumerate() {
        let x = inset + (start + i) as u32 * slot_w;
        let bh = ((v.clamp(0.0, 100.0) / 100.0) * (base_y as f32)).round() as u32;
        c.rect(x as i32, base_y - bh as i32, slot_w - gap, bh, rgb, 255);
    }
    c
}

/// Stacked memory history: app, wired, compressed from the bottom up.
pub fn graph_stacked(
    hist: &[[f32; 3]],
    slots: usize,
    w_pt: u32,
    h_pt: u32,
    rgbs: [[u8; 3]; 3],
) -> Canvas {
    let mut c = Canvas::new(w_pt, h_pt);
    let inset = INSET_PT * SCALE;
    let plot_w = c.w - 2 * inset;
    let slot_w = plot_w / slots as u32;
    let gap = SCALE;
    let base_y = c.h as i32 - 2 * SCALE as i32;
    c.rect(inset as i32, base_y, plot_w, SCALE, rgbs[0], BASE_ALPHA);
    let start = slots.saturating_sub(hist.len());
    for (i, fr) in hist.iter().enumerate() {
        let x = (inset + (start + i) as u32 * slot_w) as i32;
        let mut top = base_y;
        for (k, f) in fr.iter().enumerate() {
            let bh = (f.clamp(0.0, 1.0) * base_y as f32).round() as i32;
            c.rect(x, top - bh, slot_w - gap, bh as u32, rgbs[k], 255);
            top -= bh;
        }
    }
    c
}

/// Graph never scales below 10 KB/s so idle noise stays flat.
const NET_FLOOR: f32 = 10.0 * 1024.0;

/// Mirrored throughput history: download grows up, upload down, from a shared midline.
pub fn graph_updown(
    hist: &[[f32; 2]],
    slots: usize,
    w_pt: u32,
    h_pt: u32,
    rgbs: [[u8; 3]; 2],
) -> Canvas {
    let mut c = Canvas::new(w_pt, h_pt);
    let inset = INSET_PT * SCALE;
    let plot_w = c.w - 2 * inset;
    let slot_w = plot_w / slots as u32;
    let gap = SCALE;
    let mid = (c.h / 2) as i32;
    let half = (mid - SCALE as i32) as f32;
    c.rect(inset as i32, mid, plot_w, 1, rgbs[0], BASE_ALPHA);
    let peak = hist
        .iter()
        .flat_map(|r| r.iter().copied())
        .fold(NET_FLOOR, f32::max);
    let start = slots.saturating_sub(hist.len());
    for (i, [down, up]) in hist.iter().enumerate() {
        let x = (inset + (start + i) as u32 * slot_w) as i32;
        let dh = ((down / peak).clamp(0.0, 1.0) * half).round() as i32;
        let uh = ((up / peak).clamp(0.0, 1.0) * half).round() as i32;
        c.rect(x, mid - dh, slot_w - gap, dh as u32, rgbs[0], 255);
        c.rect(x, mid + 1, slot_w - gap, uh as u32, rgbs[1], 255);
    }
    c
}

/// One wide bar per core with a faint full-height track behind it.
pub fn graph_cores(cores: &[f32], w_pt: u32, h_pt: u32, rgb: [u8; 3]) -> Canvas {
    let mut c = Canvas::new(w_pt, h_pt);
    let inset = INSET_PT * SCALE;
    let plot_w = c.w - 2 * inset;
    let n = cores.len().max(1) as u32;
    let gap = 3 * SCALE;
    let bar_w = (plot_w + gap) / n - gap;
    let plot_h = c.h - 2 * SCALE;
    for (i, v) in cores.iter().enumerate() {
        let x = (inset + i as u32 * (bar_w + gap)) as i32;
        c.rect(x, 0, bar_w, plot_h, rgb, TRACK_ALPHA);
        let bh = ((v.clamp(0.0, 100.0) / 100.0) * plot_h as f32).round() as u32;
        c.rect(x, (plot_h - bh) as i32, bar_w, bh, rgb, 255);
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_units() {
        assert_eq!(bytes(0), "0 B");
        assert_eq!(bytes(999), "999 B");
        assert_eq!(bytes(1536), "1.5 KB");
        assert_eq!(bytes(16 * 1024 * 1024 * 1024), "16.0 GB");
        assert_eq!(bytes(460 * 1024 * 1024 * 1024), "460 GB");
    }

    #[test]
    fn bytes_short_units() {
        assert_eq!(bytes_short(12_900_000_000), "12G");
        assert_eq!(bytes_short(9_800_000_000), "9.1G");
        assert_eq!(bytes_short(512 * 1024 * 1024), "512M");
    }

    #[test]
    fn rate_units() {
        assert_eq!(rate(0), "0.00 MB/s");
        assert_eq!(rate(1536 * 1024), "1.50 MB/s");
        assert_eq!(mb(30 * 1024 * 1024), "30.0");
        assert_eq!(mb(250 * 1024 * 1024), " 250");
    }

    #[test]
    fn updown_graph_size() {
        let g = graph_updown(&[[0.0, 0.0], [5e6, 1e5]], 60, 300, 40, [[0, 255, 65]; 2]);
        assert_eq!((g.w, g.h), (600, 80));
    }

    #[test]
    fn bar_fill() {
        assert_eq!(bar(0.0, 4), "░░░░");
        assert_eq!(bar(50.0, 4), "██░░");
        assert_eq!(bar(100.0, 4), "████");
        assert_eq!(bar(250.0, 4), "████");
    }

    #[test]
    fn uptime_fmt() {
        assert_eq!(uptime(59), "0h 0m");
        assert_eq!(uptime(3_600 + 120), "1h 2m");
        assert_eq!(uptime(2 * 86_400 + 3_600), "2d 1h 0m");
    }

    #[test]
    fn short_names() {
        assert_eq!(short_name("kernel_task", 20), "kernel_task");
        assert_eq!(short_name("abcdefghij", 5), "abcd…");
    }

    #[test]
    fn canvas_png_roundtrip_size() {
        let mut c = Canvas::new(10, 4);
        c.rect(-5, -5, 100, 100, [0, 255, 65], 255);
        assert_eq!(c.px[3], 255);
        let png = c.to_png();
        assert_eq!(&png[1..4], b"PNG");
        let g = graph_history(&[0.0, 50.0, 100.0], 60, 300, 40, [0, 255, 65]);
        assert_eq!((g.w, g.h), (600, 80));
    }

    #[test]
    fn icon_dimensions() {
        let n = 10usize;
        let w = n as u32 * BAR_W + (n as u32 - 1) * GAP;
        let c = core_bars(&vec![50.0; n], [0, 255, 65]);
        assert_eq!((c.w, c.h), (w, BAR_H));
        assert_eq!(w, 39);
    }
}
