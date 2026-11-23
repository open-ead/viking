use std::sync::Once;

use crate::pyinterop::pre::*;

mod print;
pub use print::*;
mod prompt;
pub use prompt::*;
mod tabular;
pub use tabular::*;
mod tabular_fields;
pub use tabular_fields::*;
mod format;
pub use format::*;

/// Clear the terminal
#[pyfunction]
pub fn clear_terminal() {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L62
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0),
    )
    .unwrap_or(());
}

/// Initialize logging based on `RUST_LOG` environment variable.
pub fn init_logger() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // only display logging (especially debug/trace) if RUST_LOG is explicitly
        // set. This allows debugging with RUST_LOG=debug or trace but still
        // keeps our nice UI formatting when running normally
        let rust_log = cu::env_var("RUST_LOG").unwrap_or_default();
        if !rust_log.is_empty() {
            env_logger::init_from_env(rust_log);
        }
    });
}
