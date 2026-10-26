//! Matrix theme: phosphor-green Menlo attributed strings for menu rows and status bar.

use std::ffi::c_void;

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{
    NSAppearance, NSAppearanceCustomization, NSAppearanceNameDarkAqua, NSColor, NSFont,
    NSFontAttributeName, NSForegroundColorAttributeName, NSImage, NSImageView, NSMenu,
    NSStatusItem,
};
use objc2_foundation::{
    NSAttributedString, NSAttributedStringKey, NSData, NSDictionary, NSMutableAttributedString,
    NSPoint, NSRect, NSSize, NSString,
};
use tray_icon::menu::{CheckMenuItem, MenuItem, Submenu};

use crate::render::Canvas;

/// Brightness tiers of the phosphor palette; Alert is the red pill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Bright,
    Mid,
    Dim,
    Faint,
    Alert,
}

/// 8-bit sRGB per tone: #00FF41, #00C837, #4F9E62, #1E4D2B, #FF4D4D.
pub fn rgb8(tone: Tone) -> [u8; 3] {
    match tone {
        Tone::Bright => [0x00, 0xFF, 0x41],
        Tone::Mid => [0x00, 0xC8, 0x37],
        Tone::Dim => [0x4F, 0x9E, 0x62],
        Tone::Faint => [0x1E, 0x4D, 0x2B],
        Tone::Alert => [0xFF, 0x4D, 0x4D],
    }
}

fn rgb(tone: Tone) -> (f64, f64, f64) {
    let [r, g, b] = rgb8(tone);
    (r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
}

/// Icon fill colour as 8-bit RGB, same green as Tone::Bright.
pub const ICON_RGB: [u8; 3] = [0x00, 0xFF, 0x41];

/// Menu rows use 12pt Menlo; status bar uses 11pt so it fits the bar height.
const MENU_PT: f64 = 12.0;
const BAR_PT: f64 = 11.0;

/// One styled run of text.
pub type Span<'a> = (&'a str, Tone);

fn font(pt: f64) -> Retained<NSFont> {
    NSFont::fontWithName_size(&NSString::from_str("Menlo"), pt)
        .unwrap_or_else(|| NSFont::systemFontOfSize(pt))
}

fn attributed(text: &str, tone: Tone, pt: f64) -> Retained<NSAttributedString> {
    let (r, g, b) = rgb(tone);
    let color = NSColor::colorWithSRGBRed_green_blue_alpha(r, g, b, 1.0);
    let font = font(pt);
    // SAFETY: AppKit attribute-name statics are plain constant NSStrings.
    let (k_font, k_color): (&NSAttributedStringKey, &NSAttributedStringKey) =
        unsafe { (NSFontAttributeName, NSForegroundColorAttributeName) };
    let f: &AnyObject = &font;
    let c: &AnyObject = &color;
    let attrs: Retained<NSDictionary<NSAttributedStringKey, AnyObject>> =
        NSDictionary::from_slices(&[k_font, k_color], &[f, c]);
    // SAFETY: attrs dictionary matches the declared key/value types.
    unsafe {
        NSAttributedString::initWithString_attributes(
            NSAttributedString::alloc(),
            &NSString::from_str(text),
            Some(&attrs),
        )
    }
}

fn concat(spans: &[Span], pt: f64) -> Retained<NSMutableAttributedString> {
    let out = NSMutableAttributedString::new();
    for (text, tone) in spans {
        out.appendAttributedString(&attributed(text, *tone, pt));
    }
    out
}

/// Replace a menu row's title with styled spans.
pub fn style_row(item: &MenuItem, spans: &[Span]) {
    item.set_attributed_title(Some(&concat(spans, MENU_PT)));
}

/// Check item label styled like a row.
pub fn style_check(item: &CheckMenuItem, spans: &[Span]) {
    item.set_attributed_title(Some(&concat(spans, MENU_PT)));
}

/// Submenu label styled like a row.
pub fn style_submenu(item: &Submenu, spans: &[Span]) {
    item.set_attributed_title(Some(&concat(spans, MENU_PT)));
}

/// An NSImageView mounted as a menu row; `set` swaps in a freshly rendered canvas.
pub struct GraphRow {
    view: Retained<NSImageView>,
    size: NSSize,
}

impl GraphRow {
    /// Find the placeholder row by its plain title and replace it with an image view.
    pub fn attach(ns_menu: *mut c_void, placeholder: &str, w_pt: u32, h_pt: u32) -> Option<Self> {
        if ns_menu.is_null() {
            return None;
        }
        let mtm = MainThreadMarker::new()?;
        // SAFETY: muda hands back a live NSMenu* owned by the Menu we keep alive.
        let menu: &NSMenu = unsafe { &*(ns_menu as *const NSMenu) };
        let item = menu.itemWithTitle(&NSString::from_str(placeholder))?;
        let size = NSSize::new(w_pt as f64, h_pt as f64);
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), size);
        let view = NSImageView::initWithFrame(mtm.alloc(), frame);
        item.setView(Some(&view));
        Some(Self { view, size })
    }

    pub fn set(&self, canvas: &Canvas) {
        let data = NSData::with_bytes(&canvas.to_png());
        if let Some(img) = NSImage::initWithData(NSImage::alloc(), &data) {
            img.setSize(self.size);
            self.view.setImage(Some(&img));
        }
    }
}

/// Styled text next to the icon in the menu bar.
pub fn style_status(item: &NSStatusItem, spans: &[Span]) {
    let mtm = MainThreadMarker::new().expect("status item is touched on the main thread");
    if let Some(button) = item.button(mtm) {
        button.setAttributedTitle(&concat(spans, BAR_PT));
    }
}

/// Force the dropdown into dark appearance so the green sits on near-black.
pub fn force_dark(ns_menu: *mut c_void) {
    if ns_menu.is_null() {
        return;
    }
    // SAFETY: muda hands back a live NSMenu* owned by the Menu we keep alive.
    let menu: &NSMenu = unsafe { &*(ns_menu as *const NSMenu) };
    // SAFETY: appearance-name static is a constant NSString.
    let name = unsafe { NSAppearanceNameDarkAqua };
    if let Some(dark) = NSAppearance::appearanceNamed(name) {
        menu.setAppearance(Some(&dark));
    }
}

/// Tone for a utilisation percentage: hot values go red.
pub fn pct_tone(pct: f32) -> Tone {
    if pct >= 90.0 {
        Tone::Alert
    } else {
        Tone::Bright
    }
}
