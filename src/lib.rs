#![deny(clippy::suspicious)]
#![warn(clippy::single_match_else)]
#![warn(clippy::explicit_into_iter_loop)]
#![warn(clippy::semicolon_if_nothing_returned)]
#![warn(clippy::redundant_closure_for_method_calls)]
#![warn(let_underscore_drop)]

mod list;
mod refs;

use pyo3::prelude::*;

/// Reclass allows configuring various library behaviors
#[pyclass]
#[derive(Default)]
pub struct Reclass {
    // TODO(sg): add config options
}

#[pymethods]
impl Reclass {
    #[new]
    pub fn new() -> Self {
        Self {}
    }
}

#[pymodule]
fn reclass_rs(_py: Python, m: &PyModule) -> PyResult<()> {
    // Register the top-level `Reclass` Python class which is used to configure the library
    m.add_class::<Reclass>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reclass_new() {
        let _ = Reclass::new();
        let _ = Reclass::default();
    }
}
