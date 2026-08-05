//! The colors the picker paints, and how they are derived from a theme's anchors.

use ratatui::style::Color;

/// Every color the picker paints. Slots are named for the job they do, not the hue they
/// happen to be — quotr keeps four accents, each with exactly one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Fill behind the live selection.
    pub select_bg: Color,
    /// Markdown markers, the bank gutter, card borders, the status line.
    pub overlay0: Color,
    /// What [`Palette::on_fill`] lifts `overlay0` to, and where deep headings land.
    pub subtext0: Color,
    pub text: Color,
    /// Inline code and list bullets.
    pub code: Color,
    /// Bold text.
    pub strong: Color,
    /// Top-level headings.
    pub heading: Color,
    /// Third-level headings — still a heading, half a step quieter.
    pub heading_deep: Color,
    /// Link text.
    pub link: Color,
}

impl Palette {
    /// Lift a color onto the selection fill. `overlay0` sits one surface step above the fill
    /// and all but vanishes on it, so it rises to `subtext0`; everything else passes through.
    pub fn on_fill(&self, color: Color) -> Color {
        if color == self.overlay0 { self.subtext0 } else { color }
    }
}

/// A theme's intrinsic cast, which sets the derivation direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Appearance {
    Dark,
    Light,
}

/// The anchor colors a theme lists; the rest of its palette is computed from these.
#[derive(Clone, Copy, Debug)]
pub(super) struct Anchors {
    pub(super) base: Color,
    pub(super) text: Color,
    pub(super) code: Color,
    pub(super) strong: Color,
    pub(super) heading: Color,
    pub(super) link: Color,
}

/// Build `Anchors` from `0xRRGGBB` literals, so a palette reads as one compact row.
pub(super) const fn anchors(
    base: u32,
    text: u32,
    code: u32,
    strong: u32,
    heading: u32,
    link: u32,
) -> Anchors {
    Anchors {
        base: hex(base),
        text: hex(text),
        code: hex(code),
        strong: hex(strong),
        heading: hex(heading),
        link: hex(link),
    }
}

/// A `Color::Rgb` from a `0xRRGGBB` literal.
const fn hex(rgb: u32) -> Color {
    let [_, r, g, b] = rgb.to_be_bytes();
    Color::Rgb(r, g, b)
}

/// How far the selection fill steps `base` toward the contrast pole. A light theme takes a
/// much smaller step: its text is dark, so a fill moving toward black closes the gap the text
/// needs. These are the largest steps that keep every shipped theme legible.
const DARK_FILL: f64 = 0.13;
const LIGHT_FILL: f64 = 0.07;

const WHITE: Color = Color::Rgb(0xff, 0xff, 0xff);
const BLACK: Color = Color::Rgb(0x00, 0x00, 0x00);

/// Build a full palette from anchors: surfaces step `base` toward the contrast pole.
pub(super) fn derive(a: Anchors, appearance: Appearance) -> Palette {
    let (pole, fill) = match appearance {
        Appearance::Dark => (WHITE, DARK_FILL),
        Appearance::Light => (BLACK, LIGHT_FILL),
    };
    let subtext0 = blend(a.text, a.base, 0.18);

    Palette {
        select_bg: blend(a.base, pole, fill),
        overlay0: blend(a.base, pole, 0.26),
        subtext0,
        text: a.text,
        code: a.code,
        strong: a.strong,
        heading: a.heading,
        heading_deep: blend(a.heading, subtext0, 0.5),
        link: a.link,
    }
}

/// Linear per-channel blend: `t` of the way from `from` to `to`.
// Both ends are `u8` and `t` is in 0..=1, so a blended channel is always back in range.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn blend(from: Color, to: Color, t: f64) -> Color {
    let (fr, fg, fb) = channels(from);
    let (tr, tg, tb) = channels(to);
    let mix = |lhs: u8, rhs: u8| {
        (f64::from(lhs) * (1.0 - t) + f64::from(rhs) * t).round().clamp(0.0, 255.0) as u8
    };

    Color::Rgb(mix(fr, tr), mix(fg, tg), mix(fb, tb))
}

/// The RGB channels of a color; anchors are always `Rgb`, so the fallback never fires.
fn channels(color: Color) -> (u8, u8, u8) {
    match color {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

/// The WCAG contrast ratio between two colors (1.0 .. 21.0). Anchors are hand-picked rather
/// than tuned against a floor, so this only guards them in tests.
#[cfg(test)]
pub(super) fn contrast(fg: Color, bg: Color) -> f64 {
    let (lf, lb) = (luminance(fg), luminance(bg));
    let (hi, lo) = if lf >= lb { (lf, lb) } else { (lb, lf) };

    (hi + 0.05) / (lo + 0.05)
}

/// WCAG relative luminance, with sRGB linearization.
#[cfg(test)]
pub(super) fn luminance(color: Color) -> f64 {
    let (r, g, b) = channels(color);
    let lin = |channel: u8| {
        let srgb = f64::from(channel) / 255.0;
        if srgb <= 0.03928 { srgb / 12.92 } else { ((srgb + 0.055) / 1.055).powf(2.4) }
    };

    0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)
}

#[cfg(test)]
mod tests {
    use super::{BLACK, WHITE, contrast};

    #[test]
    fn contrast_black_white_is_max() {
        let r = contrast(BLACK, WHITE);
        assert!((r - 21.0).abs() < 0.01, "black vs white is ~21:1, got {r}");
    }
}
