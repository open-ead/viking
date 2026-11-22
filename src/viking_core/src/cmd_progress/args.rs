use cu::pre::*;
use rustc_hash::FxHashSet;

use crate::config::Config;
use crate::fl;

/// Show decompilation progress
#[derive(Debug, clap::Parser)]
#[clap(bin_name = "progress")]
pub struct Args {
    /// Specify the version to use
    #[clap(long)]
    pub version: Option<String>,

    /// Filter the output by status
    ///
    /// The value can either be a full status name like `NotDecompiled`, or
    /// a string of status codes like `OMmUL`
    #[clap(short, long, value_name = "STATUS_SET", value_parser = fl::Status::parse_set, default_value = "OMm")]
    pub status: FxHashSet<fl::Status>,

    /// Show difference of the progress in the current worktree's file list against
    /// another Git revision; requires Git.
    ///
    /// Default revision is `HEAD` if not specified.
    #[clap(short, long, value_name = "REV", default_missing_value = "HEAD", num_args=0..=1)]
    pub diff: Option<String>,

    /// Show per-group progress
    #[clap(short, long, value_name = "GROUP")]
    pub group_by: Option<GroupType>,

    /// When diffing per-group, show unchanged groups
    #[clap(short, long, requires = "group_by", requires = "diff")]
    pub all: bool,

    /// Specify output format
    #[clap(short, long, default_value = "pretty")]
    pub format: Format,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum GroupType {
    /// Group by top namespace
    Namespace,
    /// Group by object as listed in the file list
    Object,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Format {
    /// Human-readable, color-coded format
    Pretty,
    /// JSON
    Json,
}

/// Parse the args. If None is returned, the help or error message
/// has already been printed
pub fn parse_args(config: &Config, cli_args: &[String]) -> Option<Args> {
    // need to put a arg[0] as the placeholder for the binary
    let mut args: Args = cu::cli::try_parse(
        std::iter::once("progress").chain(cli_args.iter().map(|x| x.as_str())),
        true,
    )?;

    if args.version.is_none() {
        args.version = config.repo.default_version.clone();
    }

    Some(args)
}
