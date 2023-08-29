# reclass-rs: A Reclass implementation in Rust

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


[rustup]: https://rustup.rs/
[maturin]: https://github.com/PyO3/maturin
