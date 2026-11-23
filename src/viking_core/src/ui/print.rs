use std::io::{StderrLock, Write};

use colored::*;

use crate::pyinterop::pre::*;

/// Print a message using the step formatting
#[pyfunction]
pub fn print_step(msg: &str) {
    println!("{} {}", ">>>".bright_green(), msg.bright_green())
}
pub use viking_macro::step;

/// Print a message using the note formatting
#[pyfunction]
pub fn print_note(msg: &str) {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L11
    eprintln!("{}{}{}", "note".bold().cyan(), ": ".bold(), msg.bold())
}
pub use viking_macro::note;

/// Print either a warning or an error, controlled by `is_error`
pub fn print_warn_or_error(msg: &str, is_error: bool) {
    if is_error {
        print_error(msg);
    } else {
        print_warning(msg);
    }
}

/// Print a message using warning formatting
#[pyfunction]
pub fn print_warning(msg: &str) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    print_warning_locked(&mut lock, msg);
}
pub use viking_macro::warn_ as warn;

/// Print a message using warning formatting plus details. The details
/// are automatically wrapped.
///
/// ```text
/// warning: {msg}
///   |  detail...
///   |  detail...
///   |  detail...
/// ```
#[pyfunction]
pub fn print_warning_with_detail(msg: &str, detail: &str) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    print_warning_locked(&mut lock, msg);
    print_detail_locked(&mut lock, detail);
}

fn print_warning_locked(lock: &mut StderrLock, msg: &str) {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L15
    writeln!(
        lock,
        "{}{}{}",
        "warning".bold().yellow(),
        ": ".bold(),
        msg.bold()
    )
    .unwrap();
}

/// Print a message using error formatting
#[pyfunction]
pub fn print_error(msg: &str) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    print_error_locked(&mut lock, msg);
}
pub use viking_macro::error;

/// Print a message using error formatting plus details. The details
/// are automatically wrapped.
///
/// ```text
/// error: {msg}
///   |  detail...
///   |  detail...
///   |  detail...
/// ```
#[pyfunction]
pub fn print_error_with_detail(msg: &str, detail: &str) {
    let stderr = std::io::stderr();
    let mut lock = stderr.lock();
    print_error_locked(&mut lock, msg);
    print_detail_locked(&mut lock, detail);
}

fn print_error_locked(lock: &mut StderrLock, msg: &str) {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L25
    writeln!(
        lock,
        "{}{}{}",
        "error".bold().red(),
        ": ".bold(),
        msg.bold()
    )
    .unwrap();
}

fn print_detail_locked(lock: &mut StderrLock, msg: &str) {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L52
    writeln!(
        lock,
        "{}\n",
        textwrap::indent(&msg.clear(), &"  │  ".bold().dimmed())
    )
    .unwrap();
}
