use crate::config::Config;

use crate::cmd_progress::Args;

/// Entry point for the progress command
pub fn run(_config: &Config, args: Args) -> cu::Result<()> {
    // todo: implement the command here
    println!("{args:#?}");
    Ok(())
}
