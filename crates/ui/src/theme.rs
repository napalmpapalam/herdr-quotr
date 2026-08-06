//! The color model: named themes, each a [`Palette`] plus the syntax theme it pairs with.
//!
//! One selection sets both, so chrome and code never desync. The picker paints its own ground
//! (`Palette::base`): the popup is full-screen, so borrowing the terminal's would leave a light
//! theme unreadable on a dark terminal.

mod catalog;
mod palette;

use two_face::theme::EmbeddedThemeName;

pub use crate::theme::{catalog::NAMES, palette::Palette};

/// The default theme name — the one quotr ships tuned to.
pub const DEFAULT: &str = "vs-dark-plus";

/// A resolved theme: its name, the chrome [`Palette`], and its paired syntax theme.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub palette: Palette,
    pub syntax: SyntaxChoice,
}

/// Where a theme's syntax colors come from: vendored `.tmTheme` bytes, or the `two-face` set.
#[derive(Clone, Copy, Debug)]
pub enum SyntaxChoice {
    Bundled(&'static [u8]),
    Embedded(EmbeddedThemeName),
}

/// The theme named `name`, or `None` when it is not one quotr carries.
pub fn resolve(name: &str) -> Option<Theme> {
    catalog::resolve(name)
}

/// The default theme, for an unset name.
pub fn default_theme() -> Theme {
    catalog::default_theme()
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT, NAMES, default_theme, palette::contrast, resolve};

    /// Floors the fill steps are tuned against. Solarized's low-contrast `text` anchor is the
    /// binding case for both; every other theme clears them with room to spare.
    const MIN_TEXT_CONTRAST: f64 = 3.5;
    const MIN_MARKER_CONTRAST: f64 = 2.5;

    #[test]
    fn every_named_theme_resolves_to_itself() {
        for name in NAMES {
            assert_eq!(resolve(name).map(|t| t.name), Some(name), "{name} should resolve");
        }
    }

    #[test]
    fn an_unknown_name_resolves_to_nothing_and_the_default_stands_in() {
        assert!(resolve("nope").is_none());
        assert!(resolve("terminal").is_none());
        assert_eq!(default_theme().name, DEFAULT);
    }

    /// Prose and markers, the two things a selection has to keep readable. Accents are the
    /// theme author's own contrast choice — some read no better on the base than on the fill.
    #[test]
    fn text_and_lifted_markers_stay_legible_on_the_selection_fill() {
        for name in NAMES {
            let Some(p) = resolve(name).map(|t| t.palette) else { continue };
            assert!(
                contrast(p.text, p.select_bg) >= MIN_TEXT_CONTRAST,
                "{name}: prose is unreadable selected",
            );
            assert!(
                contrast(p.on_fill(p.overlay0), p.select_bg) >= MIN_MARKER_CONTRAST,
                "{name}: markers vanish on the selection fill even after the lift",
            );
        }
    }

    #[test]
    fn the_lift_is_what_saves_the_markers() {
        // Without it a dim marker sits one surface step above the fill and disappears.
        for name in NAMES {
            let Some(p) = resolve(name).map(|t| t.palette) else { continue };
            assert!(
                contrast(p.on_fill(p.overlay0), p.select_bg) > contrast(p.overlay0, p.select_bg),
                "{name}: the lift gains nothing",
            );
        }
    }

    #[test]
    fn every_accent_is_a_hue_of_its_own() {
        // Four accents, four jobs — a repeat would make two constructs indistinguishable.
        for name in NAMES {
            let Some(p) = resolve(name).map(|t| t.palette) else { continue };
            let accents = [p.code, p.strong, p.heading, p.link];
            for (i, a) in accents.iter().enumerate() {
                assert!(
                    !accents.iter().skip(i + 1).any(|b| b == a),
                    "{name}: {a:?} is used for two different things",
                );
            }
        }
    }
}
