pub mod diagnostics;
mod parse;
pub mod util;

use self::parse::parser;
use crate::util::span_to_range::{RangeConverter, Ranged};
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
    },

    MultiLineFreeForm {
        header: String,
        block: String,
    },

    Documentation {
        tags: Vec<DocumentationTag>,
        associated: Option<AssociatedElement>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssociatedElement {
    pub kind: DeclarationKind,
    pub name: String,
    pub params: Vec<(String, String)>,
    pub block: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclarationKind {
    Rule,
    Invariant,
    Function,
    Definition,
    Ghost,
    Methods,
}

impl Display for DeclarationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            DeclarationKind::Rule => "rule",
            DeclarationKind::Invariant => "invariant",
            DeclarationKind::Function => "function",
            DeclarationKind::Definition => "definition",
            DeclarationKind::Ghost => "ghost",
            DeclarationKind::Methods => "methods",
        };
        write!(f, "{kind}")
    }
}

impl TryFrom<&str> for DeclarationKind {
    type Error = color_eyre::Report;

    fn try_from(kw: &str) -> Result<Self, Self::Error> {
        use DeclarationKind::*;

        match kw {
            "rule" => Ok(Rule),
            "invariant" => Ok(Invariant),
            "function" => Ok(Function),
            "definition" => Ok(Definition),
            "ghost" => Ok(Ghost),
            "methods" => Ok(Methods),
            _ => bail!("unrecognized declaration keyword: {kw}"),
        }
    }
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

    pub fn from_rope(rope: Rope) -> Vec<Ranged<NatSpec>> {
        let src = rope.to_string();
        let converter = RangeConverter::new(rope);
        let (builders, _) = parser().parse_recovery(src.as_str());

        builders
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(builder, span)| {
                let natspec = builder.build_with_converter(converter.clone()).ok()?;
                let range = converter.to_range(span);
                Some((natspec, range))
            })
            .collect()
    }

    pub fn auto_generated_title(&self) -> Result<String, Report> {
        match self {
            NatSpec::Documentation { associated, .. } => associated
                .as_ref()
                .map(|element| element.name.clone())
                .ok_or_else(|| eyre!("documentation has no associated syntactic element")),
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
        matches!(
            self,
            NatSpec::Documentation {
                tags: _,
                associated: _
            }
        )
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
