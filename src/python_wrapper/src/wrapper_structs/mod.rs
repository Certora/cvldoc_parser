pub mod conversions;

use natspec_parser::{Param, Ty};
use pyo3::prelude::*;

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct Documentation {
    #[pyo3(get)]
    pub tags: Vec<DocumentationTag>,
    #[pyo3(get)]
    pub associated: Option<AssociatedElement>,
    #[pyo3(get)]
    pub range: Range,
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct Range {
    #[pyo3(get)]
    pub start: Position,
    #[pyo3(get)]
    pub end: Position,
}

#[pymethods]
impl Range {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct Position {
    #[pyo3(get)]
    pub line: u32,
    #[pyo3(get)]
    pub character: u32,
}

#[pymethods]
impl Position {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct Diagnostic {
    #[pyo3(get)]
    pub range: Range,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub severity: Severity,
}

#[pymethods]
impl Diagnostic {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct FreeForm {
    #[pyo3(get)]
    pub header: String,
    #[pyo3(get)]
    pub block: Option<String>,
    #[pyo3(get)]
    pub range: Range,
}

#[pymethods]
impl Documentation {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        let c: natspec_parser::NatSpec = self.clone().into();
        c.enumerate_diagnostics()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[pymethods]
impl FreeForm {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        let c: natspec_parser::NatSpec = self.clone().into();
        c.enumerate_diagnostics()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct AssociatedElement2 {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub params: Vec<Param>,
    #[pyo3(get)]
    pub block: Option<String>,
}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct AssociatedElement(natspec_parser::AssociatedElement);

#[pymethods]
impl AssociatedElement {
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
        self.0.block()
    }

    #[getter]
    pub fn returns(&self) -> Option<&str> {
        self.0.returns()
    }

    #[getter]
    pub fn ty_list(&self) -> Vec<Ty> {
        self.0.ty_list().map(Vec::from).unwrap_or_default()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

}

#[derive(Debug, Clone)]
#[pyclass(module = "natspec_parser")]
pub struct DocumentationTag {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub range: Option<Range>,
}

impl DocumentationTag {
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

#[pymethods]
impl DocumentationTag {
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
