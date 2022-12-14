pub mod conversions;

use cvldoc_parser_core::util::{ByteSpan, Span};
use cvldoc_parser_core::{Ast, Param};
use pyo3::prelude::*;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug, Clone)]
#[pyclass(name = "Ast")]
pub struct AstPy(Ast);

#[derive(Clone)]
#[pyclass(name = "CvlElement")]
pub struct CvlElementPy {
    #[pyo3(get)]
    pub doc: Vec<DocumentationTagPy>,
    #[pyo3(get)]
    pub ast: AstPy,
    element_span: Span,
    doc_span: Option<Span>,
    src: Arc<str>,
}

impl Debug for CvlElementPy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CvlElement")
            .field("doc", &self.doc)
            .field("ast", &self.ast)
            .finish()
    }
}

#[derive(Debug, Clone)]
#[pyclass(name = "Span")]
pub struct SpanPy {
    pub start: usize,
    pub end: usize,
}

#[pymethods]
impl SpanPy {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

impl From<SpanPy> for Span {
    fn from(span_py: SpanPy) -> Span {
        let SpanPy { start, end } = span_py;
        start..end
    }
}

#[pymethods]
impl CvlElementPy {
    pub fn span(&self) -> SpanPy {
        let start = if let Some(doc_span) = &self.doc_span {
            doc_span.start
        } else {
            self.element_span.start
        };

        let end = self.element_span.end;

        SpanPy { start, end }
    }

    pub fn raw(&self) -> &str {
        Span::from(self.span()).byte_slice(&self.src).unwrap()
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[pymethods]
impl AstPy {
    #[getter]
    pub fn kind(&self) -> String {
        self.0.to_string()
    }

    #[getter]
    pub fn name(&self) -> Option<&str> {
        self.0.name()
    }

    #[getter]
    pub fn params(&self) -> Vec<Param> {
        self.0.params().map(Vec::from).unwrap_or_default()
    }

    #[getter]
    pub fn block(&self) -> Option<&str> {
        match &self.0 {
            Ast::Rule { block, .. } | Ast::Function { block, .. } | Ast::Methods { block } => {
                Some(block.as_str())
            }

            Ast::Invariant { proof: block, .. }
            | Ast::Ghost { axioms: block, .. }
            | Ast::GhostMapping { axioms: block, .. } => block.as_ref().map(String::as_str),

            Ast::Definition { .. } => None,
            _ => None,
        }
    }

    #[getter]
    pub fn returns(&self) -> Option<&str> {
        match &self.0 {
            Ast::Function { returns, .. } => returns.as_ref().map(String::as_str),
            Ast::Definition { returns, .. } | Ast::Ghost { returns, .. } => Some(returns.as_str()),
            _ => None,
        }
    }

    #[getter]
    pub fn ty_list(&self) -> Option<Vec<String>> {
        match &self.0 {
            Ast::Ghost { ty_list, .. } => Some(ty_list.clone()),
            _ => None,
        }
    }

    #[getter]
    pub fn filters(&self) -> Option<&str> {
        match &self.0 {
            Ast::Rule { filters, .. } | Ast::Invariant { filters, .. } => {
                filters.as_ref().map(String::as_str)
            }
            _ => None,
        }
    }

    #[getter]
    pub fn invariant(&self) -> Option<&str> {
        match &self.0 {
            Ast::Invariant { invariant, .. } => Some(invariant.as_str()),
            _ => None,
        }
    }

    #[getter]
    pub fn mapping(&self) -> Option<&str> {
        match &self.0 {
            Ast::GhostMapping { mapping, .. } => Some(mapping.as_str()),
            _ => None,
        }
    }

    #[getter]
    pub fn definition(&self) -> Option<&str> {
        match &self.0 {
            Ast::Definition { definition, .. } => Some(definition.as_str()),
            _ => None,
        }
    }

    #[getter]
    pub fn axioms(&self) -> Option<&str> {
        match &self.0 {
            Ast::Ghost { axioms, .. } | Ast::GhostMapping { axioms, .. } => {
                axioms.as_ref().map(String::as_str)
            }
            _ => None,
        }
    }

    #[getter]
    pub fn text(&self) -> Option<&str> {
        match &self.0 {
            Ast::FreeFormComment { text } => Some(text.as_str()),
            _ => None,
        }
    }

    #[getter]
    pub fn proof(&self) -> Option<&str> {
        match &self.0 {
            Ast::Invariant { proof, .. } => proof.as_ref().map(String::as_str),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

#[derive(Clone)]
#[allow(unused)]
#[pyclass(name = "DocumentationTag")]
pub struct DocumentationTagPy {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub description: String,
    span: SpanPy,
}

impl DocumentationTagPy {
    fn param_name_and_description(&self) -> Option<(&str, &str)> {
        match self.kind.as_str() {
            "param" => {
                let description = self.description.trim_start();

                description
                    .split_once(|c: char| c.is_ascii_whitespace())
                    .map(|(param_name, tail)| (param_name, tail.trim_start()))
            }
            _ => None,
        }
    }
}

impl Debug for DocumentationTagPy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DocumentationTag")
            .field("kind", &self.kind)
            .field("description", &self.description)
            .finish()
    }
}

#[pymethods]
impl DocumentationTagPy {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    pub fn param_name(&self) -> Option<&str> {
        self.param_name_and_description().map(|(name, _)| name)
    }

    pub fn param_description(&self) -> Option<&str> {
        self.param_name_and_description().map(|(_, desc)| desc)
    }
}

// #[derive(Debug, Clone)]
// #[pyclass(subclass)]
// pub struct AstBase;

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct FreeFormComment {
//     text: String,
// }

// #[derive(Debug, Clone)]
// #[pyclass]
// pub struct Rule {
//     name: String,
//     params: Vec<Param>,
//     filters: Option<String>,
//     block: String,
// }

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct Invariant {
//     name: String,
//     params: Vec<Param>,
//     invariant: String,
//     filters: Option<String>,
//     proof: Option<String>,
// }

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct Function {
//     name: String,
//     params: Vec<Param>,
//     returns: Option<String>,
//     block: String,
// }

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct Definition {
//     name: String,
//     params: Vec<Param>,
//     returns: String,
//     definition: String,
// }

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct Ghost {
//     name: String,
//     ty_list: Vec<String>,
//     returns: String,
//     axioms: Option<String>,
// }

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct GhostMapping {
//     name: String,
//     mapping: String,
//     axioms: Option<String>,
// }

// #[derive(Debug, Clone)]
// #[pyclass(extends = AstBase)]
// pub struct Methods {
//     block: String,
// }
