pub mod diagnostics;
mod parse;
pub mod util;

use self::parse::parser;
use crate::util::span_to_range::RangeConverter;
use chumsky::Parser;
use color_eyre::eyre::{bail, eyre, Report};
use lsp_types::Range;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatSpec {
    SingleLineFreeForm {
        header: String,
        range: Range,
    },

    MultiLineFreeForm {
        header: String,
        block: String,
        range: Range,
    },

    Documentation {
        tags: Vec<DocumentationTag>,
        associated: Option<AssociatedElement>,
        range: Range,
    },
}

pub type Ty = String;
pub type Param = (Ty, Option<String>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociatedElement {
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
        block: Option<String>,
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
        ty_list: Vec<Ty>,
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

impl NatSpec {
    pub fn tags(&self) -> Option<&[DocumentationTag]> {
        match self {
            NatSpec::Documentation { tags, .. } => Some(tags),
            _ => None,
        }
    }

    pub fn associated_element(&self) -> Option<&AssociatedElement> {
        match self {
            NatSpec::Documentation { associated, .. } => associated.as_ref(),
            _ => None,
        }
    }

    pub fn from_rope(rope: Rope) -> Vec<NatSpec> {
        let src = rope.to_string();
        let converter = RangeConverter::new(rope);
        let (builders, _) = parser().parse_recovery(src.as_str());

        builders
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(builder, _)| builder.build_with_converter(converter.clone()).ok())
            .collect()
    }

    pub fn auto_generated_title(&self) -> Result<String, Report> {
        match self {
            NatSpec::Documentation { associated, .. } => {
                let associated = associated
                    .as_ref()
                    .ok_or_else(|| eyre!("documentation has no associated syntactic element"))?;

                associated
                    .name()
                    .map(|name| name.to_string())
                    .ok_or_else(|| eyre!("element has no name"))
            }
            _ => bail!("free form comments have no associated syntactic element"),
        }
    }

    pub fn title(&self) -> Option<String> {
        match self.tags() {
            Some(tags) => {
                if let Some(title_tag) = tags.iter().find(|tag| tag.kind == Tag::Title) {
                    Some(title_tag.description.to_string())
                } else {
                    self.auto_generated_title().ok()
                }
            }
            _ => None,
        }
    }

    pub fn is_documentation(&self) -> bool {
        matches!(self, NatSpec::Documentation { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentationTag {
    pub kind: Tag,
    pub description: String,
    pub range: Option<Range>,
}

impl DocumentationTag {
    pub fn new(kind: Tag, description: String, range: Option<Range>) -> DocumentationTag {
        DocumentationTag {
            kind,
            description,
            range,
        }
    }

    pub fn param_name(&self) -> Option<&str> {
        match self.kind {
            Tag::Param => self
                .description
                .trim_start()
                .split_once(|c: char| c.is_ascii_whitespace())
                .map(|(param_name, _)| param_name),
            _ => None,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, Default, Serialize, Deserialize)]
pub enum Tag {
    Title,
    #[default]
    Notice, //if tag kind is not specified, it is considered @notice
    Dev,
    Param,
    Return,
    Formula,
    Unexpected(String),
}

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Tag::Title => "title",
            Tag::Notice => "notice",
            Tag::Dev => "dev",
            Tag::Param => "param",
            Tag::Return => "return",
            Tag::Formula => "formula",
            Tag::Unexpected(s) => s.as_str(),
        };
        write!(f, "{s}")
    }
}

impl Tag {
    pub fn unexpected_tag(&self) -> Option<&str> {
        match self {
            Tag::Unexpected(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

impl From<&str> for Tag {
    fn from(mut s: &str) -> Self {
        if let Some(trimmed) = s.strip_prefix('@') {
            s = trimmed;
        }
        match s {
            "title" => Tag::Title,
            "notice" => Tag::Notice,
            "dev" => Tag::Dev,
            "param" => Tag::Param,
            "return" => Tag::Return,
            "formula" => Tag::Formula,
            _ => Tag::Unexpected(s.to_string()),
        }
    }
}

impl Display for AssociatedElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            AssociatedElement::Rule { .. } => "rule",
            AssociatedElement::Invariant { .. } => "invariant",
            AssociatedElement::Function { .. } => "function",
            AssociatedElement::Definition { .. } => "definition",
            AssociatedElement::Ghost { .. } | AssociatedElement::GhostMapping { .. } => "ghost",
            AssociatedElement::Methods { .. } => "methods",
        };

        write!(f, "{kind}")
    }
}

impl AssociatedElement {
    pub fn name(&self) -> Option<&str> {
        match self {
            AssociatedElement::Rule { name, .. }
            | AssociatedElement::Invariant { name, .. }
            | AssociatedElement::Function { name, .. }
            | AssociatedElement::Definition { name, .. }
            | AssociatedElement::Ghost { name, .. }
            | AssociatedElement::GhostMapping { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    pub fn params(&self) -> Option<&[Param]> {
        match self {
            AssociatedElement::Rule { params, .. }
            | AssociatedElement::Invariant { params, .. }
            | AssociatedElement::Function { params, .. }
            | AssociatedElement::Definition { params, .. } => Some(params),
            _ => None,
        }
    }

    pub fn block(&self) -> Option<&str> {
        match self {
            AssociatedElement::Rule { block, .. }
            | AssociatedElement::Function { block, .. }
            | AssociatedElement::Methods { block } => Some(block.as_str()),

            AssociatedElement::Invariant { block, .. }
            | AssociatedElement::Ghost { block, .. }
            | AssociatedElement::GhostMapping { block, .. } => block.as_ref().map(String::as_str),

            AssociatedElement::Definition { .. } => None, //TODO: return definition?
        }
    }

    pub fn returns(&self) -> Option<&str> {
        match self {
            AssociatedElement::Function { returns, .. } => returns.as_ref().map(String::as_str),
            AssociatedElement::Definition { returns, .. }
            | AssociatedElement::Ghost { returns, .. } => Some(returns.as_str()),
            _ => None,
        }
    }

    pub fn ty_list(&self) -> Option<&[Ty]> {
        match self {
            AssociatedElement::Ghost { ty_list, .. } => Some(ty_list),
            _ => None,
        }
    }
}
