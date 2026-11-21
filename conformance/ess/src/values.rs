#![forbid(unsafe_code)]
//! The model's values as the Rust values of the host interface, and back.
//!
//! A suite carries a value as an ESS [`Node`]: a `Uuid` and every newtype over one (`AgentId`,
//! `RealmId`, `ScriptId`) as its text, a `String` and an enum variant as text, an `Integer` as a
//! number, `Bytes` as padded standard base64 text (RFC 4648, section 4), and a struct
//! (`swell.host.Limits`) as a map of its fields.
//!
//! Two model types are wider than the interface's (`spec/domains/host.yaml`, the `UNMAPPED:`
//! markers of `swell.host.Limits` and of `CreateAgent`): a limit is an `Integer` in the model and
//! a `u64` or `u32` in [`Limits`], and a seed is `Bytes` of any length in the model and
//! `[u8; 16]` or `[u8; 32]` in [`Seeds`]. A limit outside its Rust type and a seed longer than
//! its array have no call in the interface, and are answered [`TargetError::Unsupported`]. A
//! shorter seed is padded with zero bytes, since the synthesized scenarios give one-byte seeds.

use std::collections::BTreeMap;

use ess_conformance::target::TargetError;
use ess_primitives::node::Node;
use swell::{Limits, Seeds};

/// The value of a `String`, a `Uuid` or an enum variant.
pub fn text(value: &str) -> Node {
    Node::Text(value.to_owned())
}

/// The 64 letters of standard base64, in the order of their values.
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// The value of one base64 letter.
fn letter(byte: u8) -> Option<u32> {
    ALPHABET
        .iter()
        .position(|&candidate| candidate == byte)
        .and_then(|index| u32::try_from(index).ok())
}

/// Padded standard base64 as bytes; `None` for any other text, padding bits that are not zero
/// included.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let input = text.as_bytes();
    if !input.len().is_multiple_of(4) {
        return None;
    }
    let groups = input.len() / 4;
    let mut bytes = Vec::with_capacity(groups * 3);
    for (number, group) in input.chunks(4).enumerate() {
        let padding = group.iter().rev().take_while(|&&byte| byte == b'=').count();
        if padding > 2 || (padding > 0 && number + 1 != groups) {
            return None;
        }
        let mut value = 0u32;
        for &byte in group.iter().take(4 - padding) {
            value = (value << 6) | letter(byte)?;
        }
        let padding_bits = u32::try_from(6 * padding).ok()?;
        value <<= padding_bits;
        let decoded = value.to_be_bytes();
        let kept = 3 - padding;
        let (data, rest) = decoded.get(1..)?.split_at(kept);
        if rest.iter().any(|&byte| byte != 0) {
            return None;
        }
        bytes.extend_from_slice(data);
    }
    Some(bytes)
}

/// `bytes` as a seed of `N` bytes: padded with zero bytes when shorter; `Unsupported` when
/// longer, a seed the interface has no call for.
pub fn seed<const N: usize>(name: &str, bytes: &[u8]) -> Result<[u8; N], TargetError> {
    let mut seed = [0u8; N];
    let Some(target) = seed.get_mut(..bytes.len()) else {
        return Err(TargetError::unsupported(
            format!(
                "`swell.host.CreateAgent` with a `{name}` of {} bytes",
                bytes.len()
            ),
            format!(
                "the interface takes a `{name}` of exactly {N} bytes (HOST-25, HOST-145, \
                 HOST-146); a longer one has no call"
            ),
        ));
    };
    target.copy_from_slice(bytes);
    Ok(seed)
}

/// The input of one command, read field by field. A field it lacks, a value of another type and
/// a field the command does not declare are each an adapter failure naming the field.
pub struct Input<'a> {
    command: &'a str,
    fields: &'a BTreeMap<String, Node>,
    read: Vec<&'static str>,
}

impl<'a> Input<'a> {
    /// The input `fields` of `command`.
    pub fn new(command: &'a str, fields: &'a BTreeMap<String, Node>) -> Self {
        Self {
            command,
            fields,
            read: Vec::new(),
        }
    }

