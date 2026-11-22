use pm::pre::*;

/// Input should be the command identifier
///
/// Output is the functions in the cli module
///
/// For a command `foo`, this generates (all gated behind the `cmd-foo` feature):
/// - `foo()`: load config from default locations and parse args from the command line
/// - `foo_argv(args)`: load config from default locations and use the given args
/// - `foo_config_argv(config, args)`: use the given config and args
///
/// The command module `crate::cmd_foo` must provide:
/// - `parse_args(config: &Config, args: &[String]) -> Option<Args>`
/// - `run(config: &Config, args: Args) -> cu::Result<()>`
pub fn expand_rust(input: TokenStream) -> pm::Result<TokenStream2> {
    let name: syn::Ident = syn::parse(input)?;
    let name_str = name.to_string();

    let feature = format!("cmd-{name_str}");
    let module = pm::format_ident!("cmd_{}", name);
    let fn_argv = pm::format_ident!("{}_argv", name);
    let fn_config_argv = pm::format_ident!("{}_config_argv", name);

    let doc_main = format!(
        "Run the `{name_str}` command, loading config from default locations and parsing args from the command line"
    );
    let doc_argv = format!(
        " Run the `{name_str}` command with the given args, loading config from default locations"
    );
    let doc_config_argv = format!(" Run the `{name_str}` command with the given config and args");

    let expanded = pm::quote! {
        #[doc = #doc_main]
        #[cfg(feature = #feature)]
        pub fn #name() -> cu::Result<u32> {
            #fn_argv(std::env::args().skip(1).collect())
        }

        #[doc = #doc_argv]
        #[cfg(feature = #feature)]
        pub fn #fn_argv(args: Vec<String>) -> cu::Result<u32> {
            let config = crate::config::load_config()?;
            #fn_config_argv(&config, args)
        }

        #[doc = #doc_config_argv]
        #[cfg(feature = #feature)]
        pub fn #fn_config_argv(config: &crate::config::Config, args: Vec<String>) -> cu::Result<u32> {
            let args = crate::#module::parse_args(config, &args);
            match args {
                None => Ok(127),
                Some(args) => {
                    crate::#module::run(config, args)?;
                    Ok(0)
                }
            }
        }
    };

    Ok(expanded)
}

/// Input should be the command identifier
///
/// Output is the `_core_cmd_<name>` Python submodule (gated behind the `cmd-<name>` feature),
/// which has a `run(state, args) -> int` function that calls `crate::cli::<name>_config_argv`
///
/// Note that pyo3 does not expand macros inside a `#[pymodule] mod`, so this macro
/// must be invoked outside of the `_core` module, and the submodule needs to
/// be exported from `_core` with:
/// ```ignore
/// #[cfg(feature = "cmd-foo")]
/// #[pymodule_export]
/// use super::_core_cmd_foo;
/// ```
pub fn expand_python(input: TokenStream) -> pm::Result<TokenStream2> {
    let name: syn::Ident = syn::parse(input)?;
    let name_str = name.to_string();

    let feature = format!("cmd-{name_str}");
    let module = pm::format_ident!("_core_cmd_{}", name);
    let fn_config_argv = pm::format_ident!("{}_config_argv", name);

    let expanded = pm::quote! {
        #[cfg(feature = #feature)]
        #[::pyo3::pymodule]
        // these are what pyo3 adds to a #[pymodule] nested inside the _core module
        #[pyo3(module = "_core")]
        #[pyo3(submodule)]
        mod #module {
            use ::pyo3::prelude::*;

            #[pyfunction]
            fn run(state: crate::python::State, args: Vec<String>) -> PyResult<u32> {
                let result = crate::cli::#fn_config_argv(&state.config, args);
                crate::pyinterop::into_pyresult(result)
            }
        }
    };

    Ok(expanded)
}
