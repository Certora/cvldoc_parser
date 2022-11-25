// pub mod diagnostics;
mod parse;
pub mod util;

use serde::{Deserialize, Serialize};
use std::fmt::Display;
use util::Span;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CvlElement {
    pub doc: Option<Documentation>,
    pub ast: Ast,
    span: Span,
    raw: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Documentation {
    pub tags: Vec<DocumentationTag>,
    raw: String,
}

impl std::fmt::Debug for Documentation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Documentation")
            .field("tags", &self.tags)
            .finish()
    }
}

impl std::fmt::Debug for CvlElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CvlElement")
            .field("doc", &self.doc)
            .field("ast", &self.ast)
            .finish()
    }
}

pub type Param = (String, Option<String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ast {
    FreeFormComment(String),
    Rule {
        name: String,
        params: Vec<Param>,
        filters: Option<String>,
        block: String,
    },
    Invariant {
        name: String,
        params: Vec<Param>,
        invariant: String,
        filters: Option<String>,
        proof: Option<String>,
    },
    Function {
        name: String,
        params: Vec<Param>,
        returns: Option<String>,
        block: String,
    },
    Definition {
        name: String,
        params: Vec<Param>,
        returns: String,
        definition: String,
    },
    Ghost {
        name: String,
        ty_list: Vec<String>,
        returns: String,
        block: Option<String>,
    },
    GhostMapping {
        name: String,
        mapping: String,
        block: Option<String>,
    },
    Methods {
        block: String,
    },
}

impl CvlElement {
    pub fn title(&self) -> Option<String> {
        let from_title_tag = self.doc_tags().and_then(|tags| {
            tags.iter().find_map(|tag| {
                if tag.kind == TagKind::Title {
                    Some(tag.description.clone())
                } else {
                    None
                }
            })
        });
        let from_name = || self.ast.name().map(String::from);

        from_title_tag.or_else(from_name)
    }

    pub fn span(&self) -> Span {
        let start = self
            .doc_tags()
            .and_then(|tags| tags.first())
            .map(|tag| tag.span.start)
            .unwrap_or(self.span.start);
        let end = self.span.end;
        start..end
    }

    pub fn doc_tags(&self) -> Option<&[DocumentationTag]> {
        self.doc.as_ref().map(|doc| doc.tags.as_slice())
    }

    pub fn raw(&self) -> String {
        if let Some(doc) = self.doc.as_ref() {
            let mut raw = doc.raw.clone();
            raw.push_str(&self.raw);
            raw
        } else {
            self.raw.clone()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentationTag {
    pub kind: TagKind,
    pub description: String,
    pub span: Span,
}

impl DocumentationTag {
    pub fn new(kind: TagKind, description: String, span: Span) -> DocumentationTag {
        DocumentationTag {
            kind,
            description,
            span,
        }
    }

    pub fn tag_name_span(&self) -> Option<Span> {
        if let Some(ampersat_pos) = self.description.chars().position(|c| c == '@') {
            let start = self.span.start + ampersat_pos;
            let end = start + self.kind.len();
            Some(start..end)
        } else {
            None
        }
    }

    pub fn param_name(&self) -> Option<&str> {
        match self.kind {
            TagKind::Param => self
                .description
                .trim_start()
                .split_once(|c: char| c.is_ascii_whitespace())
                .map(|(param_name, _)| param_name),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Default, Serialize, Deserialize)]
pub enum TagKind {
    Title,
    #[default]
    Notice, //if tag kind is not specified, it is considered @notice
    Dev,
    Param,
    Return,
    Formula,
    Unexpected(String),
}

impl Display for TagKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TagKind::Title => "title",
            TagKind::Notice => "notice",
            TagKind::Dev => "dev",
            TagKind::Param => "param",
            TagKind::Return => "return",
            TagKind::Formula => "formula",
            TagKind::Unexpected(s) => s.as_str(),
        };
        write!(f, "{s}")
    }
}

impl TagKind {
    pub fn unexpected_tag(&self) -> Option<&str> {
        match self {
            TagKind::Unexpected(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub(crate) fn len(&self) -> usize {
        let len_without_ampersat = match self {
            TagKind::Dev => 3,
            TagKind::Title | TagKind::Param => 5,
            TagKind::Notice | TagKind::Return => 6,
            TagKind::Formula => 7,
            TagKind::Unexpected(s) => s.len(),
        };

        len_without_ampersat + 1
    }
}

impl From<&str> for TagKind {
    fn from(mut s: &str) -> Self {
        if let Some(trimmed) = s.strip_prefix('@') {
            s = trimmed;
        }
        match s {
            "title" => TagKind::Title,
            "notice" => TagKind::Notice,
            "dev" => TagKind::Dev,
            "param" => TagKind::Param,
            "return" => TagKind::Return,
            "formula" => TagKind::Formula,
            _ => TagKind::Unexpected(s.to_string()),
        }
    }
}

impl From<String> for TagKind {
    fn from(mut s: String) -> Self {
        if s.starts_with('@') {
            s.remove(0);
        }

        match s.as_str() {
            "title" => TagKind::Title,
            "notice" => TagKind::Notice,
            "dev" => TagKind::Dev,
            "param" => TagKind::Param,
            "return" => TagKind::Return,
            "formula" => TagKind::Formula,
            _ => TagKind::Unexpected(s),
        }
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Ast::Rule { .. } => "rule",
            Ast::Invariant { .. } => "invariant",
            Ast::Function { .. } => "function",
            Ast::Definition { .. } => "definition",
            Ast::Ghost { .. } | Ast::GhostMapping { .. } => "ghost",
            Ast::Methods { .. } => "methods",

            Ast::FreeFormComment(..) => "freeform comment",
        };

        write!(f, "{kind}")
    }
}

impl Ast {
    pub fn name(&self) -> Option<&str> {
        match self {
            Ast::Rule { name, .. }
            | Ast::Invariant { name, .. }
            | Ast::Function { name, .. }
            | Ast::Definition { name, .. }
            | Ast::Ghost { name, .. }
            | Ast::GhostMapping { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    pub fn params(&self) -> Option<&[Param]> {
        match self {
            Ast::Rule { params, .. }
            | Ast::Invariant { params, .. }
            | Ast::Function { params, .. }
            | Ast::Definition { params, .. } => Some(params),
            _ => None,
        }
    }

    pub fn block(&self) -> Option<&str> {
        match self {
            Ast::Rule { block, .. } | Ast::Function { block, .. } | Ast::Methods { block } => {
                Some(block.as_str())
            }

            Ast::Invariant { proof: block, .. }
            | Ast::Ghost { block, .. }
            | Ast::GhostMapping { block, .. } => block.as_ref().map(String::as_str),

            Ast::Definition { .. } => None,
            _ => None,
        }
    }

    pub fn returns(&self) -> Option<&str> {
        match self {
            Ast::Function { returns, .. } => returns.as_ref().map(String::as_str),
            Ast::Definition { returns, .. } | Ast::Ghost { returns, .. } => Some(returns.as_str()),
            _ => None,
        }
    }

    pub fn ty_list(&self) -> Option<&[String]> {
        match self {
            Ast::Ghost { ty_list, .. } => Some(ty_list),
            _ => None,
        }
    }

    pub fn filters(&self) -> Option<&str> {
        match self {
            Ast::Rule { filters, .. } | Ast::Invariant { filters, .. } => {
                filters.as_ref().map(String::as_str)
            }
            _ => None,
        }
    }

    pub fn invariant(&self) -> Option<&str> {
        match self {
            Ast::Invariant { invariant, .. } => Some(invariant.as_str()),
            _ => None,
        }
    }

    pub fn mapping(&self) -> Option<&str> {
        match self {
            Ast::GhostMapping { mapping, .. } => Some(mapping.as_str()),
            _ => None,
        }
    }

    pub fn definition(&self) -> Option<&str> {
        match self {
            Ast::Definition { definition, .. } => Some(definition.as_str()),
            _ => None,
        }
    }
}
