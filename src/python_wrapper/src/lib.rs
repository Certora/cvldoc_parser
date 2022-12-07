mod wrapper_structs;

use color_eyre::eyre::eyre;
use color_eyre::eyre::WrapErr;
use cvldoc_parser_core::parse::builder::Builder;
use itertools::Itertools;
use pyo3::prelude::*;

/// takes a list of file paths as strings, returns a list of parsed cvldocs for each path,
/// if any cvldocs were parsed for the path, otherwise returns an empty list for that path.
/// currently panics if a file fails to open, or fails to read.
#[pyfunction]
fn parse(paths: Vec<&str>) -> Vec<Vec<wrapper_structs::CvlElementPy>> {
    let elements_in_file = |file_path: &str| {
        let src = std::fs::read_to_string(file_path)
            .wrap_err_with(|| eyre!("file does not exist: {file_path}"))?;

        Builder::new(&src)
            .build()
            .map(|rust_elements| rust_elements.into_iter().map(Into::into).collect())
    };

    paths
        .into_iter()
        .map(elements_in_file)
        .try_collect()
        .unwrap() //TODO: figure out how to deal with errors here
}

#[pymodule]
fn cvldoc_parser(_py: Python, m: &PyModule) -> PyResult<()> {
    use wrapper_structs::*;

    m.add_class::<Documentation>()?;
    m.add_class::<FreeForm>()?;
    m.add_class::<AssociatedElement>()?;
    m.add_class::<DocumentationTag>()?;
    m.add_class::<Diagnostic>()?;
    m.add_class::<Severity>()?;
    m.add_class::<Position>()?;
    m.add_class::<Range>()?;

    m.add_function(wrap_pyfunction!(parse, m)?)?;
    Ok(())
}
