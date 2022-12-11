use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use itertools::Itertools;
use lsp_types::{Position, Range};
use ropey::Rope;
use std::fmt::Debug;
use std::ops::RangeBounds;

pub type Span = std::ops::Range<usize>;
pub type Spanned<T> = (T, Span);

pub type Ranged<T> = (T, Range);

#[derive(Clone)]
pub struct RangeConverter(Rope);

impl RangeConverter {
    pub fn new(rope: Rope) -> RangeConverter {
        RangeConverter(rope)
    }

    fn position_of(&self, char_idx: usize) -> Position {
        let rope = &self.0;
        assert!(char_idx <= rope.len_chars());

        let line = rope.char_to_line(char_idx);
        let line_start_idx = rope.line_to_char(line);
        let character = char_idx - line_start_idx;

        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    fn char_idx_of(&self, pos: Position) -> usize {
        let rope = &self.0;
        let [line, character] = [pos.line, pos.character].map(|n| n as usize);

        assert!(line <= rope.len_lines());

        let line_start_idx = rope.line_to_char(pos.line as usize);

        line_start_idx + character
    }

    pub fn to_range(&self, span: Span) -> Range {
        let [start, end] = [span.start, span.end].map(|char_idx| self.position_of(char_idx));
        Range { start, end }
    }

    pub fn to_span(&self, range: Range) -> Span {
        let [start, end] = [range.start, range.end].map(|range| self.char_idx_of(range));
        start..end
    }

    pub fn slice(&self, char_range: impl RangeBounds<usize>) -> Result<String> {
        let rope_slice = self.0.get_slice(char_range).wrap_err("not in range")?;
        Ok(rope_slice.to_string())
    }
}

pub trait SingleElement {
    type Item;
    fn single_element(self) -> Self::Item;
}

impl<T: Debug> SingleElement for Vec<T> {
    type Item = T;

    fn single_element(self) -> Self::Item {
        self.into_iter()
            .exactly_one()
            .expect("must have exactly one element")
    }
}
