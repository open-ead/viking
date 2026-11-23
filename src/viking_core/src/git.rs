use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

use cu::pre::*;

use crate::config::Config;
use crate::ui;

/// Read a file from a Git revision and decode it as UTF-8
///
/// The path must be within the repo and tracked by Git
#[inline(always)]
pub fn read_utf8_file_at_revision(
    config: &Config,
    path: impl AsRef<Path>,
    rev: &str,
) -> cu::Result<String> {
    read_utf8_file_at_revision_imp(config, path.as_ref(), rev)
}

fn read_utf8_file_at_revision_imp(config: &Config, path: &Path, rev: &str) -> cu::Result<String> {
    let bytes = read_file_at_revision_imp(config, path, rev)?;
    match String::from_utf8(bytes) {
        Ok(x) => Ok(x),
        Err(_) => {
            cu::bail!("file '{}' at revision {} is not UTF-8", path.display(), rev);
        }
    }
}

/// Read a file from a Git revision.
///
/// The path must be within the repo and tracked by Git
#[inline(always)]
pub fn read_file_at_revision(
    config: &Config,
    path: impl AsRef<Path>,
    rev: &str,
) -> cu::Result<Vec<u8>> {
    read_file_at_revision_imp(config, path.as_ref(), rev)
}

#[cu::context("failed to read file '{}' at revision {}", path.display(), rev)]
fn read_file_at_revision_imp(config: &Config, path: &Path, rev: &str) -> cu::Result<Vec<u8>> {
    // REFERENCE=https://github.com/LynxDev2/nx-decomp-tools/blob/917b6acc40d67a8e6b25f32f8b89121ebb0c6081/viking/src/repo.rs#L85-L86
    let abs_path = if path.is_absolute() {
        Cow::Borrowed(path)
    } else {
        Cow::Owned(path.normalize()?)
    };
    if !abs_path.starts_with(&config.repo.root_path) {
        cu::bail!(
            "cannot read file from git that is not in the repo: {}",
            path.display()
        );
    }
    let rel_path = abs_path.try_to_rel_from(&config.repo.root_path);
    let rel_path_str = rel_path.as_utf8()?;
    let output = Command::new("git")
        .current_dir(&config.repo.root_path)
        .arg("show")
        .arg(format!("{rev}:{rel_path_str}"))
        .output()?;
    if !output.status.success() {
        ui::error!("{}", String::from_utf8_lossy(&output.stderr));
        cu::bail!("git exited with {}", output.status);
    }

    Ok(output.stdout)
}
