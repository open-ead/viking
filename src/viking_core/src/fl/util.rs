use cu::pre::*;

use crate::config::Config;

/// Parse an absolute address (offset + base address). The value is checked
/// to ensure the offset is a valid U32.
pub fn parse_abs_address(config: &Config, hex_value: &str) -> cu::Result<u64> {
    Ok(parse_abs_address_as_offset(config, hex_value)? as u64 + config.repo.address_base)
}

/// Parse an absolute address (offset + base address) to just the offset
/// according to the base address in the config
pub fn parse_abs_address_as_offset(config: &Config, hex_value: &str) -> cu::Result<u32> {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/functions.rs#L59
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/functions.rs#L66
    let hex_value = hex_value.strip_prefix("0x").unwrap_or(hex_value);
    let hex = cu::check!(
        u64::from_str_radix(hex_value, 16),
        "invalid address: address must be in hex"
    )?;
    let base = config.repo.address_base;
    let offset = cu::check!(
        hex.checked_sub(base),
        "invalid address: address must be greater than base (0x{base:x}), got 0x{hex:x}"
    )?;
    cu::ensure!(
        offset <= u32::MAX as u64,
        "invalid address: address is too big (0x{offset:x}), this is unlikely to be correct."
    )?;
    Ok(offset as u32)
}

/// Parse an address as the offset (without the base address)
pub fn parse_offset(hex_value: &str) -> cu::Result<u32> {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/functions.rs#L59
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/functions.rs#L66
    let hex_value = hex_value.strip_prefix("0x").unwrap_or(hex_value);
    cu::check!(
        u32::from_str_radix(hex_value, 16),
        "invalid address: address must be in hex"
    )
}
