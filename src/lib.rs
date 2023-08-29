use pyo3::prelude::*;

/// Reclass allows configuring various library behaviors
#[pyclass]
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
    fn test_reclass() {
        let _ = Reclass {};
    }
}
