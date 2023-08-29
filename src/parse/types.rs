use crate::util::Span;
use itertools::Itertools;
use std::fmt::{Display, Formatter};

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
    String(String),
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
    Import,
    Use,
    Builtin,
    As,
    Sload,
    Sstore,
    Create,
    Storage,
    Key,
    Index,
    Exists,
    ForAll,
    Return,
    Override,
    Sig,
    Description,
    Old,
    Offset,
    Slot,
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Ghost => write!(f, "ghost"),
            Token::Definition => write!(f, "definition"),
            Token::Rule => write!(f, "rule"),
            Token::Invariant => write!(f, "invariant"),
            Token::Methods => write!(f, "methods"),
            Token::Function => write!(f, "function"),
            Token::Mapping => write!(f, "mapping"),
            Token::Returns => write!(f, "returns"),
            Token::Filtered => write!(f, "filtered"),
            Token::Builtin => write!(f, "builtin"),
            Token::RoundOpen => write!(f, "("),
            Token::RoundClose => write!(f, ")"),
            Token::SquareOpen => write!(f, "["),
            Token::SquareClose => write!(f, "]"),
            Token::CurlyOpen => write!(f, "{{"),
            Token::CurlyClose => write!(f, "}}"),
            Token::Dot => write!(f, "."),
            Token::Comma => write!(f, ","),
            Token::Semicolon => write!(f, ";"),
            Token::Equals => write!(f, "="),
            Token::Arrow => write!(f, "=>"),
            Token::Axiom => write!(f, "axiom"),
            Token::Using => write!(f, "using"),
            Token::Hook => write!(f, "hook"),
            Token::Preserved => write!(f, "preserved"),
            Token::Import => write!(f, "import"),
            Token::Use => write!(f, "use"),
            Token::As => write!(f, "as"),
            Token::Sload => write!(f, "Sload"),
            Token::Sstore => write!(f, "Sstore"),
            Token::Create => write!(f, "Create"),
            Token::Storage => write!(f, "STORAGE"),
            Token::Exists => write!(f, "exists"),
            Token::ForAll => write!(f, "forall"),
            Token::Return => write!(f, "return"),
            Token::Override => write!(f, "override"),
            Token::Sig => write!(f, "sig"),
            Token::Description => write!(f, "description"),
            Token::Old => write!(f, "old"),
            Token::Key => write!(f, "KEY"),
            Token::Index => write!(f, "INDEX"),
            Token::Slot => write!(f, "slot"),
            Token::Offset => write!(f, "offset"),

            Token::Ident(data) | Token::Other(data) | Token::Number(data) | Token::String(data) => {
                write!(f, "{data}")
            }

            Token::CvlDocSlashed
            | Token::CvlDocStarred
            | Token::FreeFormSlashed
            | Token::FreeFormStarred
            | Token::SingleLineComment
            | Token::MultiLineComment => write!(f, "{self:?}"),
        }
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
        axioms: Option<Span>,
    },
    Ghost {
        name: String,
        ty_list: Vec<String>,
        returns: String,
        axioms: Option<Span>,
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
        definition: Span,
    },
    Invariant {
        name: String,
        params: Vec<(String, Option<String>)>,
        invariant: Span,
        filters: Option<Span>,
        proof: Option<Span>,
    },
    Import(String),
    UseRule {
        name: String,
        filters: Option<Span>,
    },
    UseBuiltinRule {
        name: String,
    },
    UseInvariant {
        name: String,
        proof: Option<Span>,
    },
    HookSload {
        loaded_value: (String, String),
        slot_pattern: Span,
        block: Span,
    },
    HookSstore {
        stored_value: (String, String),
        old_value: Option<(String, String)>,
        slot_pattern: Span,
        block: Span,
    },
    HookCreate {
        created: (String, String), // currently, the type is required to be `address`
        block: Span,
    },
    HookOpcode {
        opcode: String, // we over-approximate the opcode to be any ident
        params: Option<Vec<(String, String)>>,
        returned_value: Option<(String, String)>,
        block: Span,
    },
    Using {
        contract_name: String,
        spec_name: String,
    },
    ParseError,
}

#[derive(Debug, Clone)]
pub enum Style {
    Slashed,
    Starred,
}
