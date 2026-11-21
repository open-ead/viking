use goblin::container;
use goblin::elf::sym::{STT_COMMON, STT_FUNC, STT_NOTYPE, STT_OBJECT, STT_TLS};
use goblin::elf::{SectionHeader, Sym};

#[inline]
pub fn make_goblin_ctx() -> container::Ctx {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/elf.rs#L54
    // 64-bit, little endian
    container::Ctx::new(container::Container::Big, container::Endian::Little)
}

pub fn is_symbol_useful(sym: &Sym) -> bool {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/elf.rs#L176
    matches!(sym.st_type(), STT_OBJECT | STT_FUNC | STT_COMMON | STT_TLS)
}

#[inline]
pub fn is_in_section(section: &SectionHeader, addr: u64, size: u64) -> bool {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/elf.rs#L271
    let begin = section.sh_addr;
    let end = begin + section.sh_size;
    (begin..end).contains(&addr) && (begin..=end).contains(&(addr + size))
}

#[inline]
pub fn is_undefined_sym(sym: &Sym) -> bool {
    // REFERENCE=https://github.com/open-ead/viking/blob/e0eae2f7f403befec6b60cb615f6680a6937d0c9/viking/src/elf.rs#L184
    sym.st_type() == STT_NOTYPE && sym.st_value == 0
}
