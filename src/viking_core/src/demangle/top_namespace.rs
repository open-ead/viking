use crate::pyinterop::pre::*;
// LLM Generated

/// Get the top-level namespace of a (possibly mangled) function name.
#[pyfunction]
pub fn get_top_namespace(function_name: &str) -> Option<&str> {
    match function_name.strip_prefix("_Z") {
        Some(mangled) => mangled_encoding_top_namespace(mangled),
        None => {
            let idx = function_name.find("::")?;
            let ns = &function_name[..idx];
            if ns.is_empty() || ns.contains(['<', '(']) {
                return None;
            }
            Some(ns)
        }
    }
}

/// `<encoding> ::= <name> <bare-function-type> | <name> | <special-name>`
fn mangled_encoding_top_namespace(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    match *b.first()? {
        b'T' => match *b.get(1)? {
            // vtable, VTT, typeinfo, typeinfo name: followed by <type>
            b'V' | b'T' | b'I' | b'S' => mangled_name_top_namespace(&s[2..]),
            // thunks: T <call-offset> <encoding> / Tc <call-offset> <call-offset> <encoding>
            b'h' | b'v' => mangled_encoding_top_namespace(skip_call_offset(&s[1..])?),
            b'c' => {
                let rest = skip_call_offset(&s[2..])?;
                mangled_encoding_top_namespace(skip_call_offset(rest)?)
            }
            _ => None,
        },
        // guard variable / reference temporary: followed by <name>
        b'G' => match *b.get(1)? {
            b'V' | b'R' => mangled_name_top_namespace(&s[2..]),
            _ => None,
        },
        _ => mangled_name_top_namespace(s),
    }
}

/// `<name> ::= <nested-name> | <unscoped-name> | <unscoped-template-name> <template-args> | <local-name>`
fn mangled_name_top_namespace(s: &str) -> Option<&str> {
    let b = s.as_bytes();
    match *b.first()? {
        // internal linkage prefix (clang)
        b'L' => mangled_name_top_namespace(&s[1..]),
        b'N' => mangled_nested_name_top_namespace(&s[1..]),
        // local name: Z <function encoding> E <entity>, use the enclosing function
        b'Z' => mangled_encoding_top_namespace(&s[1..]),
        // St <unqualified-name> (::std::), or std substitutions (Sa, Ss, ...)
        b'S' => std_substitution(&s[1..]),
        // unscoped name - no namespace
        _ => None,
    }
}

/// After `N`: `[<CV-qualifiers>] [<ref-qualifier>] <prefix> <unqualified-name> E`
fn mangled_nested_name_top_namespace(s: &str) -> Option<&str> {
    let s = s.trim_start_matches(['r', 'V', 'K']);
    let s = s.strip_prefix(['R', 'O']).unwrap_or(s);
    let s = s.strip_prefix('L').unwrap_or(s);
    if let Some(rest) = s.strip_prefix('S') {
        return std_substitution(rest);
    }
    let (name, mut rest) = parse_source_name(s)?;
    // skip ABI tags: B <source-name>
    while let Some(tag) = rest.strip_prefix('B') {
        rest = parse_source_name(tag)?.1;
    }
    match rest.as_bytes().first()? {
        // E: only one component, so no namespace
        // I: top level is a template
        b'E' | b'I' => None,
        _ => {
            if name.starts_with("_GLOBAL__N") {
                // (anonymous namespace)
                None
            } else {
                Some(name)
            }
        }
    }
}

/// After `S`: check for `St` or a standard abbreviation that lives in `std`
fn std_substitution(s: &str) -> Option<&'static str> {
    match *s.as_bytes().first()? {
        b't' | b'a' | b'b' | b's' | b'i' | b'o' | b'd' => Some("std"),
        // S_ / S<seq-id>_ are substitutions, which cannot appear
        // before anything to substitute
        _ => None,
    }
}

/// `<source-name> ::= <positive length number> <identifier>`
///
/// Returns the identifier and the rest of the input
fn parse_source_name(s: &str) -> Option<(&str, &str)> {
    let digits = s.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let len: usize = s[..digits].parse().ok()?;
    let s = &s[digits..];
    if len == 0 || !s.is_char_boundary(len) {
        return None;
    }
    s.split_at_checked(len)
}

/// `<call-offset> ::= h <nv-offset> _ | v <v-offset> _`, where
/// `<nv-offset> ::= <number>` and `<v-offset> ::= <number> _ <number>`
fn skip_call_offset(s: &str) -> Option<&str> {
    fn skip_number_underscore(s: &str) -> Option<&str> {
        let s = s.strip_prefix('n').unwrap_or(s);
        let s = s.trim_start_matches(|c: char| c.is_ascii_digit());
        s.strip_prefix('_')
    }
    match *s.as_bytes().first()? {
        b'h' => skip_number_underscore(&s[1..]),
        b'v' => skip_number_underscore(skip_number_underscore(&s[1..])?),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mangled() {
        assert_eq!(get_top_namespace("_Z3foov"), None);
        assert_eq!(get_top_namespace("_ZL3foov"), None);
        assert_eq!(get_top_namespace("_ZN3foo3barEv"), Some("foo"));
        assert_eq!(get_top_namespace("_ZNK3foo3Bar3bazEv"), Some("foo"));
        assert_eq!(get_top_namespace("_ZN3foo3barE"), Some("foo"));
        assert_eq!(get_top_namespace("_ZN3FooC2Ev"), Some("Foo"));
        assert_eq!(get_top_namespace("_ZN3FooIiE3barEv"), None);
        assert_eq!(
            get_top_namespace("_ZNSt6vectorIiSaIiEE9push_backERKi"),
            Some("std")
        );
        assert_eq!(get_top_namespace("_ZSt4swapIiEvRT_S1_"), Some("std"));
        assert_eq!(get_top_namespace("_ZNSs4sizeEv"), Some("std"));
        assert_eq!(get_top_namespace("_ZN12_GLOBAL__N_13fooEv"), None);
        assert_eq!(get_top_namespace("_ZZN3foo3barEvE1x"), Some("foo"));
        assert_eq!(get_top_namespace("_ZZ3barvE1x"), None);
        assert_eq!(get_top_namespace("_ZTVN3foo3BarE"), Some("foo"));
        assert_eq!(get_top_namespace("_ZTIi"), None);
        assert_eq!(get_top_namespace("_ZGVZN3foo3barEvE1x"), Some("foo"));
        assert_eq!(get_top_namespace("_ZThn8_N3foo3Bar3bazEv"), Some("foo"));
        assert_eq!(get_top_namespace("_ZTv0_n24_N3foo3Bar3bazEv"), Some("foo"));
        assert_eq!(get_top_namespace("_ZN3foo3barB5cxx11Ev"), Some("foo"));
        assert_eq!(get_top_namespace("_ZN2ns3FooB3tagIiE3barEv"), Some("ns"));
    }

    #[test]
    fn test_unmangled() {
        assert_eq!(get_top_namespace("main"), None);
        assert_eq!(get_top_namespace("foo::bar"), Some("foo"));
        assert_eq!(get_top_namespace("foo::Bar::baz(int)"), Some("foo"));
        assert_eq!(get_top_namespace("Foo<a::b>::bar"), None);
        assert_eq!(get_top_namespace("foo(a::b)"), None);
        assert_eq!(get_top_namespace("::foo"), None);
    }
}
