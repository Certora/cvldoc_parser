pub mod conversions;

use cvldoc_parser_core::{Param, Ty};
use derivative::Derivative;
use pyo3::prelude::*;

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[pyclass(module = "cvldoc_parser")]
pub struct Documentation {
    #[pyo3(get)]
    #[derivative(Debug = "ignore")]
    pub raw: String,
    #[pyo3(get)]
    #[derivative(Debug = "ignore")]
    pub range: Range,
    #[pyo3(get)]
    pub tags: Vec<DocumentationTag>,
    #[pyo3(get)]
    pub associated: Option<AssociatedElement>,
}

#[derive(Debug, Clone)]
#[pyclass(module = "cvldoc_parser")]
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
#[pyclass(module = "cvldoc_parser")]
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

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[pyclass(module = "cvldoc_parser")]
pub struct Diagnostic {
    #[derivative(Debug = "ignore")]
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
#[pyclass(module = "cvldoc_parser")]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[pyclass(module = "cvldoc_parser")]
pub struct FreeForm {
    #[derivative(Debug = "ignore")]
    #[pyo3(get)]
    pub raw: String,
    #[derivative(Debug = "ignore")]
    #[pyo3(get)]
    pub range: Range,
    #[pyo3(get)]
    pub text: String,
}

#[pymethods]
impl Documentation {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    fn diagnostics(&self) -> Vec<Diagnostic> {
        let c: cvldoc_parser_core::CvlDoc = self.clone().into();
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
        let c: cvldoc_parser_core::CvlDoc = self.clone().into();
        c.enumerate_diagnostics()
            .into_iter()
            .map(Into::into)
            .collect()
    }
}

#[derive(Debug, Clone)]
#[pyclass(module = "cvldoc_parser")]
pub struct AssociatedElement(cvldoc_parser_core::AssociatedElement);

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

    #[getter]
    pub fn filters(&self) -> Option<&str> {
        self.0.filters()
    }

    #[getter]
    pub fn invariant(&self) -> Option<&str> {
        self.0.invariant()
    }

    #[getter]
    pub fn mapping(&self) -> Option<&str> {
        self.0.mapping()
    }

    #[getter]
    pub fn definition(&self) -> Option<&str> {
        self.0.definition()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[pyclass(module = "cvldoc_parser")]
pub struct DocumentationTag {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub description: String,
    #[derivative(Debug = "ignore")]
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
