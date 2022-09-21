mod wrapper_structs;

use ::cvldoc_parser::CvlDoc;
use color_eyre::eyre::WrapErr;
use color_eyre::Result;
use itertools::Itertools;
use pyo3::{prelude::*, types::PyList};
use ropey::Rope;
use std::{fs::File, io::Read};
use wrapper_structs::conversions::cvldoc_to_py_object;

fn cvldocs_from_path(path: &str) -> Result<Vec<CvlDoc>> {
    let mut file = File::open(&path).wrap_err_with(|| format!("file does not exist: {path}"))?;

    let rope = {
        let mut data = String::new();
        file.read_to_string(&mut data)
            .wrap_err_with(|| format!("unable to read file: {path}"))?;

        Rope::from_str(&data)
    };

    let natspecs = CvlDoc::from_rope(rope);

    Ok(natspecs)
}

/// takes a list of file paths as strings, returns a list of parsed cvldocs for each path,
/// if any natspecs were parsed for the path, otherwise returns an empty list for that path.
/// currently panics if a file fails to open, or fails to read.
#[pyfunction]
fn parse(paths: Vec<&str>) -> Vec<Py<PyAny>> {
    let cvldocs_per_file: Vec<Vec<_>> = paths
        .into_iter()
        .map(cvldocs_from_path)
        .try_collect()
        .unwrap(); //TODO: figure out how to deal with errors here

    Python::with_gil(|py| {
        cvldocs_per_file
            .into_iter()
            .map(|file_natspecs| {
                let elements = file_natspecs
                    .into_iter()
                    .map(|natspec| cvldoc_to_py_object(natspec, py));

                PyList::new(py, elements).into_py(py)
            })
            .collect()
    })
}

#[pymodule]
fn cvldoc_parser(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
