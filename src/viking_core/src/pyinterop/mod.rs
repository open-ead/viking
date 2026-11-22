/// Pyo3 prelude import
#[doc(hidden)]
pub mod pre {
    #[cfg(feature = "python")]
    pub use pyo3::prelude::*;
    #[cfg(not(feature = "python"))]
    pub use viking_proc_macro::{pyclass, pyfunction, pymethods};
}

#[cfg(feature = "python")]
mod py_handle;
#[cfg(feature = "python")]
#[allow(unused)]
pub(crate) use py_handle::*;
#[cfg(feature = "python")]
mod py_result;
#[cfg(feature = "python")]
pub use py_result::*;
