pub mod diagnostics;
pub mod parse;
pub mod util;

use color_eyre::eyre::bail;
use serde::Serialize;
use std::fmt::{Debug, Display};
use std::sync::Arc;
use util::{ByteSpan, Span};

#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct CvlElement {
    pub doc: Vec<DocumentationTag>,
    pub ast: Ast,
    pub element_span: Span,
    pub doc_span: Option<Span>,
    #[serde(skip)]
    pub src: Arc<str>,
}

impl Debug for CvlElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CvlElement")
            .field("doc", &self.doc)
            .field("ast", &self.ast)
            // .field("element_span", &self.element_span)
            // .field("doc_span", &self.doc_span)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Param {
    pub ty: String,
    pub name: String,
}
impl Param {
    pub fn new<S1: ToString, S2: ToString>(ty: S1, name: S2) -> Param {
        Param {
            ty: ty.to_string(),
            name: name.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "type")]
pub enum Ast {
    FreeFormComment {
        text: String,
    },
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
    GhostFunction {
        name: String,
        ty_list: Vec<String>,
        returns: String,
        axioms: Option<String>,
    },
    GhostMapping {
        name: String,
        mapping: String,
        axioms: Option<String>,
    },
    Methods {
        block: String,
    },
    Import {
        imported: String,
    },
    Using {
        contract_name: String,
        spec_name: String,
    },
    UseRule {
        name: String,
        filters: Option<String>,
    },
    UseBuiltinRule {
        name: String,
    },
    UseInvariant {
        name: String,
        proof: Option<String>,
    },
    HookSload {
        loaded: Param,
        slot_pattern: String,
        block: String,
    },
    HookSstore {
        stored: Param,
        old: Option<Param>,
        slot_pattern: String,
        block: String,
    },
    HookCreate {
        created: Param,
        block: String,
    },
    HookOpcode {
        opcode: String,
        params: Vec<Param>,
        returns: Option<Param>,
        block: String,
    },
}

impl CvlElement {
    pub fn title(&self) -> Option<String> {
        let from_title_tag = self.doc.iter().find_map(|tag| {
            if matches!(tag.kind, TagKind::Title) {
                Some(tag.description.clone())
            } else {
                None
            }
        });
        let from_name = || self.ast.name().map(ToOwned::to_owned);

        from_title_tag.or_else(from_name)
    }

    pub fn span(&self) -> Span {
        let start = if let Some(doc_span) = &self.doc_span {
            doc_span.start
        } else {
            self.element_span.start
        };

        let end = self.element_span.end;

        start..end
    }

    pub fn raw(&self) -> &str {
        self.span().byte_slice(&self.src).unwrap()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, PartialEq, Eq, Clone, Hash, Default, Serialize)]
pub enum TagKind {
    Title,
    #[default]
    Notice, //if tag kind is not specified, it is considered @notice
    Dev,
    Param,
    Return,
    Formula,
}

impl TagKind {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            TagKind::Title => "title",
            TagKind::Notice => "notice",
            TagKind::Dev => "dev",
            TagKind::Param => "param",
            TagKind::Return => "return",
            TagKind::Formula => "formula",
        }
    }

    pub(crate) fn len(&self) -> usize {
        let len_without_ampersat = self.as_str().len();
        len_without_ampersat + 1
    }
}

impl TryFrom<&str> for TagKind {
    type Error = color_eyre::Report;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        let s = s.strip_prefix('@').unwrap_or(s);

        match s {
            "title" => Ok(TagKind::Title),
            "notice" => Ok(TagKind::Notice),
            "dev" => Ok(TagKind::Dev),
            "param" => Ok(TagKind::Param),
            "return" => Ok(TagKind::Return),
            "formula" => Ok(TagKind::Formula),
            _ => bail!("unrecognized tag: {s}"),
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
            Ast::GhostFunction { .. } | Ast::GhostMapping { .. } => "ghost",
            Ast::Methods { .. } => "methods",
            Ast::FreeFormComment { .. } => "freeform comment",
            Ast::Import { .. } => "import",
            Ast::Using { .. } => "using",
            Ast::UseRule { .. } | Ast::UseBuiltinRule { .. } | Ast::UseInvariant { .. } => "use",
            Ast::HookSload { .. }
            | Ast::HookSstore { .. }
            | Ast::HookCreate { .. }
            | Ast::HookOpcode { .. } => "hook",
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
            | Ast::GhostFunction { name, .. }
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
            Ast::Rule { block, .. }
            | Ast::Function { block, .. }
            | Ast::Methods { block }
            | Ast::HookSload { block, .. }
            | Ast::HookSstore { block, .. }
            | Ast::HookCreate { block, .. }
            | Ast::HookOpcode { block, .. } => Some(block.as_str()),

            Ast::Invariant { proof: block, .. }
            | Ast::GhostFunction { axioms: block, .. }
            | Ast::GhostMapping { axioms: block, .. } => block.as_ref().map(String::as_str),

            _ => None,
        }
    }

    pub fn returns(&self) -> Option<&str> {
        match self {
            Ast::Function { returns, .. } => returns.as_deref(),
            Ast::Definition { returns, .. } | Ast::GhostFunction { returns, .. } => {
                Some(returns.as_str())
            }
            _ => None,
        }
    }

    pub fn ty_list(&self) -> Option<&[String]> {
        match self {
            Ast::GhostFunction { ty_list, .. } => Some(ty_list),
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
