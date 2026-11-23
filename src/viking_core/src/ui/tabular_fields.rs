use colored::{ColoredString, Colorize};

use crate::ui;

pub fn tabular_diff_count(from: u32, to: u32) -> Vec<ColoredString> {
    let diff_column = match from.cmp(&to) {
        std::cmp::Ordering::Less => {
            let diff = to - from;
            format!("+{diff}").green()
        }
        std::cmp::Ordering::Equal => "".into(),
        std::cmp::Ordering::Greater => {
            let diff = from - to;
            format!("-{diff}").red()
        }
    };
    vec![
        from.to_string().dimmed(),
        "->".dimmed(),
        to.to_string().into(),
        diff_column,
    ]
}

pub fn tabular_diff_count_with_percentage(from: u32, to: u32, total: u32) -> Vec<ColoredString> {
    let (diff, diff_percentage) = match from.cmp(&to) {
        std::cmp::Ordering::Less => {
            let diff = to - from;
            (
                format!("+{diff}").green(),
                format!("({})", ui::Percent(diff, total)).green(),
            )
        }
        std::cmp::Ordering::Equal => ("".into(), "".into()),
        std::cmp::Ordering::Greater => {
            let diff = from - to;
            (
                format!("-{diff}").red(),
                format!("({})", ui::Percent(diff, total)).red(),
            )
        }
    };
    vec![
        from.to_string().dimmed(),
        "->".dimmed(),
        to.to_string().into(),
        format!("({})", ui::Percent(to, total)).into(),
        diff,
        diff_percentage,
    ]
}

pub fn tabular_diff_bytes(from: u32, to: u32) -> Vec<ColoredString> {
    let diff_column = match from.cmp(&to) {
        std::cmp::Ordering::Less => {
            let diff = to - from;
            format!("+{}", ui::ByteFormat(diff)).green()
        }
        std::cmp::Ordering::Equal => "".into(),
        std::cmp::Ordering::Greater => {
            let diff = from - to;
            format!("-{}", ui::ByteFormat(diff)).red()
        }
    };
    vec![
        ui::ByteFormat(from).to_string().dimmed(),
        "->".dimmed(),
        ui::ByteFormat(to).to_string().into(),
        diff_column,
    ]
}
