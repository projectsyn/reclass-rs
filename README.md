# reclass-rs: A Reclass implementation in Rust

Reclass is a library which defines a syntax and directory structure for recursively merging YAML data sources.

This repository contains a Rust implementation of Reclass which is based on the [Reclass fork](https://github.com/kapicorp/reclass) maintained by [kapicorp](https://github.com/kapicorp/).
The Reclass implementation provided in this repository can be used both from other Rust programs and in Python programs.
The `reclass-rs` Python module is implemented directly in Rust with [PyO3](https://pyo3.rs/latest/).

Please note that this implementation doesn't yet support all the features and extensions which are availble in Kapitan Reclass.
However, for features which are implemented, we aim to be compatbile with Kapitan Reclass.

The implementation currently supports the following features of Kapicorp Reclass:

* The Reclass options `nodes_path` and `classes_path`
* The Reclass option `ignore_class_notfound`
* Escaped parameter references
* Merging referenced lists and dictionaries
* Constant parameters
* Nested references
* References in class names
* Loading classes with relative names

The following Kapicorp Reclass features aren't supported:

* Loading Reclass configuration options from `reclass-config.yaml`
* Ignoring overwritten missing references
* Inventory Queries
* The Reclass option `ignore_class_notfound_regexp`
* The Reclass option `componse_node_name`
* The Reclass option `allow_none_override` can't be set to `False`
* The Reclass `yaml_git` and `mixed` storage types
* Any Reclass option which is not mentioned explicitly here or above

Documentation for the original Reclass can be found at https://reclass.pantsfullofunix.net/.
Documentation on Reclass extensions introduced in the Kapicorp Reclass fork can be found at https://github.com/kapicorp/reclass/blob/develop/README-extensions.rst.

## Prerequisites

* Python >= 3.8
* Rust >= 1.56 (we recommend installing the latest stable toolchain with [rustup])

## Setup local development environment for Python bindings

1. Create a local virtualenv for running Python tests and install [maturin] and pytest

```
python -m venv .venv
source .venv/bin/activate
pip install maturin pytest
```

2. Build the reclass-rs Python library and install it in the virtualenv

```
maturin develop
```

3. Run Python tests

```
pytest
```

## Rust development

You should be able to run the Rust tests through Cargo if you have the Rust toolchain setup:

```
cargo test
```

### Linting and formatting

* Use `cargo fmt` to format code
* Use `cargo check` for checking that the code compiles
* Use `cargo clippy` to check for code issues

## Testing reclass-rs in Kapitan


If you're using [Kapitan], you can test reclass-rs by installing `reclass-rs` in your Kapitan virtualenv:

1. Build the `reclass-rs` wheel locally (this assumes that you've setup a local Python development environment, see above)

```
source .venv/bin/activate
maturin build --release
```

2. Install the wheel in your Kapitan virtualenv

```
KAPITAN_VENV=/path/to/your/kapitan/virtualenv
source ${KAPITAN_VENV}/bin/activate
# NOTE: make sure you use the same Python version as you use for the Kapitan virtualenv to build the wheel.
pip install target/wheels/reclass_rs-*
```

3. Patch the Kapitan package in the virtualenv with the following command

```
patch -p1 -d $KAPITAN_VENV < hack/kapitan_0.32_reclass_rs.patch
```

Please note that we've only tested the patch against the Kapitan 0.32 release as published on PyPI.


[rustup]: https://rustup.rs/
[maturin]: https://github.com/PyO3/maturin
[Kapitan]: https://kapitan.dev
