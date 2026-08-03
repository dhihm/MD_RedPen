//! Grapheme, word, and Markdown paragraph navigation.

use super::Editor;

impl Editor {
    /// Moves to the next selectable grapheme.
    pub fn move_right(&mut self) {
        if let Some(next) = self.next_selectable(self.cursor.saturating_add(1), 1) {
            self.cursor = next;
        }
    }

    /// Moves to the previous selectable grapheme.
    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        if let Some(previous) = self.next_selectable(self.cursor - 1, -1) {
            self.cursor = previous;
        }
    }

    /// Moves to the first selectable grapheme in the next Markdown paragraph.
    pub fn move_paragraph_down(&mut self) -> bool {
        let Some(current) = self.current_paragraph() else {
            return false;
        };
        let Some(next) = self
            .projection
            .graphemes()
            .iter()
            .position(|item| item.is_selectable() && item.paragraph() > current)
        else {
            return false;
        };
        self.cursor = next;
        true
    }

    /// Moves to the first selectable grapheme in the previous Markdown paragraph.
    pub fn move_paragraph_up(&mut self) -> bool {
        let Some(current) = self.current_paragraph() else {
            return false;
        };
        let Some(previous) = self
            .projection
            .graphemes()
            .iter()
            .rev()
            .find(|item| item.is_selectable() && item.paragraph() < current)
            .map(crate::markdown::DisplayGrapheme::paragraph)
        else {
            return false;
        };
        let Some(target) = self
            .projection
            .graphemes()
            .iter()
            .position(|item| item.is_selectable() && item.paragraph() == previous)
        else {
            return false;
        };
        self.cursor = target;
        true
    }

    /// Selects the current contiguous rendered word.
    pub fn select_current_word(&mut self) {
        let graphemes = self.projection.graphemes();
        let Some(class) = graphemes
            .get(self.cursor)
            .and_then(|item| word_class(item.text()))
        else {
            return;
        };
        let mut start = self.cursor;
        while start > 0 && word_class(graphemes[start - 1].text()) == Some(class) {
            start -= 1;
        }
        let mut end = self.cursor;
        while end + 1 < graphemes.len() && word_class(graphemes[end + 1].text()) == Some(class) {
            end += 1;
        }
        self.anchor = Some(start);
        self.cursor = end;
    }

    fn current_paragraph(&self) -> Option<usize> {
        self.projection
            .graphemes()
            .get(self.cursor)
            .map(crate::markdown::DisplayGrapheme::paragraph)
    }

    fn next_selectable(&self, start: usize, direction: isize) -> Option<usize> {
        let length = self.projection.graphemes().len();
        let mut index = start;
        while index < length {
            if self.projection.graphemes()[index].is_selectable() {
                return Some(index);
            }
            if direction < 0 {
                if index == 0 {
                    return None;
                }
                index -= 1;
            } else {
                index = index.saturating_add(1);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WordClass {
    Ascii,
    NonAscii,
}

fn word_class(grapheme: &str) -> Option<WordClass> {
    let mut characters = grapheme
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == '_' || *character == '-');
    let first = characters.next()?;
    Some(if first.is_ascii() {
        WordClass::Ascii
    } else {
        WordClass::NonAscii
    })
}
