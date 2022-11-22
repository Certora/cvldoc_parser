use crate::util::span_to_range::Span;

#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub enum Terminator {
    CRLF,
    LF,
    CR,
    EOF,
}

pub(super) trait JoinToString {
    fn join_to_string(self) -> String;
}

pub(super) trait JoinToString2 {
    fn join_to_string(self) -> String;
}

impl<I> JoinToString for I
where
    I: IntoIterator<Item = TerminatedLine>,
{
    fn join_to_string(self) -> String {
        let mut joined = String::new();
        for line in self {
            joined.extend(line.content);
            joined.extend(line.terminator.as_char_slice());
        }

        while joined.ends_with(&['\r', '\n']) {
            joined.pop();
        }

        joined
    }
}

impl Terminator {
    pub(super) fn as_char_slice(&self) -> &[char] {
        match self {
            Terminator::CRLF => &['\r', '\n'],
            Terminator::LF => &['\n'],
            Terminator::CR => &['\r'],
            Terminator::EOF => &[],
        }
    }

    pub(super) fn as_str(&self) -> &str {
        match self {
            Terminator::CRLF => "\r\n",
            Terminator::LF => "\n",
            Terminator::CR => "\r",
            Terminator::EOF => "",
        }
    }
}

impl From<Terminator> for &str {
    fn from(ter: Terminator) -> Self {
        match ter {
            Terminator::CRLF => "\r\n",
            Terminator::LF => "\n",
            Terminator::CR => "\r",
            Terminator::EOF => "",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TerminatedLine {
    pub content: Vec<char>,
    pub terminator: Terminator,
}


impl ToString for TerminatedLine {
    fn to_string(&self) -> String {
        let term_chars: &[char] = self.terminator.as_char_slice();
        self.content.iter().chain(term_chars).collect()
    }
}

impl TerminatedLine {
    pub(super) fn new(content: Vec<char>, terminator: Terminator) -> TerminatedLine {
        TerminatedLine {
            content,
            terminator,
        }
    }

    pub(super) fn trim_end(mut self, padding: &[char]) -> TerminatedLine {
        if let Some(content_start) = self.content.iter().rposition(|c| !padding.contains(c)) {
            self.content.truncate(content_start + 1);
        } else {
            self.content.clear();
        }

        TerminatedLine::new(self.content, self.terminator)
    }

    pub(super) fn trim_start(mut self, padding: &[char]) -> TerminatedLine {
        let content_start = self
            .content
            .iter()
            .position(|c| !padding.contains(c))
            .unwrap_or(self.content.len());
        let content = self.content.split_off(content_start);

        TerminatedLine::new(content, self.terminator)
    }

    pub(super) fn trim(self, padding: &[char]) -> TerminatedLine {
        self.trim_start(padding).trim_end(padding)
    }

    pub(super) fn from_char_slice(line: &[char]) -> TerminatedLine {
        use Terminator::*;

        for terminator in [CRLF, LF, CR, EOF] {
            let terminator_chars = terminator.as_char_slice();

            if line.ends_with(terminator_chars) {
                let line = &line[..line.len() - terminator_chars.len()];
                return TerminatedLine::new(line.to_vec(), terminator);
            }
        }

        unreachable!()
    }
}

