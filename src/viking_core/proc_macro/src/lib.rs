use pm::pre::*;

/// A stub `pyclass` attribute to just remove all `#[pyo3(...)]` attributes
/// inside the item
///
/// https://github.com/PyO3/pyo3/issues/5125
#[proc_macro_attribute]
pub fn pyclass(_attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(py_stubs::remove_pyo3(input))
}

/// A stub `pyfunction` attribute to just remove all `#[pyo3(...)]` attributes
/// inside the item
///
/// https://github.com/PyO3/pyo3/issues/5125
#[proc_macro_attribute]
pub fn pyfunction(_attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(py_stubs::remove_pyo3(input))
}

/// A stub `pymethods` attribute to just remove all `#[pyo3(...)]` attributes
/// inside the item
///
/// https://github.com/PyO3/pyo3/issues/5125
#[proc_macro_attribute]
pub fn pymethods(_attr: TokenStream, input: TokenStream) -> TokenStream {
    pm::flatten(py_stubs::remove_pyo3(input))
}

mod py_stubs;

/// Generate the CLI entry point functions in Rust
#[proc_macro]
pub fn make_cli(input: TokenStream) -> TokenStream {
    pm::flatten(make_cli::expand_rust(input))
}

/// Generate the CLI entry point functions in Rust for Python
#[proc_macro]
pub fn make_python_cli(input: TokenStream) -> TokenStream {
    pm::flatten(make_cli::expand_python(input))
}
mod make_cli;
