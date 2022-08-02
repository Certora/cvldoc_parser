pub mod conversions;

use pyo3::prelude::*;

#[derive(Debug, Clone)]
#[pyclass]
pub struct Documentation {
    #[pyo3(get)]
    pub tags: Vec<DocumentationTag>,
    #[pyo3(get)]
    pub associated: Option<AssociatedElement>,
    #[pyo3(get)]
    pub range: Range,
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct Diagnostic {
    pub range: Range,
    pub description: String,
    pub severity: Severity,
}

#[pymethods]
impl Diagnostic {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone)]
#[pyclass]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone)]
#[pyclass]
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
#[pyclass]
pub struct AssociatedElement {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub params: Vec<(String, String)>,
    #[pyo3(get)]
    pub block: String,
}

#[pymethods]
impl AssociatedElement {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct DocumentationTag {
    #[pyo3(get)]
    pub kind: String,
    #[pyo3(get)]
    pub description: String,
    #[pyo3(get)]
    pub range: Option<Range>,
}

#[pymethods]
impl DocumentationTag {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}
