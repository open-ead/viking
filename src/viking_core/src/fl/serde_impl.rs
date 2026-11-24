use std::borrow::Cow;
use std::fmt::Write;
use std::io::Cursor;

use cu::pre::*;
use rayon::prelude::*;

use crate::config::OBJECT_LEN_UPPER_BOUND;
use crate::fl;
use crate::ui;

/// Parse the file list yml objects fast
pub fn parse_file_list_yml_objects(bytes: &[u8]) -> cu::Result<Vec<(String, fl::Object)>> {
    // manually parse the span for each object, then parse the objects
    // parallely. We also assume UTF-8 to avoid check code point boundary

    // first parse the spans for each object
    let mut object_indices = Vec::with_capacity(OBJECT_LEN_UPPER_BOUND);
    let mut curr_object: Option<(String, usize)> = None;
    let mut curr_i = 0;
    let bytes_len = bytes.len();
    while curr_i < bytes_len {
        let line_end_i = match bytes[curr_i..].iter().position(|b| *b == b'\n') {
            None => bytes_len,
            Some(i) => curr_i + i + 1,
        };
        if line_end_i == curr_i {
            break;
        }
        match bytes[curr_i] {
            b'#' | b' ' => {
                // comment or not an object start
            }
            _ => {
                // manually parse the object name
                let line = match str::from_utf8(&bytes[curr_i..line_end_i]) {
                    Err(_) => {
                        cu::bail!(
                            "object name is not utf-8: {}",
                            String::from_utf8_lossy(&bytes[curr_i..line_end_i])
                        )
                    }
                    Ok(line) => line,
                };
                let object_name = match line.split_once(": ") {
                    Some((x, _)) => x,
                    None => cu::check!(
                        line.trim_end().strip_suffix(':'),
                        "invalid object name line: {}",
                        line.trim_end()
                    )?,
                };
                let object_name = fl::unquote_yaml_string(object_name);
                if let Some((object_name, start)) = curr_object.take() {
                    object_indices.push((object_name, start, curr_i));
                }
                curr_object = Some((object_name, line_end_i));
            }
        }
        curr_i = line_end_i;
    }
    if let Some((object_name, start)) = curr_object.take() {
        object_indices.push((object_name, start, bytes_len));
    }

    // parallely parse each span
    let mut object_results = Vec::with_capacity(object_indices.len());
    object_indices
        .into_par_iter()
        .map(|(name, start_i, end_i)| {
            let object_bytes = &bytes[start_i..end_i];
            let object = cu::check!(
                yaml::read::<fl::Object>(Cursor::new(&object_bytes)),
                "failed to parse file list object '{name}'"
            )?;
            cu::Ok((name, object))
        })
        .collect_into_vec(&mut object_results);

    // final pass to print any errors
    let mut objects = Vec::with_capacity(object_results.len());
    let expected_len = object_results.len();
    for result in object_results {
        match result {
            Err(e) => {
                ui::error!("{e:?}");
            }
            Ok(x) => objects.push(x),
        }
    }
    if objects.len() != expected_len {
        cu::bail!("error parsing objects in the file list");
    }

    Ok(objects)
}

// The YAML serialization is custom so we can layout the exact format,
// like spacing and serializing numbers in hex
//
// The format mimics what SMO already does:
// - https://github.com/MonsterDruide1/OdysseyDecomp/blob/master/data/file_list.yml
// - https://github.com/LynxDev2/nx-decomp-tools/blob/file-list-tools/viking/src/functions.rs

impl fl::Object {
    /// Format into YAML file list writer
    pub fn fmt_file_list_yml<W: Write>(&self, mut out: W) -> std::fmt::Result {
        let prefix = "  ";
        writeln!(out, "{prefix}'.text':")?;
        for info in &self.text_section {
            info.fmt_file_list_yml(&mut out)?;
        }
        Ok(())
    }
}

impl fl::InfoList {
    /// Format into YAML file list writer
    pub fn fmt_file_list_yml<W: Write>(&self, mut out: W) -> std::fmt::Result {
        for info in self.iter() {
            info.fmt_file_list_yml(&mut out)?;
        }
        Ok(())
    }
}

