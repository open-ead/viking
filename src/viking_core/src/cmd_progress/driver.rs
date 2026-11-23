use std::path::PathBuf;

use cu::pre::*;

use crate::config::Config;
use crate::fl;
use crate::git;
use crate::ui;

use crate::cmd_progress::report::{Builder, Report};
use crate::cmd_progress::{Args, Format, GroupType};

/// Entry point for the progress command
pub fn run(config: &Config, args: Args) -> cu::Result<()> {
    let version = args.version.as_deref();

    // the diff is base_source minus diff_source
    // e.g. count = 5 in base_source and count = 3 in diff_source -> diff = 2
    let (base_source, diff_source) = resolve_file_list_sources(config, &args)?;

    // check csv: does not support --group-by object
    if args.group_by == Some(GroupType::Object) {
        match &base_source {
            FileListSource::CurrentWorktree => {
                if fl::is_csv(config, version) {
                    cu::bail!(
                        "--group-by object cannot be used with the legacy CSV format: file list in the current worktree"
                    );
                }
            }
            FileListSource::Revision(rev, p) => {
                if fl::is_csv_path(p) {
                    cu::bail!(
                        "--group-by object cannot be used with the legacy CSV format: file list in revision {rev}: {}",
                        p.display()
                    );
                }
            }
        }
        match &diff_source {
            None => {}
            Some(FileListSource::CurrentWorktree) => {
                if fl::is_csv(config, version) {
                    cu::bail!(
                        "--group-by object cannot be used with the legacy CSV format: file list in the current worktree"
                    );
                }
            }
            Some(FileListSource::Revision(rev, p)) => {
                if fl::is_csv_path(p) {
                    cu::bail!(
                        "--group-by object cannot be used with the legacy CSV format: file list in revision {rev}: {}",
                        p.display()
                    );
                }
            }
        }
    }

    let mut report = build_report(config, &args, &base_source)?;

    if let Some(diff_source) = diff_source {
        let diff_report = build_report(config, &args, &diff_source)?;
        report.diff(&diff_report)?;
    }

    match args.format {
        Format::Json => {
            let serialized = json::stringify_pretty(&report)?;
            println!("{serialized}");
        }
        Format::Pretty => display_pretty(&report, &args),
    }

    Ok(())
}

fn resolve_file_list_sources(
    config: &Config,
    args: &Args,
) -> cu::Result<(FileListSource, Option<FileListSource>)> {
    let Some(diff_revs) = &args.diff else {
        return Ok((FileListSource::CurrentWorktree, None));
    };
    match diff_revs.split_once("..") {
        None => {
            // diffing from rev to current worktree
            if args.diff_path.len() > 1 {
                cu::bail!("too many --diff-path: only one revision is specified");
            }
            let diff_from_path = resolve_diff_path(config, args, 0);
            Ok((
                FileListSource::CurrentWorktree,
                Some(FileListSource::Revision(
                    diff_revs.trim().to_string(),
                    diff_from_path,
                )),
            ))
        }
        Some((from_rev, to_rev)) => {
            let from_rev = from_rev.trim();
            let to_rev = to_rev.trim();
            if from_rev.is_empty() {
                cu::bail!("invalid revision: {from_rev}");
            }
            if to_rev.is_empty() {
                cu::bail!("invalid revision: {to_rev}");
            }
            if args.diff_path.len() > 2 {
                cu::bail!("too many --diff-path: max two revisions");
            }
            let from_path = resolve_diff_path(config, args, 0);
            let to_path = resolve_diff_path(config, args, 1);
            Ok((
                FileListSource::Revision(to_rev.trim().to_string(), to_path),
                Some(FileListSource::Revision(
                    from_rev.trim().to_string(),
                    from_path,
                )),
            ))
        }
    }
}

fn resolve_diff_path(config: &Config, args: &Args, arg_position: usize) -> PathBuf {
    match args.diff_path.get(arg_position) {
        None => {
            let version = args.version.as_deref();
            config.repo.file_list_with_version(version)
        }
        Some(p) => p.into(),
    }
}

