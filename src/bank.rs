//! The quote+question pairs waiting to go out together, and the actions that steer them.

use serde::{Deserialize, Serialize};
use transcript::Pos;

use crate::app::{App, Mode};

/// One quote and the question that goes under it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pair {
    pub from: Pos,
    pub to: Pos,
    /// Empty for a bare quote — the block is then just the quoted lines.
    pub question: String,
}

/// The batch `S` sends, in the order the pairs were banked.
#[derive(Debug, Default)]
pub struct Bank {
    pairs: Vec<Pair>,
}

impl Bank {
    pub fn len(&self) -> usize {
        self.pairs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pairs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pair> {
        self.pairs.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Pair> {
        self.pairs.get(index)
    }

    /// Append a pair; returns its index.
    pub fn add(&mut self, pair: Pair) -> usize {
        self.pairs.push(pair);
        self.pairs.len() - 1
    }

    /// Replace the question of the pair at `index`, leaving its quote alone.
    pub fn set_question(&mut self, index: usize, question: String) {
        if let Some(pair) = self.pairs.get_mut(index) {
            pair.question = question;
        }
    }

    /// Drop the pair at `index`. The ones after it shift down, so the gutter renumbers.
    pub fn remove(&mut self, index: usize) {
        if index < self.pairs.len() {
            self.pairs.remove(index);
        }
    }

    /// The pair whose range covers `line` — what `e` and `d` act on. First one wins, so
    /// overlapping pairs are reached in bank order.
    pub fn at_line(&self, line: usize) -> Option<usize> {
        self.pairs.iter().position(|pair| (pair.from.line..=pair.to.line).contains(&line))
    }
}

impl From<Vec<Pair>> for Bank {
    fn from(pairs: Vec<Pair>) -> Self {
        Self { pairs }
    }
}

impl App {
    /// Save the typed question — as a new pair over the live selection, or onto the banked
    /// pair `e` reopened — and go back to browsing.
    pub fn bank_pair(&mut self) {
        let Mode::Ask { editing } = self.mode else { return };
        let question = self.question.trim().to_owned();
        let Some(index) = self.store(editing, question) else {
            "nothing selected".clone_into(&mut self.status);
            return;
        };
        self.clear_range();
        self.mode = Mode::Browse;
        self.status = format!("pair {} saved — {} banked, S sends", index + 1, self.bank.len());
    }

    /// Write the question onto the pair `e` reopened, or anchor a new pair to the live
    /// selection. `None` when the selection has nothing to quote.
    fn store(&mut self, editing: Option<usize>, question: String) -> Option<usize> {
        if let Some(index) = editing {
            self.bank.set_question(index, question);
            return Some(index);
        }
        let (from, to) = self.range();
        if self.transcript.slice(from, to).is_empty() {
            return None;
        }
        Some(self.bank.add(Pair { from, to, question }))
    }

    /// Reopen the question of the banked pair under the caret.
    pub fn edit_pair(&mut self) {
        let Some(index) = self.bank.at_line(self.cursor.line) else {
            "no banked pair here".clone_into(&mut self.status);
            return;
        };
        self.question = self.bank.get(index).map(|pair| pair.question.clone()).unwrap_or_default();
        self.mode = Mode::Ask { editing: Some(index) };
    }

    /// Drop the banked pair under the caret.
    pub fn delete_pair(&mut self) {
        let Some(index) = self.bank.at_line(self.cursor.line) else {
            "no banked pair here".clone_into(&mut self.status);
            return;
        };
        self.bank.remove(index);
        self.status = format!("pair {} deleted — {} left", index + 1, self.bank.len());
    }

    /// Bank the live selection as a bare quote, so `S` carries what is on screen too.
    pub(crate) fn bank_selection(&mut self) {
        if self.anchor.is_none() {
            return;
        }
        let (from, to) = self.range();
        if self.transcript.slice(from, to).is_empty() {
            return;
        }
        self.bank.add(Pair { from, to, question: String::new() });
        self.clear_range();
    }

    /// The whole batch as one markdown blob, pairs separated by `---`.
    pub(crate) fn compose(&self) -> String {
        let blocks: Vec<String> = self
            .bank
            .iter()
            .map(|pair| {
                let quoted = self.transcript.slice(pair.from, pair.to);
                let question = pair.question.trim();
                export::block(&quoted, (!question.is_empty()).then_some(question))
            })
            .collect();
        export::batch(&blocks)
    }
}

#[cfg(test)]
mod tests {
    use transcript::Pos;

    use super::{Bank, Pair};

    fn pair(from: usize, to: usize) -> Pair {
        Pair { from: Pos::line_start(from), to: Pos::new(to, 3), question: String::new() }
    }

    #[test]
    fn finds_the_pair_a_line_falls_inside() {
        let bank = Bank::from(vec![pair(2, 4), pair(9, 9)]);
        assert_eq!(bank.at_line(2), Some(0));
        assert_eq!(bank.at_line(3), Some(0));
        assert_eq!(bank.at_line(9), Some(1));
        assert_eq!(bank.at_line(5), None);
    }

    #[test]
    fn deleting_shifts_the_rest_down_so_they_renumber() {
        let mut bank = Bank::from(vec![pair(2, 4), pair(9, 9)]);
        bank.remove(0);
        assert_eq!(bank.len(), 1);
        assert_eq!(bank.at_line(9), Some(0));
    }

    #[test]
    fn editing_replaces_the_question_and_leaves_the_quote() {
        let mut bank = Bank::from(vec![pair(2, 4)]);
        bank.set_question(0, "why?".to_owned());
        assert_eq!(bank.get(0).map(|pair| pair.question.as_str()), Some("why?"));
        assert_eq!(bank.get(0).map(|pair| pair.from), Some(Pos::line_start(2)));
    }
}
