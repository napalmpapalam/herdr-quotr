//! The text typed into the question box, and the caret editing it.

/// A question being written, with the caret as a character offset into it — the same unit as
/// [`markup::Pos::col`], so nothing in the picker counts bytes.
#[derive(Debug, Default)]
pub struct Question {
    text: String,
    caret: usize,
}

impl Question {
    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn caret(&self) -> usize {
        self.caret
    }

    /// Load a question for editing, caret at the end — where `e` leaves it.
    pub fn set(&mut self, text: String) {
        self.caret = text.chars().count();
        self.text = text;
    }

    pub fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
    }

    pub fn insert(&mut self, c: char) {
        let at = self.byte_at(self.caret);
        self.text.insert(at, c);
        self.caret += 1;
    }

    /// Remove the character before the caret.
    pub fn backspace(&mut self) {
        let Some(before) = self.caret.checked_sub(1) else { return };
        self.text.remove(self.byte_at(before));
        self.caret = before;
    }

    /// Remove the character the caret sits on, leaving the caret where it is.
    pub fn delete(&mut self) {
        let at = self.byte_at(self.caret);
        if at < self.text.len() {
            self.text.remove(at);
        }
    }

    pub fn left(&mut self) {
        self.caret = self.caret.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.caret = self.caret.saturating_add(1).min(self.len());
    }

    pub fn home(&mut self) {
        self.caret = 0;
    }

    pub fn end(&mut self) {
        self.caret = self.len();
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Byte index of character `col`, or the end of the text past the last one.
    fn byte_at(&self, col: usize) -> usize {
        self.text.char_indices().nth(col).map_or(self.text.len(), |(at, _)| at)
    }
}

#[cfg(test)]
mod tests {
    use super::Question;

    fn typed(text: &str) -> Question {
        let mut question = Question::default();
        for c in text.chars() {
            question.insert(c);
        }
        question
    }

    #[test]
    fn typing_leaves_the_caret_after_the_text() {
        let question = typed("why?");
        assert_eq!(question.text(), "why?");
        assert_eq!(question.caret(), 4);
    }

    #[test]
    fn a_character_lands_where_the_caret_sits() {
        let mut question = typed("wh?");
        question.left();
        question.insert('y');
        assert_eq!(question.text(), "why?");
        assert_eq!(question.caret(), 3);
    }

    #[test]
    fn backspace_takes_the_character_before_the_caret() {
        let mut question = typed("whyy?");
        question.left();
        question.backspace();
        assert_eq!(question.text(), "why?");
        assert_eq!(question.caret(), 3);
    }

    #[test]
    fn delete_takes_the_character_under_the_caret() {
        let mut question = typed("whyx?");
        question.left();
        question.left();
        question.delete();
        assert_eq!(question.text(), "why?");
        assert_eq!(question.caret(), 3);
    }

    #[test]
    fn the_caret_stops_at_both_ends() {
        let mut question = typed("hi");
        question.home();
        question.left();
        question.backspace();
        assert_eq!((question.text(), question.caret()), ("hi", 0));
        question.end();
        question.right();
        question.delete();
        assert_eq!((question.text(), question.caret()), ("hi", 2));
    }

    #[test]
    fn a_wide_glyph_counts_as_one_character() {
        let mut question = typed("日本語");
        question.left();
        question.insert('x');
        assert_eq!(question.text(), "日本x語");
        assert_eq!(question.caret(), 3);
        question.backspace();
        assert_eq!(question.text(), "日本語");
    }

    #[test]
    fn editing_a_banked_question_opens_at_its_end() {
        let mut question = Question::default();
        question.set("why?".to_owned());
        assert_eq!(question.caret(), 4);
        question.clear();
        assert_eq!((question.text(), question.caret()), ("", 0));
    }
}
