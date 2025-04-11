# reclass-rs: A Reclass implementation in Rust

Reclass is a library which defines a syntax and directory structure for recursively merging YAML data sources.

This repository contains a Rust implementation of Reclass which is based on the [Reclass fork](https://github.com/kapicorp/reclass) maintained by [kapicorp](https://github.com/kapicorp/).
The Reclass implementation provided in this repository can be used both from other Rust programs and in Python programs.
The `reclass-rs` Python module is implemented directly in Rust with [PyO3](https://pyo3.rs/latest/).

Please note that this implementation doesn't yet support all the features and extensions which are available in Kapitan Reclass.
However, for features which are implemented, we aim to be compatible with Kapitan Reclass.

The implementation currently supports the following features of Kapicorp Reclass:

* The Reclass options `nodes_path` and `classes_path`
* The Reclass option `ignore_class_notfound`
* The Reclass option `ignore_class_notfound_regexp`
* Escaped parameter references
* Merging referenced lists and dictionaries
* Constant parameters
* Nested references
* References in class names
* Loading classes with relative names
* Loading Reclass configuration options from `reclass-config.yaml`
* The Reclass option `componse_node_name`
  * reclass-rs provides a non-compatible mode for `compose_node_name` which preserves literal dots in node names
* The Reclass options `class_mappings` and `class_mappings_match_path`
  * reclass-rs transparently rewrites backreferences in mapped classes to work with Rust's `regex` crate
  * Users are free to use either `\\1` (the Python variant) or `${1}` (the native Rust variant) when using backreferences in mapped classes
  * reclass-rs uses the `fancy-regex` crate for regex patterns in `class_mappings`.
    The `fancy-regex` crate should support most regex patterns supported by Python.

The following Kapicorp Reclass features aren't supported:

* Ignoring overwritten missing references
* Inventory Queries
* The Reclass option `allow_none_override` can't be set to `False`
* The Reclass `yaml_git` and `mixed` storage types
* Any Reclass option which is not mentioned explicitly here or above

Documentation for the original Reclass can be found at https://reclass.pantsfullofunix.net/.
Documentation on Reclass extensions introduced in the Kapicorp Reclass fork can be found at https://github.com/kapicorp/reclass/blob/develop/README-extensions.rst.

## Prerequisites

* Python >= 3.9
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

### Benchmarks

You can run benchmarks for `Reclass::render_inventory()` with

```
cargo bench -F bench
```

The benchmarks are implemented with `criterion`.

## Testing reclass-rs in Kapitan


If you're using [Kapitan], you can use reclass-rs by installing Kapitan with the optional `reclass-rs` dependency and specifying `--inventory-backend=reclass-rs` when running Kapitan.

```
KAPITAN_VENV=/path/to/your/kapitan/virtualenv
source ${KAPITAN_VENV}/bin/activate
pip install kapitan[reclass-rs]
```

See the [upstream Kapitan docs](https://kapitan.dev/pages/inventory/reclass-rs/) for more details.

## Automated package version management

We generate the package version of `reclass-rs` from the latest Git tag when building Python wheels.
To ensure this always works, we keep the version in the committed `Cargo.toml` as `0.0.0`.

We generate the package version from Git by calling `git describe --tags --always --match=v*`.
This command produces something like `v0.1.1-61-g531ca91`.
We always strip the leading `v`, since neither Cargo nor maturin support versions with leading `v`.
If we're building a branch or PR, we discard the component derived from the commit hash.
For the example output above, the package version for a branch or PR build will become `0.1.1.post61`.
For tag builds, the command ouptut will be just the tag, so the package version will match the tag.

The version is injected with [cargo-edit]'s `cargo set-version` before the Python wheels are built.

See the ["Python" workflow](./.github/workflows/python.yml) for more details.

[rustup]: https://rustup.rs/
[maturin]: https://github.com/PyO3/maturin
[Kapitan]: https://kapitan.dev
[cargo-edit]: https://github.com/killercup/cargo-edit
