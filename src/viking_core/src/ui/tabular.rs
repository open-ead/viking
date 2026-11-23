use std::io::IsTerminal;

use colored::ColoredString;

/// A table format
pub struct Table<T: Truncate> {
    sep: Vec<ColoredString>,
    right_justify_columns: Vec<usize>,
    rows: Vec<Vec<ColoredString>>,
    truncate: T,
}
impl<T: Truncate> Table<T> {
    /// Create an empty table with the truncation strategy, separated by a single space
    pub fn new(truncate: T) -> Self {
        Self {
            sep: vec![],
            right_justify_columns: vec![],
            rows: vec![],
            truncate,
        }
    }
    /// Set the separators between columns. `sep[i]` is printed after column `i`.
    /// Columns without a separator in the list are followed by a single space.
    /// No separator is printed after the last column
    pub fn set_sep<S: Into<ColoredString>>(
        &mut self,
        sep: impl IntoIterator<Item = S>,
    ) -> &mut Self {
        self.sep = sep.into_iter().map(Into::into).collect();
        self
    }
    /// Get the separator printed after column i
    fn sep_at(&self, i: usize) -> &str {
        match self.sep.get(i) {
            Some(s) => s,
            None => " ",
        }
    }
    /// Total width of the separators between num_columns columns
    fn sep_width(&self, num_columns: usize) -> usize {
        (0..num_columns.saturating_sub(1))
            .map(|i| self.sep_at(i).len())
            .sum()
    }
    /// Set which columns (0-indexed) are right-justified. Replaces the previous setting
    pub fn set_right_justify(&mut self, columns: impl IntoIterator<Item = usize>) -> &mut Self {
        self.right_justify_columns = columns.into_iter().collect();
        self
    }

    /// Add a new row with the given fields
    pub fn add_row(&mut self, fields: impl IntoIterator<Item = ColoredString>) {
        self.rows.push(fields.into_iter().collect())
    }

    pub fn rows(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn columns(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or_default()
    }
    pub fn width(&self) -> u16 {
        let width: u32 = self.column_widths().iter().map(|x| *x as u32).sum();
        width.min(u16::MAX as u32) as u16
    }
    /// Get the width of each column
    pub fn column_widths(&self) -> Vec<u16> {
        let mut widths = vec![];
        for row in &self.rows {
            for (i, value) in row.iter().enumerate() {
                while widths.len() < i {
                    widths.push(0);
                }
                let w = value.len().min(u16::MAX as usize) as u16;
                if widths.len() == i {
                    widths.push(w);
                } else {
                    widths[i] = widths[i].max(w);
                }
            }
        }
        widths
    }
    fn print_row(
        &self,
        f: &mut std::fmt::Formatter,
        columns: &[ColoredString],
        column_widths: &[u16],
        right_justify: &[bool],
        is_last: bool,
    ) -> std::fmt::Result {
        let iter = columns
            .iter()
            .zip(column_widths)
            .zip(right_justify)
            .map(|((a, b), c)| (a, *b as usize, *c));
        for (i, (s, w, rj)) in iter.enumerate() {
            if i > 0 {
                match self.sep.get(i - 1) {
                    Some(sep) => write!(f, "{sep}")?,
                    None => write!(f, " ")?,
                }
            }
            // note that .len() on colored string return the bytes without the formatting
            if s.len() > w {
                let truncated = self.truncate.truncate(s, w as u16);
                let mut recolored = ColoredString::from(truncated);
                recolored.fgcolor = s.fgcolor;
                recolored.bgcolor = s.bgcolor;
                recolored.style = s.style;
                write_cell(f, &recolored, w, rj)?;
            } else {
                write_cell(f, s, w, rj)?;
            }
        }
        if is_last {
            write!(f, "")?;
        } else {
            writeln!(f)?;
        }
        Ok(())
    }
}

fn write_cell(
    f: &mut std::fmt::Formatter,
    s: &ColoredString,
    w: usize,
    rj: bool,
) -> std::fmt::Result {
    if rj {
        write!(f, "{s:>w$}")
    } else {
        write!(f, "{s:<w$}")
    }
}

impl<T: Truncate> std::fmt::Display for Table<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num_columns = self.columns();
        if self.width() == 0 || num_columns == 0 {
            return Ok(()); // empty table
        }
        let mut column_widths = self.column_widths();
        let mut right_justify = vec![false; num_columns];
        for i in &self.right_justify_columns {
            if let Some(rj) = right_justify.get_mut(*i) {
                *rj = true;
            }
        }
        let terminal_width = if std::io::stdout().is_terminal() {
            crossterm::terminal::size().ok().map(|(c, _)| c)
        } else {
            None // not a terminal
        };
        if let Some(terminal_width) = terminal_width {
            let sep_width = self.sep_width(num_columns).min(u16::MAX as usize) as u16;
            let content_width: u32 = column_widths.iter().map(|x| *x as u32).sum();
            let available_width = terminal_width.saturating_sub(sep_width);
            if content_width > available_width as u32 {
                self.truncate
                    .calc_widths(&mut column_widths, available_width);
            }
        }
        let len = self.rows();
        for (i, row) in self.rows.iter().enumerate() {
            self.print_row(f, row, &column_widths, &right_justify, i + 1 == len)?;
        }
        Ok(())
    }
}

pub trait Truncate {
    /// Calculate truncated column widths. The result array must sum up to total_width
    fn calc_widths(&self, columns: &mut [u16], total_width: u16);
    /// Truncate one column to the width
    fn truncate(&self, value: &str, width: u16) -> String;
}

