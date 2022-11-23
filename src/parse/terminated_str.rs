#[allow(clippy::upper_case_acronyms)]
#[derive(Clone, Debug)]
pub enum Terminator {
    CRLF,
    LF,
    CR,
    EOF,
}

impl Terminator {
    pub(super) fn as_str(&self) -> &str {
        match self {
            Terminator::CRLF => "\r\n",
            Terminator::LF => "\n",
            Terminator::CR => "\r",
            Terminator::EOF => "",
        }
    }
}

pub struct TerminatedStr<'a> {
    pub content: &'a str,
    ter: Terminator,
}

impl<'a> From<&'a str> for TerminatedStr<'a> {
    fn from(line: &'a str) -> Self {
        use Terminator::*;

        for ter in [CRLF, LF, CR, EOF] {
            if let Some(without_ter) = line.strip_suffix(ter.as_str()) {
                return TerminatedStr {
                    content: without_ter,
                    ter,
                };
            }
        }

        unreachable!()
    }
}

impl ToString for TerminatedStr<'_> {
    fn to_string(&self) -> String {
        let line_chars = self.content.chars();
        let ter_chars = self.ter.as_str().chars();
        line_chars.chain(ter_chars).collect()
    }
}

impl<'a> FromIterator<TerminatedStr<'a>> for String {
    fn from_iter<T: IntoIterator<Item = TerminatedStr<'a>>>(iter: T) -> Self {
        let mut joined = String::new();
        for ter_line in iter {
            joined.push_str(ter_line.content);
            joined.push_str(ter_line.ter.as_str());
        }

        while joined.ends_with(&['\r', '\n']) {
            joined.pop();
        }

        joined
    }
}

