use std::borrow::Cow;
use std::fmt::Write;

use cu::pre::*;

use crate::fl;

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
    if is_safe_yaml_string(s) {
        return s.into();
    }
    format!("\'{}\'", s.replace('\'', "''")).into()
}

/// If the string is parsed back as the same string when written as a plain scalar,
/// including in flow context
fn is_safe_yaml_string(s: &str) -> bool {
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
