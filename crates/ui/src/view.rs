//! What the app hands the paint layer, and what the paint layer hands back.

use ratatui::layout::Rect;

/// Who wrote a line, as far as painting is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Agent,
    User,
    Gap,
}

/// One source line, as the paint layer sees it.
#[derive(Debug, Clone, Copy)]
pub struct SourceLine<'a> {
    pub text: &'a str,
    pub tone: Tone,
}

/// Everything the picker paints, filled by the app each frame.
#[derive(Debug)]
pub struct View<'a> {
    pub lines: &'a [SourceLine<'a>],
    pub cursor: usize,
    /// Inclusive source-line range, when a selection is live.
    pub selection: Option<(usize, usize)>,
    /// First source line painted.
    pub scroll: usize,
    /// The question being typed, when the box is open.
    pub question: Option<&'a str>,
    pub status: &'a str,
}

/// What the last frame put on screen — how the app scrolls and hit-tests the mouse.
#[derive(Debug, Default)]
pub struct Painted {
    /// Source line index for each painted row, top to bottom.
    rows: Vec<usize>,
    /// Source lines that fit whole.
    lines: usize,
    /// Screen row the first painted row sits on.
    top: u16,
}

impl Painted {
    pub(crate) fn new(rows: Vec<usize>, lines: usize, area: Rect) -> Self {
        Self { rows, lines, top: area.y }
    }

    /// How many source lines a page holds — at least one, so paging always moves.
    pub fn page(&self) -> usize {
        self.lines.max(1)
    }

    /// Source line under a screen row.
    pub fn hit(&self, y: u16) -> Option<usize> {
        let row = y.checked_sub(self.top)?;
        self.rows.get(usize::from(row)).copied()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::Painted;

    #[test]
    fn hit_maps_a_screen_row_to_the_wrapped_source_line() {
        // Line 7 wrapped to three rows, line 8 to one, starting at screen row 5.
        let painted = Painted::new(vec![7, 7, 7, 8], 2, Rect::new(0, 5, 40, 4));
        assert_eq!(painted.hit(4), None);
        assert_eq!(painted.hit(6), Some(7));
        assert_eq!(painted.hit(8), Some(8));
        assert_eq!(painted.hit(9), None);
    }
}