    fn field(&mut self, name: &'static str, kind: &str) -> Result<&'a Node, TargetError> {
        self.read.push(name);
        self.fields.get(name).ok_or_else(|| {
            TargetError::unavailable(
                format!("reading the input of `{}`", self.command),
                format!("`{name}` ({kind}) is missing"),
            )
        })
    }

    fn wrong(&self, name: &str, kind: &str, value: &Node) -> TargetError {
        TargetError::unavailable(
            format!("reading the input of `{}`", self.command),
            format!(
                "`{name}` is not {kind}: {}",
                ess_conformance::report::quote(value)
            ),
        )
    }

    /// A `String`, `Uuid` or enum field.
    pub fn text(&mut self, name: &'static str) -> Result<String, TargetError> {
        match self.field(name, "text")? {
            Node::Text(value) => Ok(value.clone()),
            other => Err(self.wrong(name, "text", other)),
        }
    }

    /// A `Bytes` field.
    pub fn bytes(&mut self, name: &'static str) -> Result<Vec<u8>, TargetError> {
        let value = self.field(name, "padded base64 text")?;
        match value {
            Node::Text(text) => {
                decode(text).ok_or_else(|| self.wrong(name, "padded base64 text", value))
            }
            other => Err(self.wrong(name, "padded base64 text", other)),
        }
    }

    /// A `swell.host.Limits` field, as the interface's [`Limits`].
    pub fn limits(&mut self, name: &'static str) -> Result<Limits, TargetError> {
        let value = self.field(name, "a swell.host.Limits")?;
        let Node::Map(fields) = value else {
            return Err(self.wrong(name, "a swell.host.Limits", value));
        };
        let mut limits = Input::new(self.command, fields);
        let read = Limits {
            steps_per_entry: limits.limit("steps_per_entry")?,
            memory_bytes: limits.limit("memory_bytes")?,
            frame_depth: limits.limit("frame_depth")?,
            reentry_depth: limits.limit("reentry_depth")?,
            parser_nesting: limits.limit("parser_nesting")?,
            string_length: limits.limit("string_length")?,
            bigint_bits: limits.limit("bigint_bits")?,
        };
        limits.finish()?;
        Ok(read)
    }

    /// One limit, an `Integer` in the model, as the Rust type `T` of its field of [`Limits`].
    fn limit<T: TryFrom<i64>>(&mut self, name: &'static str) -> Result<T, TargetError> {
        let value = self.field(name, "an integer")?;
        let integer = match value {
            Node::Number(number) => number.as_i64(),
            _ => None,
        }
        .ok_or_else(|| self.wrong(name, "an integer", value))?;
        T::try_from(integer).map_err(|_| {
            TargetError::unsupported(
                format!("`swell.host.CreateAgent` with the limit `{name}` = {integer}"),
                format!(
                    "`Limits.{name}` of the interface does not hold {integer} (decision 2: u32 \
                     for the depths and `parser_nesting`, u64 for the rest)"
                ),
            )
        })
    }

    /// Fails naming every field the command does not declare.
    pub fn finish(self) -> Result<(), TargetError> {
        let undeclared: Vec<&str> = self
            .fields
            .keys()
            .map(String::as_str)
            .filter(|field| !self.read.contains(field))
            .collect();
        if undeclared.is_empty() {
            Ok(())
        } else {
            Err(TargetError::unavailable(
                format!("reading the input of `{}`", self.command),
                format!("undeclared field(s): {}", undeclared.join(", ")),
            ))
        }
    }
}

/// The seeds of `CreateAgent` from its two `Bytes` inputs.
pub fn seeds(hash_key: &[u8], random_seed: &[u8]) -> Result<Seeds, TargetError> {
    Ok(Seeds {
        hash_key: seed("hash_key", hash_key)?,
        random_seed: seed("random_seed", random_seed)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{decode, seed};

    #[test]
    fn base64_reads_the_rfc_4648_vectors_and_the_suites_seeds() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(decode(coded), Some(plain.as_bytes().to_vec()), "{coded}");
        }
        assert_eq!(decode("AA=="), Some(vec![0]));
        assert_eq!(decode("+A=="), Some(vec![0xf8]));
        assert_eq!(
            decode("AAECAwQFBgcICQoLDA0ODw=="),
            Some((0..16).collect::<Vec<u8>>())
        );
    }

    #[test]
    fn base64_refuses_what_is_not_padded_standard_base64() {
        for text in [
            "A", "AA", "AA=", "A===", "AA==AA==", "AB==", "Zm9-", "Zm9_", "Zg= =",
        ] {
            assert_eq!(decode(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_short_seed_is_padded_with_zero_bytes_and_a_long_one_is_unsupported() {
        assert_eq!(seed::<4>("hash_key", &[7]).ok(), Some([7, 0, 0, 0]));
        assert_eq!(seed::<2>("hash_key", &[1, 2]).ok(), Some([1, 2]));
        let refused = seed::<2>("hash_key", &[1, 2, 3]);
        assert!(refused.is_err_and(|error| error.is_unsupported()));
    }
}
