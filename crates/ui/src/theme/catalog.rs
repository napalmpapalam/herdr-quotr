//! The named themes: anchor rows, and the syntax theme each one pairs with.
//!
//! Names match herdr's, so a value copied from a herdr config resolves to the same palette
//! here. Anchor hues come from each theme's own canonical set.

// This file is a color table; 6-digit `0xRRGGBB` literals read better grouped as one value.
#![allow(clippy::unreadable_literal)]

use two_face::theme::EmbeddedThemeName as Embedded;

use crate::theme::{
    SyntaxChoice, Theme,
    palette::{Anchors, Appearance, anchors, derive},
};

/// Every theme quotr ships, in the order they are offered.
pub const NAMES: [&str; 19] = [
    "vs-dark-plus",
    "catppuccin",
    "catppuccin-latte",
    "catppuccin-frappe",
    "catppuccin-macchiato",
    "dracula",
    "nord",
    "gruvbox",
    "gruvbox-light",
    "one-dark",
    "one-light",
    "solarized",
    "solarized-light",
    "github-light",
    "monokai",
    "tokyo-night",
    "tokyo-night-day",
    "rose-pine",
    "rose-pine-dawn",
];

/// The theme named `name`, or `None` when it is not one quotr carries.
pub(super) fn resolve(name: &str) -> Option<Theme> {
    use Appearance::{Dark, Light};

    Some(match name {
        // VS Code's "Visual Studio Dark+", the syntax theme delta and bat use. Its chrome
        // anchors are the terminal's own palette, so the picker matches the shell around it.
        "vs-dark-plus" => bundled("vs-dark-plus", Dark, VS_DARK_PLUS_TM, VS_DARK_PLUS),

        // herdr's spelling of Catppuccin Mocha, so a name copied from its config resolves.
        "catppuccin" => bundled("catppuccin", Dark, MOCHA_TM, MOCHA),
        "catppuccin-latte" => derived("catppuccin-latte", Light, Embedded::CatppuccinLatte, LATTE),
        "catppuccin-frappe" => {
            derived("catppuccin-frappe", Dark, Embedded::CatppuccinFrappe, FRAPPE)
        }
        "catppuccin-macchiato" => {
            derived("catppuccin-macchiato", Dark, Embedded::CatppuccinMacchiato, MACCHIATO)
        }

        "dracula" => derived("dracula", Dark, Embedded::Dracula, DRACULA),
        "nord" => derived("nord", Dark, Embedded::Nord, NORD),
        "gruvbox" => derived("gruvbox", Dark, Embedded::GruvboxDark, GRUVBOX),
        "gruvbox-light" => derived("gruvbox-light", Light, Embedded::GruvboxLight, GRUVBOX_LIGHT),
        "one-dark" => derived("one-dark", Dark, Embedded::TwoDark, ONE_DARK),
        "one-light" => derived("one-light", Light, Embedded::OneHalfLight, ONE_LIGHT),
        "solarized" => derived("solarized", Dark, Embedded::SolarizedDark, SOLARIZED),
        "solarized-light" => {
            derived("solarized-light", Light, Embedded::SolarizedLight, SOLARIZED_LIGHT)
        }
        "github-light" => derived("github-light", Light, Embedded::Github, GITHUB_LIGHT),
        "monokai" => derived("monokai", Dark, Embedded::MonokaiExtended, MONOKAI),

        // herdr names whose syntax `two-face` lacks, paired with a vendored `.tmTheme`.
        "tokyo-night" => bundled("tokyo-night", Dark, TOKYO_NIGHT_TM, TOKYO_NIGHT),
        "tokyo-night-day" => bundled("tokyo-night-day", Light, TOKYO_NIGHT_DAY_TM, TOKYO_NIGHT_DAY),
        "rose-pine" => bundled("rose-pine", Dark, ROSE_PINE_TM, ROSE_PINE),
        "rose-pine-dawn" => bundled("rose-pine-dawn", Light, ROSE_PINE_DAWN_TM, ROSE_PINE_DAWN),

        _ => return None,
    })
}

/// The default, built directly rather than looked up so the fallback can never itself fail.
pub(super) fn default_theme() -> Theme {
    bundled(super::DEFAULT, Appearance::Dark, VS_DARK_PLUS_TM, VS_DARK_PLUS)
}

/// A theme paired with a `two-face` embedded syntax theme.
fn derived(name: &'static str, cast: Appearance, syntax: Embedded, a: Anchors) -> Theme {
    Theme { name, palette: derive(a, cast), syntax: SyntaxChoice::Embedded(syntax) }
}

