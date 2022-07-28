mod conversions;

use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use conversions::natspec_to_py_object;
use itertools::Itertools;
use pyo3::{prelude::*, types::PyList};
use ropey::Rope;
use std::{fs::File, io::Read};

#[derive(Debug, Clone)]
#[pyclass]
pub struct Documentation {
    #[pyo3(get)]
    pub tags: Vec<DocumentationTag>,
    #[pyo3(get)]
    pub associated: Option<AssociatedElement>,
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct FreeForm {
    #[pyo3(get)]
    pub header: String,
    #[pyo3(get)]
    pub block: Option<String>,
}

#[pymethods]
impl Documentation {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[pymethods]
impl FreeForm {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

fn natspecs_from_path(path: &str) -> Result<Vec<natspec_parser::NatSpec>> {
    let mut file = File::open(&path).wrap_err_with(|| format!("file does not exist: {path}"))?;

    let rope = {
        let mut data = String::new();
        file.read_to_string(&mut data)
            .wrap_err_with(|| format!("unable to read file: {path}"))?;

        Rope::from_str(&data)
    };

    let natspecs = natspec_parser::NatSpec::from_rope(rope)
        .into_iter()
        .map(|(natspec, _range)| natspec)
        .collect();

    Ok(natspecs)
}

#[derive(Debug, Clone)]
#[pyclass]
pub struct AssociatedElement {
    #[pyo3(get)]
    kind: String,
    #[pyo3(get)]
    name: String,
    #[pyo3(get)]
    params: Vec<(String, String)>,
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
    kind: String,
    #[pyo3(get)]
    description: String,
}

#[pymethods]
impl DocumentationTag {
    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

/// takes a list of file paths as strings, returns a list of parsed natspecs for each path,
/// if any natspecs were parsed for the path, otherwise returns an empty list for that path.
/// currently panics if a file fails to open, or fails to read.
#[pyfunction]
fn parse(paths: Vec<&str>) -> Vec<Py<PyAny>> {
    let natspecs_per_file: Vec<Vec<_>> = paths
        .into_iter()
        .map(natspecs_from_path)
        .try_collect()
        .unwrap(); //TODO: figure out how to deal with errors here

    Python::with_gil(|py| {
        natspecs_per_file
            .into_iter()
            .map(|file_natspecs| {
                let elements = file_natspecs
                    .into_iter()
                    .map(|natspec| natspec_to_py_object(natspec, py));

                PyList::new(py, elements).into_py(py)
            })
            .collect()
    })
}

#[pymodule]
fn natspec_parser(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
