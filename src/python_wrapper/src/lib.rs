mod wrapper_structs;

use cvldoc_parser_core::parse::builder::Builder;
use pyo3::exceptions::{PyFileNotFoundError, PyOSError, PyRuntimeError};
use pyo3::prelude::*;
use std::fs::read_to_string;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use wrapper_structs::{AstKindPy, AstPy, CvlElementPy, DocumentationTagPy, SpanPy, TagKindPy};

fn file_contents(path: &Path) -> PyResult<String> {
    read_to_string(path).map_err(|e| handle_io_error(path, e))
}

fn handle_io_error(path: &Path, e: std::io::Error) -> PyErr {
    let display = path.display();

    match e.kind() {
        ErrorKind::NotFound => {
            let desc = format!("file not found: {display}");
            PyFileNotFoundError::new_err(desc)
        }
        kind => {
            let desc = format!("got error while reading file {display}: {kind}");
            PyOSError::new_err(desc)
        }
    }
}

/// takes a path to a file a(s a string). returns a list of parsed cvldocs,
/// or an appropriate error in the case of a failure.
///
/// throws:
/// - `OSError` if file reading failed.
/// - `RuntimeError` if source code parsing failed.
#[pyfunction]
fn parse(py: Python, path: PathBuf) -> PyResult<Vec<CvlElementPy>> {
    let src = file_contents(path.as_path())?;

    let elements = Builder::new(&src).build().map_err(|_| {
        let display = path.display();
        let desc = format!("failed to parse source file: {display}");
        PyRuntimeError::new_err(desc)
    })?;

    elements
        .into_iter()
        .map(|cvl_element| CvlElementPy::new(py, cvl_element))
        .collect()
}

#[pymodule]
fn cvldoc_parser(_py: Python, module: &PyModule) -> PyResult<()> {
    module.add_class::<CvlElementPy>()?;
    module.add_class::<AstPy>()?;
    module.add_class::<AstKindPy>()?;
    module.add_class::<TagKindPy>()?;
    module.add_class::<SpanPy>()?;
    module.add_class::<DocumentationTagPy>()?;

    wrap_pyfunction!(parse, module).and_then(|function| module.add_function(function))?;

    Ok(())
}