/// A theme paired with a vendored `.tmTheme`'s bytes.
fn bundled(name: &'static str, cast: Appearance, syntax: &'static [u8], a: Anchors) -> Theme {
    Theme { name, palette: derive(a, cast), syntax: SyntaxChoice::Bundled(syntax) }
}

/// Vendored `.tmTheme` assets for the syntax themes `two-face` does not carry. Licenses are
/// listed in the README.
const MOCHA_TM: &[u8] = include_bytes!("../../../../assets/catppuccin-mocha.tmTheme");
const VS_DARK_PLUS_TM: &[u8] = include_bytes!("../../../../assets/vs-dark-plus.tmTheme");
const TOKYO_NIGHT_TM: &[u8] = include_bytes!("../../../../assets/tokyo-night.tmTheme");
const TOKYO_NIGHT_DAY_TM: &[u8] = include_bytes!("../../../../assets/tokyo-night-day.tmTheme");
const ROSE_PINE_TM: &[u8] = include_bytes!("../../../../assets/rose-pine.tmTheme");
const ROSE_PINE_DAWN_TM: &[u8] = include_bytes!("../../../../assets/rose-pine-dawn.tmTheme");

// Anchors, in slot order: base, text, code, strong, heading, link. The four accents sit in
// one narrow blue-violet band on purpose — a wide spread of hues fights the prose, which is
// what the reader is actually there for.
const VS_DARK_PLUS: Anchors = anchors(0x0d1117, 0xc1c1c1, 0x9cb4e8, 0xa8d8f0, 0xc586c0, 0x6ea8fe);
const MOCHA: Anchors = anchors(0x1e1e2e, 0xcdd6f4, 0xb4befe, 0x89dceb, 0xcba6f7, 0x89b4fa);
const LATTE: Anchors = anchors(0xeff1f5, 0x4c4f69, 0x7287fd, 0x04a5e5, 0x8839ef, 0x1e66f5);
const FRAPPE: Anchors = anchors(0x303446, 0xc6d0f5, 0xbabbf1, 0x99d1db, 0xca9ee6, 0x8caaee);
const MACCHIATO: Anchors = anchors(0x24273a, 0xcad3f5, 0xb7bdf8, 0x91d7e3, 0xc6a0f6, 0x8aadf4);
const DRACULA: Anchors = anchors(0x282a36, 0xf8f8f2, 0xbd93f9, 0x8be9fd, 0xff79c6, 0x8fa8ff);
const NORD: Anchors = anchors(0x2e3440, 0xd8dee9, 0xb48ead, 0x8fbcbb, 0xc9a7d6, 0x81a1c1);
const GRUVBOX: Anchors = anchors(0x282828, 0xebdbb2, 0xd3869b, 0x8ec07c, 0xe6a3bd, 0x83a598);
const GRUVBOX_LIGHT: Anchors = anchors(0xfbf1c7, 0x3c3836, 0x8f3f71, 0x427b58, 0xb1608f, 0x076678);
const ONE_DARK: Anchors = anchors(0x282c34, 0xabb2bf, 0xc678dd, 0x56b6c2, 0xe08fe8, 0x61afef);
const ONE_LIGHT: Anchors = anchors(0xfafafa, 0x383a42, 0xa626a4, 0x0184bc, 0xc04ec0, 0x4078f2);
const SOLARIZED: Anchors = anchors(0x002b36, 0x93a1a1, 0x6c71c4, 0x2aa198, 0xd33682, 0x268bd2);
const SOLARIZED_LIGHT: Anchors =
    anchors(0xfdf6e3, 0x586e75, 0x6c71c4, 0x2aa198, 0xd33682, 0x268bd2);
const GITHUB_LIGHT: Anchors = anchors(0xffffff, 0x1f2328, 0x8250df, 0x1b7c83, 0xa475f9, 0x0969da);
const MONOKAI: Anchors = anchors(0x272822, 0xf8f8f2, 0xae81ff, 0x66d9ef, 0xf92672, 0x8fa8ff);
const TOKYO_NIGHT: Anchors = anchors(0x1a1b26, 0xc0caf5, 0xbb9af7, 0x7dcfff, 0xc0a0ff, 0x7aa2f7);
const TOKYO_NIGHT_DAY: Anchors =
    anchors(0xe1e2e7, 0x3760bf, 0x9854f1, 0x007197, 0xb45cf1, 0x2e7de9);
const ROSE_PINE: Anchors = anchors(0x191724, 0xe0def4, 0xc4a7e7, 0x9ccfd8, 0xebbcba, 0x31748f);
const ROSE_PINE_DAWN: Anchors = anchors(0xfaf4ed, 0x575279, 0x907aa9, 0x56949f, 0xd7827e, 0x286983);