impl fl::InfoData {
    /// Format into YAML file list writer
    pub fn fmt_file_list_yml<W: Write>(&self, mut out: W) -> std::fmt::Result {
        let prefix = "  ";
        writeln!(out, "{prefix}- offset: 0x{:08x}", self.offset)?;
        writeln!(out, "{prefix}  size: {}", self.size)?;
        if self.label.names().len() == 1 {
            writeln!(
                out,
                "{prefix}  label: {}",
                fl::quote_yaml_string_if_needed(self.label.name())
            )?;
        } else {
            writeln!(out, "{prefix}  label:")?;
            for l in self.label.names() {
                writeln!(out, "{prefix}  - {}", fl::quote_yaml_string_if_needed(l))?;
            }
        }
        writeln!(
            out,
            "{prefix}  status: {}",
            yaml::stringify(&self.status).unwrap()
        )?;
        if self.lazy {
            writeln!(out, "{prefix}  lazy: true")?;
        }
        if self.guess {
            writeln!(out, "{prefix}  guess: true")?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for fl::AddressLabel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, SeqAccess, Visitor};
        struct LabelVisitor;
        impl<'de> Visitor<'de> for LabelVisitor {
            type Value = fl::AddressLabel;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a string, or a non-empty sequence of non-empty strings")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                self.visit_string(v.to_string())
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(fl::AddressLabel::new_unchecked(vec![v]))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut names = Vec::with_capacity(seq.size_hint().unwrap_or(2));
                while let Some(name) = seq.next_element::<String>()? {
                    names.push(name);
                }
                if names.is_empty() {
                    return Err(de::Error::invalid_length(0, &self));
                }
                if names.len() > 1 {
                    for n in &names {
                        if n.is_empty() {
                            return Err(de::Error::invalid_value(
                                de::Unexpected::Str(n),
                                &"a non-empty string",
                            ));
                        }
                    }
                }
                Ok(fl::AddressLabel::new_unchecked(names))
            }
        }

        deserializer.deserialize_any(LabelVisitor)
    }
}

impl<T: Serialize> Serialize for fl::StatusMap<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        // serialize in Status::all() order for stable output
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for status in fl::Status::all() {
            if let Some(value) = self.get(status) {
                map.serialize_entry(status, value)?;
            }
        }
        map.end()
    }
}

pub fn quote_yaml_string_if_needed(s: &str) -> Cow<'_, str> {
    // LLM Generated
    if is_safe_yaml_string(s) {
        return s.into();
    }
    format!("\'{}\'", s.replace('\'', "''")).into()
}

/// Unquote a maybe-quoted YAML scalar like `abc`, `'ab''c'`, or `"ab\nc"`.
///
/// - Plain scalars are returned as-is (trimmed)
/// - Single-quoted: `''` is an escaped `'`, everything else is literal
/// - Double-quoted: backslash escapes are processed. Invalid escapes are kept verbatim
///
/// Line folding of multi-line quoted scalars is not supported, since the input
/// is expected to be a single line.
pub fn unquote_yaml_string(s: &str) -> String {
    // LLM Generated
    let s = s.trim();
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        match (bytes[0], bytes[bytes.len() - 1]) {
            (b'\'', b'\'') => return s[1..s.len() - 1].replace("''", "'"),
            (b'"', b'"') => return unescape_yaml_double_quoted(&s[1..s.len() - 1]),
            _ => {}
        }
    }
    s.to_string()
}

/// Process escapes in the content of a double-quoted YAML scalar (without the quotes)
fn unescape_yaml_double_quoted(s: &str) -> String {
    // LLM Generated
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('\\') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let mut chars = after.chars();
        let Some(c) = chars.next() else {
            // trailing backslash, keep it
            out.push('\\');
            return out;
        };
        let simple = match c {
            '0' => Some('\0'),
            'a' => Some('\x07'),
            'b' => Some('\x08'),
            't' | '\t' => Some('\t'),
            'n' => Some('\n'),
            'v' => Some('\x0b'),
            'f' => Some('\x0c'),
            'r' => Some('\r'),
            'e' => Some('\x1b'),
            ' ' => Some(' '),
            '"' => Some('"'),
            '/' => Some('/'),
            '\\' => Some('\\'),
            'N' => Some('\u{85}'),
            '_' => Some('\u{a0}'),
            'L' => Some('\u{2028}'),
            'P' => Some('\u{2029}'),
            _ => None,
        };
        if let Some(ch) = simple {
            out.push(ch);
            rest = &after[c.len_utf8()..];
            continue;
        }
        let hex_len = match c {
            'x' => 2,
            'u' => 4,
            'U' => 8,
            _ => 0,
        };
        let decoded = (hex_len > 0)
            .then(|| after.get(1..1 + hex_len))
            .flatten()
            .filter(|hex| hex.bytes().all(|b| b.is_ascii_hexdigit()))
            .and_then(|hex| u32::from_str_radix(hex, 16).ok())
            .and_then(char::from_u32);
        match decoded {
            Some(ch) => {
                out.push(ch);
                rest = &after[1 + hex_len..];
            }
            None => {
                // invalid escape, keep verbatim
                out.push('\\');
                out.push(c);
                rest = &after[c.len_utf8()..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// If the string is parsed back as the same string when written as a plain scalar,
/// including in flow context
fn is_safe_yaml_string(s: &str) -> bool {
    // LLM Generated
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '$' | '/' | '-')) {
        return false;
    }
    // words that could be parsed as non-string
    !matches!(
        s.to_ascii_lowercase().as_str(),
        "null" | "true" | "false" | "yes" | "no" | "on" | "off" | "y" | "n"
    )
}
