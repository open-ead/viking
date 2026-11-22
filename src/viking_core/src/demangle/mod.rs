use std::borrow::Cow;

use cpp_demangle::Symbol;

use crate::pyinterop::pre::*;

mod top_namespace;
pub use top_namespace::*;

/// Demangle a C++ symbol.
#[inline(always)]
pub fn demangle_str(name: &str) -> cu::Result<String> {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/functions.rs#L247
    if !name.starts_with("_Z") {
        cu::bail!("not an external mangled name");
    }

    let symbol = Symbol::new(name)?;
    Ok(symbol.demangle(&Default::default())?)
}

/// Try to demangle the name, or return the name unchanged if it cannot be demangled
#[pyfunction]
#[inline(always)]
pub fn demangle_or_as_is(name: &str) -> Cow<'_, str> {
    match demangle_str(name) {
        Ok(x) => x.into(),
        Err(_) => name.into(),
    }
}

/// Check if the function name contains `query` either as-is or when demangled.
///
/// Returns `true` if:
///
/// - `function_name` contains `query`, or
/// - `demangle(function_name)` contains `query`
#[pyfunction]
pub fn raw_or_demangled_contains(function_name: &str, query: &str) -> bool {
    function_name.contains(query) || demangle_str(function_name).is_ok_and(|x| x.contains(query))
}
