use std::fmt::{Display, Formatter};
use crate::util::Span;
use itertools::Itertools;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Token {
    Ghost,
    Definition,
    Rule,
    Invariant,
    Methods,
    Function,
    Mapping,
    Returns,
    Filtered,
    CvlDocSlashed,
    CvlDocStarred,
    FreeFormSlashed,
    FreeFormStarred,
    RoundOpen,
    RoundClose,
    SquareOpen,
    SquareClose,
    CurlyOpen,
    CurlyClose,
    Ident(String),
    Number(String),
    Other(String),
    Dot,
    SingleLineComment,
    MultiLineComment,
    Comma,
    Semicolon,
    Equals,
    Arrow,
    Axiom,
    Using,
    Hook,
    Preserved,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Token::Ghost => "ghost",
            Token::Definition => "definition",
            Token::Rule => "rule",
            Token::Invariant => "invariant",
            Token::Methods => "methods",
            Token::Function => "function",
            Token::Mapping => "mapping",
            Token::Returns => "returns",
            Token::Filtered => "filtered",
            Token::Ident(data) | Token::Other(data) | Token::Number(data) => data.as_str(),
            Token::RoundOpen => "(",
            Token::RoundClose => ")",
            Token::SquareOpen => "[",
            Token::SquareClose => "]",
            Token::CurlyOpen => "{",
            Token::CurlyClose => "}",
            Token::Dot => ".",
            Token::Comma => ",",
            Token::Semicolon => ";",
            Token::Equals => "=",
            Token::Arrow => "=>",
            Token::Axiom => "axiom",
            Token::Using => "using",
            Token::Hook => "hook",
            Token::Preserved => "preserved",
            _ => panic!("{self:?}"),
        };

        write!(f, "{s}")
    }
}

impl FromIterator<Token> for String {
    fn from_iter<T: IntoIterator<Item = Token>>(iter: T) -> Self {
        iter.into_iter().join(" ")
    }
}

#[derive(Debug, Clone)]
pub enum Intermediate {
    FreeFormComment(Style, Span),
    Documentation(Style, Span),
    Methods(Span),
    Function {
        name: String,
        params: Vec<(String, Option<String>)>,
        returns: Option<String>,
        block: Span,
    },
    GhostMapping {
        name: String,
        mapping: String,
        block: Option<Span>,
    },
    Ghost {
        name: String,
        ty_list: Vec<String>,
        returns: String,
        block: Option<Span>,
    },
    Rule {
        name: String,
        params: Vec<(String, Option<String>)>,
        filters: Option<Span>,
        block: Span,
    },
    Definition {
        name: String,
        params: Vec<(String, Option<String>)>,
        returns: String,
        definition: String,
    },
    Invariant {
        name: String,
        params: Vec<(String, Option<String>)>,
        invariant: Span,
        filters: Option<Span>,
        proof: Option<Span>,
    },
    ParseError,
}

#[derive(Debug, Clone)]
pub enum Style {
    Slashed,
    Starred,
}