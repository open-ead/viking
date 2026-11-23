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

    /// Filter the output by status (only affects --format=pretty)
    ///
    /// The value can either be a full status name like `NotDecompiled`, or
    /// a string of status codes like `OMmUL`
    #[clap(short, long, value_name = "STATUS_SET", value_parser = fl::Status::parse_set, default_value = "OMm")]
    pub status: FxHashSet<fl::Status>,

    /// Show difference of the progress (requires Git)
    ///
    /// If a single revision is specified, the current worktree's file list against
    /// that revision. If `from..to` is specified, then the file list in the `to`
    /// revision is diffed against that in the `from` revision. Default revision is
    /// `HEAD` if not specified.
    ///
    /// If the path or format of file list in a revision is different from that
    /// of the current worktree, use `diff_path` to specify the path of the file
    /// list in that revision.
    #[clap(short, long, value_name = "REV", default_missing_value = "HEAD", num_args=0..=1)]
    pub diff: Option<String>,

    /// Path of the file list in the revision being diffed. Only needed if the file list
    /// path or format has been changed.
    ///
    /// For multiple revisions, use `--diff from..to --diff-path from_path --diff-path to_path`
    #[clap(long, requires = "diff")]
    pub diff_path: Vec<String>,

    /// Show per-group progress
    #[clap(short, long, value_name = "GROUP")]
    pub group_by: Option<GroupType>,

    /// When diffing per-group, show unchanged groups (only affects --format=pretty)
    #[clap(short, long, requires = "group_by", requires = "diff")]
    pub all: bool,

    /// Specify output format
    #[clap(short, long, default_value = "pretty")]
    pub format: Format,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
pub enum GroupType {
    /// Group by top namespace
    Namespace,
    /// Group by object as listed in the file list
    Object,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
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
