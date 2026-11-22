use cu::pre::*;

use crate::config::Config;

// REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/tools/list_symbols.rs#L13

/// Print symbols in the output ELF.
#[derive(Debug, clap::Parser)]
#[clap(bin_name = "listsym")]
pub struct Args {
    /// Show symbols that are undefined (e.g. unimplemented functions)
    #[clap(short = 'u', long)]
    pub show_undefined: bool,

    /// Show data symbols
    #[clap(short = 'd', long)]
    pub show_data: bool,

    /// Show symbols that are listed as decompiled in the file list
    #[clap(short = 'l', long)]
    pub show_decompiled: bool,

    /// Show only symbols containing the specified substring
    pub filter: Option<String>,

    /// Specify the version to use
    #[clap(long)]
    pub version: Option<String>,
}

/// Parse the args. If None is returned, the help or error message
/// has already been printed
pub fn parse_args(config: &Config, cli_args: &[String]) -> Option<Args> {
    // need to put a arg[0] as the placeholder for the binary
    let mut args: Args = cu::cli::try_parse(
        std::iter::once("listsym").chain(cli_args.iter().map(|x| x.as_str())),
        true,
    )?;

    if args.version.is_none() {
        args.version = config.repo.default_version.clone();
    }

    Some(args)
}
