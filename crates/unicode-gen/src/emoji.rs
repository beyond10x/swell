//! The properties of strings of ECMA-262's table (`#table-binary-unicode-properties-of-strings`)
//! from the emoji sequence files of UTS #51 (`emoji/emoji-sequences.txt`,
//! `emoji/emoji-zwj-sequences.txt`): each property's single code points as merged ranges, its
//! sequences of two code points or more sorted, and the most code points of one of its
//! sequences, which bounds the lookups of the matcher at one position. `RGI_Emoji` is the union
//! of the other six (UTS #51, ED-27) and is not written: it is made at run time.

use std::collections::BTreeMap;

use crate::ucd::{Element, Ucd, merge};

/// The properties of strings the files name, in the order of ECMA-262's table.
pub(crate) const PROPERTIES: [&str; 6] = [
    "Basic_Emoji",
    "Emoji_Keycap_Sequence",
    "RGI_Emoji_Modifier_Sequence",
    "RGI_Emoji_Flag_Sequence",
    "RGI_Emoji_Tag_Sequence",
    "RGI_Emoji_ZWJ_Sequence",
];

/// The property of strings of ECMA-262's table that is the union of the others.
pub(crate) const UNION: &str = "RGI_Emoji";

/// What one property of strings holds.
pub(crate) struct Strings {
    /// Its single code points: sorted, disjoint inclusive ranges, two never adjacent.
    pub(crate) singles: Vec<(u32, u32)>,
    /// Its sequences of two code points or more, sorted, none twice.
    pub(crate) sequences: Vec<Vec<u32>>,
    /// The most code points of one of its sequences; 1 when it has none.
    pub(crate) longest: usize,
}

/// The six properties of [`PROPERTIES`], in that order, from `sources`, the files of sequences.
/// A line of a property no other file names, a property of the list no file names, a sequence
/// given twice, and a property named [`UNION`] in a file are errors.
pub(crate) fn properties(ucd: &mut Ucd, sources: &[&str]) -> Result<Vec<Strings>, String> {
    let mut singles: BTreeMap<&str, Vec<(u32, u32)>> = BTreeMap::new();
    let mut sequences: BTreeMap<&str, Vec<Vec<u32>>> = BTreeMap::new();
    for source in sources {
        let name = ucd.source_name(source);
        for read in ucd.sequences(source)? {
            let Some(&property) = PROPERTIES.iter().find(|p| **p == read.property) else {
                return Err(format!(
                    "{name}:{}: the property {} is none of the properties of strings {}",
                    read.line,
                    read.property,
                    PROPERTIES.join(", ")
                ));
            };
            match read.element {
                Element::Range(first, last) => {
                    singles.entry(property).or_default().push((first, last));
                }
                Element::Sequence(sequence) => {
                    sequences.entry(property).or_default().push(sequence);
                }
            }
        }
    }
    let mut out = Vec::with_capacity(PROPERTIES.len());
    for property in PROPERTIES {
        let own_singles = merge(singles.remove(property).unwrap_or_default());
        let mut own = sequences.remove(property).unwrap_or_default();
        if own_singles.is_empty() && own.is_empty() {
            return Err(format!(
                "no line of {} names the property {property}",
                sources.join(", ")
            ));
        }
        own.sort_unstable();
        if let Some(pair) = own.windows(2).find(|pair| pair.first() == pair.get(1)) {
            return Err(format!(
                "{property}: the sequence {:04X?} is given twice",
                pair.first().cloned().unwrap_or_default()
            ));
        }
        let longest = own.iter().map(Vec::len).max().unwrap_or(1);
        out.push(Strings {
            singles: own_singles,
            sequences: own,
            longest,
        });
    }
    Ok(out)
}

/// The name of the constants of `property`: its name in capitals.
pub(crate) fn constant(property: &str) -> String {
    property.to_ascii_uppercase()
}