enum FileListSource {
    CurrentWorktree,
    Revision(String, PathBuf),
}

fn build_report(config: &Config, args: &Args, source: &FileListSource) -> cu::Result<Report> {
    let file_list = match source {
        FileListSource::CurrentWorktree => fl::load_file_list(config, args.version.as_deref())?,
        FileListSource::Revision(rev, path) => {
            let content = cu::check!(
                git::read_utf8_file_at_revision(config, path, rev),
                "failed to read file list at revision {rev}"
            )?;
            if fl::is_csv_path(path) {
                fl::parse_file_list_csv(config, &content)?
            } else {
                fl::parse_file_list_yml(config, &content)?
            }
        }
    };
    let report = build_report_from_file_list(&file_list, args.group_by);
    Ok(report)
}

fn build_report_from_file_list(file_list: &fl::FileList, group_type: Option<GroupType>) -> Report {
    let mut builder = Builder::new(group_type);
    for (object_name, object) in file_list.iter() {
        for info in &object.text_section {
            builder.add_info(object_name, info.clone());
        }
    }
    builder.done()
}

fn display_pretty(report: &Report, args: &Args) {
    if report.grouped_by.is_some() {
        ui::print_warning("showing progress by group is not implemented yet");
    }

    let total = &report.overall.total;
    let mut overall_table = ui::Table::new(ui::RightTruncate(0));
    let overall_changed = report.overall.changed();
    if overall_changed {
        overall_table.set_right_justify([0, 2, 4, 5, 6, 7, 8, 10, 11]);
        overall_table.set_sep(["", " ", " ", " ", "  ", " ", " ", " "]);
    } else {
        overall_table.set_right_justify([0, 2, 3, 4]);
        overall_table.set_sep(["", " ", "  ", "  "]);
    }

    for status in fl::Status::all().iter() {
        if !args.status.contains(status) {
            continue;
        }
        let stats = report.overall.by_status.get(status).unwrap_or_default();
        let mut row = vec![];
        row.push(status.colored_description());
        row.push(":".into());

        if !overall_changed {
            row.push(stats.count.to_string().into());
            row.push(format!("({})", ui::Percent(stats.count, total.count)).into());
            row.push(format!("{}", ui::ByteFormat(stats.bytes)).into());
        } else if stats.changed() {
            let comparison = stats.comparison();
            row.extend(ui::tabular_diff_count_with_percentage(
                comparison.count,
                stats.count,
                total.count,
            ));
            row.extend(ui::tabular_diff_bytes(comparison.bytes, stats.bytes));
        } else {
            row.push("".into());
            row.push("".into());
            row.push(stats.count.to_string().into());
            row.push(format!("({})", ui::Percent(stats.count, total.count)).into());
            row.push("".into());
            row.push("".into());
            row.push("".into());
            row.push("".into());
            row.push(format!("{}", ui::ByteFormat(stats.bytes)).into());
            row.push("".into());
        }
        overall_table.add_row(row);
    }

    if !overall_table.is_empty() {
        overall_table.add_row([]);
    }

    let mut row = vec![];
    row.push("total functions".into());
    row.push(":".into());
    if !overall_changed {
        row.push(total.count.to_string().into());
        row.push("".into());
        row.push(format!("{}", ui::ByteFormat(total.bytes)).into());
    } else if total.changed() {
        let comparison = total.comparison();
        row.extend(ui::tabular_diff_count(comparison.count, total.count));
        row.push("".into());
        row.push("".into());
        row.extend(ui::tabular_diff_bytes(comparison.bytes, total.bytes));
    } else {
        row.push("".into());
        row.push("".into());
        row.push(total.count.to_string().into());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push("".into());
        row.push(format!("{}", ui::ByteFormat(total.bytes)).into());
        row.push("".into());
    }
    overall_table.add_row(row);

    println!("{overall_table}");
}
