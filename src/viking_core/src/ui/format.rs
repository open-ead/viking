use colored::Colorize;

use cu::pre::*;

use crate::demangle;
use crate::pyinterop::pre::*;

/// Format a mangled symbol name for display. Show as `{demangled} ({mangled})`
#[pyfunction]
pub fn format_symbol_name(name: &str) -> String {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L36
    match demangle::demangle_str(name) {
        Ok(demangled) => format!("{} ({})", demangled.blue(), name.blue().dimmed()),
        Err(_) => name.blue().to_string(),
    }
}

/// Format an address for display
#[pyfunction]
pub fn format_address(addr: u64) -> String {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/ui.rs#L42
    format!("{addr:#x}").green().to_string()
}

/// Human readable byte size, using SI units (1 KB = 1000 B).
pub struct ByteFormat(pub u32);
impl std::fmt::Display for ByteFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const UNITS: [(u32, &str); 3] = [(1_000_000_000, "GB"), (1_000_000, "MB"), (1_000, "KB")];
        let bytes = self.0;
        let precision = f.precision().unwrap_or(2);
        for (scale, unit) in UNITS {
            if bytes >= scale {
                let value = bytes as f64 / scale as f64;
                return write!(f, "{value:.precision$} {unit}");
            }
        }
        write!(f, "{bytes} B")
    }
}

/// Percentage format
#[derive(Display)]
#[display("{:.2}%", self.0 as f32 * 100f32 / self.1 as f32)]
pub struct Percent(pub u32, pub u32);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_format() {
        assert_eq!(ByteFormat(0).to_string(), "0 B");
        assert_eq!(ByteFormat(999).to_string(), "999 B");
        assert_eq!(ByteFormat(1000).to_string(), "1.00 KB");
        assert_eq!(ByteFormat(1500).to_string(), "1.50 KB");
        assert_eq!(ByteFormat(12_345_678).to_string(), "12.35 MB");
        assert_eq!(ByteFormat(u32::MAX).to_string(), "4.29 GB");
        assert_eq!(format!("{:.1}", ByteFormat(1_250_000)), "1.2 MB");
    }
}