// note since the tools only support English for now, we assume byte length = char length for
// display

/// Shrink each column evenly, truncating the left
pub struct LeftEvenTruncate;
impl Truncate for LeftEvenTruncate {
    fn calc_widths(&self, columns: &mut [u16], total_width: u16) {
        evenly_shrink(columns, total_width)
    }
    fn truncate(&self, value: &str, width: u16) -> String {
        truncate_imp(value, width, true)
    }
}
/// Shrink each column evenly, truncating the right
pub struct RightEvenTruncate;
impl Truncate for RightEvenTruncate {
    fn calc_widths(&self, columns: &mut [u16], total_width: u16) {
        evenly_shrink(columns, total_width)
    }
    fn truncate(&self, value: &str, width: u16) -> String {
        truncate_imp(value, width, false)
    }
}

/// Shrink the columns so they sum up to total_width, by capping the widest
/// columns at the same width (columns already narrower than the cap are untouched).
fn evenly_shrink(columns: &mut [u16], total_width: u16) {
    let sum: u32 = columns.iter().map(|x| *x as u32).sum();
    if sum <= total_width as u32 {
        return;
    }
    let mut sorted = columns.to_vec();
    sorted.sort_unstable();
    // find the cap: columns narrower than the cap keep their width,
    // the rest share the remaining budget evenly
    let mut budget = total_width as u32;
    let mut cap = 0u32;
    let mut extra = 0u32;
    for (i, w) in sorted.iter().enumerate() {
        let w = *w as u32;
        let remaining = (sorted.len() - i) as u32;
        if w * remaining <= budget {
            budget -= w;
        } else {
            cap = budget / remaining;
            extra = budget % remaining;
            break;
        }
    }
    // capped columns have width > cap, so cap + 1 never grows a column
    for w in columns.iter_mut() {
        if *w as u32 > cap {
            if extra > 0 {
                *w = (cap + 1) as u16;
                extra -= 1;
            } else {
                *w = cap as u16;
            }
        }
    }
}

/// left-truncate column X
pub struct LeftTruncate(pub usize);
impl Truncate for LeftTruncate {
    fn calc_widths(&self, columns: &mut [u16], total_width: u16) {
        shrink_at_column(columns, total_width, self.0)
    }

    fn truncate(&self, value: &str, width: u16) -> String {
        truncate_imp(value, width, true)
    }
}
/// right-truncate column X
pub struct RightTruncate(pub usize);
impl Truncate for RightTruncate {
    fn calc_widths(&self, columns: &mut [u16], total_width: u16) {
        shrink_at_column(columns, total_width, self.0)
    }

    fn truncate(&self, value: &str, width: u16) -> String {
        truncate_imp(value, width, false)
    }
}

/// Shrink only the column at `at` so the columns sum up to width. If that column
/// alone cannot absorb the difference, fall back to shrinking all columns evenly
fn shrink_at_column(columns: &mut [u16], width: u16, at: usize) {
    let sum: u32 = columns.iter().map(|x| *x as u32).sum();
    if sum <= width as u32 {
        return;
    }
    let excess = sum - width as u32;
    match columns.get_mut(at) {
        Some(w) if *w as u32 > excess => *w -= excess as u16,
        _ => evenly_shrink(columns, width),
    }
}

// truncate the string + put ellipsis, if width < 4, then fill with `.`
fn truncate_imp(value: &str, width: u16, left: bool) -> String {
    // use chars() so we don't panic, even we calculate based on ascii
    let width = width as usize;
    let len = value.chars().count();
    if len <= width {
        return value.to_string();
    }
    if width < 4 {
        return ".".repeat(width);
    }
    let keep = width - 3;
    if left {
        let rest: String = value.chars().skip(len - keep).collect();
        format!("...{rest}")
    } else {
        let rest: String = value.chars().take(keep).collect();
        format!("{rest}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_imp() {
        assert_eq!(truncate_imp("hello", 5, true), "hello");
        assert_eq!(truncate_imp("hello world", 8, true), "...world");
        assert_eq!(truncate_imp("hello world", 8, false), "hello...");
        assert_eq!(truncate_imp("hello world", 3, false), "...");
        assert_eq!(truncate_imp("hello world", 2, true), "..");
        assert_eq!(truncate_imp("hello world", 0, true), "");
    }

    #[test]
    fn test_evenly_shrink() {
        let mut c = [10, 20, 30];
        evenly_shrink(&mut c, 60);
        assert_eq!(c, [10, 20, 30]);
        let mut c = [10, 20, 30];
        evenly_shrink(&mut c, 45);
        assert_eq!(c, [10, 18, 17]);
        let mut c = [10, 20, 30];
        evenly_shrink(&mut c, 9);
        assert_eq!(c, [3, 3, 3]);
        let mut c = [5, 50, 50];
        evenly_shrink(&mut c, 50);
        assert_eq!(c, [5, 23, 22]);
        let mut c = [5, 50];
        evenly_shrink(&mut c, 0);
        assert_eq!(c, [0, 0]);
    }

    #[test]
    fn test_shrink_at_column() {
        let mut c = [10, 20, 30];
        shrink_at_column(&mut c, 50, 2);
        assert_eq!(c, [10, 20, 20]);
        let mut c = [10, 20, 30];
        shrink_at_column(&mut c, 25, 0);
        assert_eq!(c, [9, 8, 8]);
        let mut c = [10, 20, 30];
        shrink_at_column(&mut c, 50, 7);
        assert_eq!(c, [10, 20, 20]);
    }
}
