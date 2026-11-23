//! The property data of the tables of `swell-unicode` (design, decision 8) that the property
//! escapes `\p{...}` and `\P{...}` read (`#sec-runtime-semantics-unicodematchproperty-p`,
//! `#sec-runtime-semantics-unicodematchpropertyvalue-p-v`): the values of General_Category of
//! `DerivedGeneralCategory.txt`, the values of Script of `Scripts.txt`, the sets of
//! `ScriptExtensions.txt`, the binary properties of ECMA-262's table from the files that hold
//! them, and the names and aliases of all of them from `PropertyAliases.txt` and
//! `PropertyValueAliases.txt`.
//!
//! ECMA-262 admits the names and aliases of its two tables of properties
//! (`#table-nonbinary-unicode-properties`, `#table-binary-unicode-properties`) and no other, so
//! those tables are written here as the draft gives them, and each name and alias of them is
//! checked against its line of `PropertyAliases.txt`. The values of General_Category and Script
//! and their aliases are every field of their lines of `PropertyValueAliases.txt`.
//!
//! The lines of `PropertyAliases.txt` and `PropertyValueAliases.txt` are fields separated by `;`
//! with no code point, which [`Ucd::ranges`] does not read; they are read here from
//! [`Ucd::data_lines`].

use std::collections::{BTreeMap, BTreeSet};

use crate::ucd::Ucd;

/// The non-binary properties of ECMA-262 (`#table-nonbinary-unicode-properties`): the canonical
/// name and its aliases, in the order of the table. Their numbers in
/// `property_names::PROPERTIES` are their places here.
pub(crate) const NON_BINARY: &[(&str, &[&str])] = &[
    ("General_Category", &["gc"]),
    ("Script", &["sc"]),
    ("Script_Extensions", &["scx"]),
];

/// The binary properties of ECMA-262 (`#table-binary-unicode-properties`): the canonical name and
/// its aliases, in the order of the table.
pub(crate) const BINARY: &[(&str, &[&str])] = &[
    ("ASCII", &[]),
    ("ASCII_Hex_Digit", &["AHex"]),
    ("Alphabetic", &["Alpha"]),
    ("Any", &[]),
    ("Assigned", &[]),
    ("Bidi_Control", &["Bidi_C"]),
    ("Bidi_Mirrored", &["Bidi_M"]),
    ("Case_Ignorable", &["CI"]),
    ("Cased", &[]),
    ("Changes_When_Casefolded", &["CWCF"]),
    ("Changes_When_Casemapped", &["CWCM"]),
    ("Changes_When_Lowercased", &["CWL"]),
    ("Changes_When_NFKC_Casefolded", &["CWKCF"]),
    ("Changes_When_Titlecased", &["CWT"]),
    ("Changes_When_Uppercased", &["CWU"]),
    ("Dash", &[]),
    ("Default_Ignorable_Code_Point", &["DI"]),
    ("Deprecated", &["Dep"]),
    ("Diacritic", &["Dia"]),
    ("Emoji", &[]),
    ("Emoji_Component", &["EComp"]),
    ("Emoji_Modifier", &["EMod"]),
    ("Emoji_Modifier_Base", &["EBase"]),
    ("Emoji_Presentation", &["EPres"]),
    ("Extended_Pictographic", &["ExtPict"]),
    ("Extender", &["Ext"]),
    ("Grapheme_Base", &["Gr_Base"]),
    ("Grapheme_Extend", &["Gr_Ext"]),
    ("Hex_Digit", &["Hex"]),
    ("IDS_Binary_Operator", &["IDSB"]),
    ("IDS_Trinary_Operator", &["IDST"]),
    ("ID_Continue", &["IDC"]),
    ("ID_Start", &["IDS"]),
    ("Ideographic", &["Ideo"]),
    ("Join_Control", &["Join_C"]),
    ("Logical_Order_Exception", &["LOE"]),
    ("Lowercase", &["Lower"]),
    ("Math", &[]),
    ("Noncharacter_Code_Point", &["NChar"]),
    ("Pattern_Syntax", &["Pat_Syn"]),
    ("Pattern_White_Space", &["Pat_WS"]),
    ("Quotation_Mark", &["QMark"]),
    ("Radical", &[]),
    ("Regional_Indicator", &["RI"]),
    ("Sentence_Terminal", &["STerm"]),
    ("Soft_Dotted", &["SD"]),
    ("Terminal_Punctuation", &["Term"]),
    ("Unified_Ideograph", &["UIdeo"]),
    ("Uppercase", &["Upper"]),
    ("Variation_Selector", &["VS"]),
    ("White_Space", &["space"]),
    ("XID_Continue", &["XIDC"]),
    ("XID_Start", &["XIDS"]),
];

/// The binary properties of ECMA-262's table that no data file holds, made at run time from
/// their definitions in UTS #18 (RL1.2): `Any` every code point, `ASCII` U+0000 to U+007F,
/// `Assigned` every code point whose General_Category is not `Cn`. In the names table they come
/// after the properties of [`binary_order`], in this order.
pub(crate) const RUN_TIME: &[&str] = &["Any", "ASCII", "Assigned"];

/// The binary properties whose ranges another table of the list writes, with the path of that
/// constant from `binary_properties.rs`: they are not written twice.
const ELSEWHERE: &[(&str, &str)] = &[
    ("Case_Ignorable", "super::case_ignorable::CASE_IGNORABLE"),
    ("Cased", "super::cased::CASED"),
];

/// The groups of General_Category (UAX #44, table 12, "General_Category Values"; the comments of
/// their lines of `PropertyValueAliases.txt` name the same): a value of the property whose code
/// points are those of its members, made at run time.
const GROUPS: &[(&str, &[&str])] = &[
    ("C", &["Cc", "Cf", "Cs", "Co", "Cn"]),
    ("L", &["Lu", "Ll", "Lt", "Lm", "Lo"]),
    ("LC", &["Lu", "Ll", "Lt"]),
    ("M", &["Mn", "Mc", "Me"]),
    ("N", &["Nd", "Nl", "No"]),
    ("P", &["Pc", "Pd", "Ps", "Pe", "Pi", "Pf", "Po"]),
    ("S", &["Sm", "Sc", "Sk", "So"]),
    ("Z", &["Zs", "Zl", "Zp"]),
];

/// The largest code point.
const MAX_CODE_POINT: u32 = 0x10FFFF;

/// Sorted, disjoint inclusive ranges `(first, last)`.
pub(crate) type Ranges = Vec<(u32, u32)>;

/// The binary properties of [`BINARY`] the data files hold, in the order of the table: every one
/// but those of [`RUN_TIME`]. Their numbers in `property_names::BINARY` are their places here.
pub(crate) fn binary_order() -> Vec<&'static str> {
    BINARY
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| !RUN_TIME.contains(name))
        .collect()
}

/// The constant that holds the ranges of the binary property `name` in `binary_properties.rs`:
/// its own, named as the property in capitals, or that of another table.
pub(crate) fn binary_constant(name: &str) -> String {
    ELSEWHERE
        .iter()
        .find(|(other, _)| *other == name)
        .map_or_else(|| name.to_ascii_uppercase(), |(_, path)| (*path).to_owned())
}

/// Whether the binary property `name` has its own constant in `binary_properties.rs`.
pub(crate) fn binary_written_here(name: &str) -> bool {
    !ELSEWHERE.iter().any(|(other, _)| *other == name)
}

/// The fields of `data`, a data line without its comment, split at `;` and trimmed.
fn fields(data: &str) -> Vec<&str> {
    data.split(';').map(str::trim).collect()
}

/// The value names of `source`, a file of ranges: the second field of each data line, once each,
/// sorted.
pub(crate) fn value_names(ucd: &mut Ucd, source: &str) -> Result<BTreeSet<String>, String> {
    let name = ucd.source_name(source);
    let mut names = BTreeSet::new();
    for (number, data) in ucd.data_lines(source)? {
        let value = fields(data)
            .get(1)
            .copied()
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("{name}:{number}: not <code point or range> ; <value>"))?;
        if !names.contains(value) {
            names.insert(value.to_owned());
        }
    }
    if names.is_empty() {
        return Err(format!("{name}: no value"));
    }
    Ok(names)
}

/// One value of `PropertyValueAliases.txt`: its short name first, then its long name and its
/// further aliases, as the line gives them.
pub(crate) struct Value {
    pub(crate) names: Vec<String>,
}

impl Value {
    /// Its short name, the first field after the property.
    pub(crate) fn short(&self) -> &str {
        self.names.first().map(String::as_str).unwrap_or_default()
    }

    /// Its long name, the second field after the property.
    pub(crate) fn long(&self) -> &str {
        self.names.get(1).map(String::as_str).unwrap_or_default()
    }
}

/// The values of the property `property` (`gc` or `sc`) in `source`, a file of the form of
/// `PropertyValueAliases.txt` (`<property> ; <short> ; <long> [; <alias> ...]`), in the order of
/// the file. A line with fewer than two names, or a property with no line, is an error.
pub(crate) fn values(ucd: &mut Ucd, source: &str, property: &str) -> Result<Vec<Value>, String> {
    let name = ucd.source_name(source);
    let mut values = Vec::new();
    for (number, data) in ucd.data_lines(source)? {
        let fields = fields(data);
        if fields.first() != Some(&property) {
            continue;
        }
        let names: Vec<String> = fields
            .iter()
            .skip(1)
            .map(|field| (*field).to_owned())
            .collect();
        if names.len() < 2 || names.iter().any(String::is_empty) {
            return Err(format!(
                "{name}:{number}: not {property} ; <short> ; <long> [; <alias> ...]: {data:?}"
            ));
        }
        values.push(Value { names });
    }
    if values.is_empty() {
        return Err(format!("{name}: no value of {property}"));
    }
    Ok(values)
}

/// Every name and alias of ECMA-262's tables of properties that a data file holds is on one line
/// of `source`, a file of the form of `PropertyAliases.txt` (`<short> ; <long> [; <alias> ...]`):
/// the line whose fields hold the name holds each of its aliases. A name or alias with no line,
/// or on another line than its name, is an error. The properties of [`RUN_TIME`] are none of
/// that file's.
pub(crate) fn check_property_aliases(ucd: &mut Ucd, source: &str) -> Result<(), String> {
    let name = ucd.source_name(source);
    let lines: Vec<Vec<String>> = ucd
        .data_lines(source)?
        .into_iter()
        .map(|(_, data)| fields(data).into_iter().map(str::to_owned).collect())
        .collect();
    for (property, aliases) in NON_BINARY.iter().chain(BINARY) {
        if RUN_TIME.contains(property) {
            continue;
        }
        let line = lines
            .iter()
            .find(|fields| fields.iter().any(|field| field == property))
            .ok_or_else(|| format!("{name}: no line names the property {property}"))?;
        for alias in *aliases {
            if !line.iter().any(|field| field == alias) {
                return Err(format!(
                    "{name}: the line of {property} does not name its alias {alias}"
                ));
            }
        }
    }
    Ok(())
}

/// The values of General_Category in `source` (`DerivedGeneralCategory.txt`) with their merged
/// ranges, sorted by name: the order of the bits of a mask of categories. Every code point has
/// exactly one value, so the ranges of all of them are disjoint and cover U+0000 to U+10FFFF; a
/// file where they do not is an error.
pub(crate) fn categories(ucd: &mut Ucd, source: &str) -> Result<Vec<(String, Ranges)>, String> {
    let name = ucd.source_name(source);
    let names: Vec<String> = value_names(ucd, source)?.into_iter().collect();
    if names.len() > 32 {
        return Err(format!(
            "{name}: {} values of General_Category, more than the 32 bits of a mask",
            names.len()
        ));
    }
    let keys: Vec<&str> = names.iter().map(String::as_str).collect();
    let ranges = ucd.ranges(source, &keys)?;
    let categories: Vec<(String, Ranges)> = names.into_iter().zip(ranges).collect();
    let mut all: Vec<(u32, u32)> = categories
        .iter()
        .flat_map(|(_, ranges)| ranges.iter().copied())
        .collect();
    all.sort_unstable();
    let mut next = 0u32;
    for (first, last) in all {
        if first != next {
            return Err(format!(
                "{name}: the values of General_Category do not cover {next:04X} exactly once"
            ));
        }
        next = last.saturating_add(1);
    }
    if next != MAX_CODE_POINT + 1 {
        return Err(format!(
            "{name}: the values of General_Category end at {:04X}, not at 10FFFF",
            next.saturating_sub(1)
        ));
    }
    Ok(categories)
}

/// The mask of each value of General_Category and of each of its aliases, from the values of
/// `categories` (in the order of [`categories`]) and `values` (the `gc` lines of
/// `PropertyValueAliases.txt`): one bit for a value with code points, the bits of its members for
/// a group of [`GROUPS`]. A line that names neither, or a group whose member is no value, is an
/// error.
pub(crate) fn category_masks(
    categories: &[String],
    values: &[Value],
) -> Result<Vec<(String, u32)>, String> {
    let bit = |short: &str| -> Option<u32> {
        categories
            .iter()
            .position(|category| category == short)
            .and_then(|at| u32::try_from(at).ok())
            .map(|at| 1u32 << at)
    };
    let mut masks = Vec::new();
    for value in values {
        let short = value.short();
        let mask = match bit(short) {
            Some(mask) => mask,
            None => {
                let (_, members) = GROUPS
                    .iter()
                    .find(|(group, _)| *group == short)
                    .ok_or_else(|| {
                        format!("the value {short} of General_Category has no code point and is no group")
                    })?;
                let mut mask = 0;
                for member in *members {
                    mask |= bit(member).ok_or_else(|| {
                        format!("the member {member} of the group {short} is no value of General_Category")
                    })?;
                }
                mask
            }
        };
        for name in &value.names {
            masks.push((name.clone(), mask));
        }
    }
    for category in categories {
        if !values.iter().any(|value| value.short() == category) {
            return Err(format!(
                "the value {category} of General_Category has no line of PropertyValueAliases.txt"
            ));
        }
    }
    Ok(masks)
}

/// The ranges of each value of Script, in the order of `values` (the `sc` lines of
/// `PropertyValueAliases.txt`), from `source` (`Scripts.txt`), which names a value by its long
/// name. `Unknown` (`Zzzz`), which the file does not list, is every code point the file gives
/// no value; another value the file does not list has no code point. A value of the file that no
/// line of `values` names is an error.
pub(crate) fn scripts(
    ucd: &mut Ucd,
    source: &str,
    values: &[Value],
) -> Result<Vec<Ranges>, String> {
    let name = ucd.source_name(source);
    let present = value_names(ucd, source)?;
    for script in &present {
        if !values.iter().any(|value| value.long() == script) {
            return Err(format!(
                "{name}: the script {script} has no line of PropertyValueAliases.txt"
            ));
        }
    }
    let mut scripts = Vec::with_capacity(values.len());
    let mut unknown = None;
    for (at, value) in values.iter().enumerate() {
        if present.contains(value.long()) {
            let ranges = ucd.ranges(source, &[value.long()])?;
            scripts.push(ranges.into_iter().next().unwrap_or_default());
        } else {
            if value.short() == "Zzzz" {
                unknown = Some(at);
            }
            scripts.push(Vec::new());
        }
    }
    let at = unknown.ok_or_else(|| format!("{name}: no value Zzzz (Unknown) to complete"))?;
    let mut all: Vec<(u32, u32)> = scripts.iter().flatten().copied().collect();
    all.sort_unstable();
    let mut rest = Vec::new();
    let mut next = 0u32;
    for (first, last) in all {
        if first < next {
            return Err(format!("{name}: two scripts of {first:04X}"));
        }
        if first > next {
            rest.push((next, first - 1));
        }
        next = last.saturating_add(1);
    }
    if next <= MAX_CODE_POINT {
        rest.push((next, MAX_CODE_POINT));
    }
    if let Some(slot) = scripts.get_mut(at) {
        *slot = rest;
    }
    Ok(scripts)
}

/// The ranges of `ScriptExtensions.txt` (`source`) with the scripts of each, as indices into
/// `values` (the `sc` lines of `PropertyValueAliases.txt`; the file names a script by its short
/// name), each list sorted: sorted, disjoint ranges, adjacent ranges of one list joined. A short
/// name `values` does not have, or two lists for one code point, is an error.
pub(crate) fn script_extensions(
    ucd: &mut Ucd,
    source: &str,
    values: &[Value],
) -> Result<Vec<(u32, u32, Vec<u8>)>, String> {
    let name = ucd.source_name(source);
    let sets: Vec<String> = value_names(ucd, source)?.into_iter().collect();
    let keys: Vec<&str> = sets.iter().map(String::as_str).collect();
    let ranges = ucd.ranges(source, &keys)?;
    let mut rows = Vec::new();
    for (set, ranges) in sets.iter().zip(ranges) {
        let mut list = Vec::new();
        for short in set.split_whitespace() {
            let at = values
                .iter()
                .position(|value| value.short() == short)
                .and_then(|at| u8::try_from(at).ok())
                .ok_or_else(|| {
                    format!("{name}: the script {short} has no line of PropertyValueAliases.txt")
                })?;
            list.push(at);
        }
        list.sort_unstable();
        list.dedup();
        for (first, last) in ranges {
            rows.push((first, last, list.clone()));
        }
    }
    rows.sort_unstable_by_key(|(first, _, _)| *first);
    let mut joined: Vec<(u32, u32, Vec<u8>)> = Vec::with_capacity(rows.len());
    for (first, last, list) in rows {
        match joined.last_mut() {
            Some(previous) if previous.1 >= first => {
                return Err(format!("{name}: two lists of scripts for {first:04X}"));
            }
            Some(previous) if previous.1.saturating_add(1) == first && previous.2 == list => {
                previous.1 = last;
            }
            _ => joined.push((first, last, list)),
        }
    }
    Ok(joined)
}

/// The ranges of each binary property of [`binary_order`] that `binary_properties.rs` writes
/// itself ([`binary_written_here`]), from the first of `sources` that has the property as a
/// value: the property and its ranges. A property that none of them has is an error.
pub(crate) fn binary_properties(
    ucd: &mut Ucd,
    sources: &[&str],
) -> Result<Vec<(&'static str, Ranges)>, String> {
    let mut names: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for source in sources {
        names.insert(source, value_names(ucd, source)?);
    }
    let mut properties = Vec::new();
    for property in binary_order() {
        if !binary_written_here(property) {
            continue;
        }
        let source = sources
            .iter()
            .find(|source| {
                names
                    .get(**source)
                    .is_some_and(|names| names.contains(property))
            })
            .ok_or_else(|| {
                format!(
                    "the binary property {property} is in none of {}",
                    sources.join(", ")
                )
            })?;
        let ranges = ucd.ranges(source, &[property])?;
        properties.push((property, ranges.into_iter().next().unwrap_or_default()));
    }
    Ok(properties)
}

/// The width of a line rustfmt writes (its `max_width`).
const MAX_WIDTH: usize = 100;

/// The widest array literal rustfmt keeps on one line (its `array_width`).
const ARRAY_WIDTH: usize = 60;

/// The constant `name` of `ranges`, sorted, disjoint inclusive ranges `(first, last)`, as rustfmt
/// writes it: on one line where the array literal fits `array_width` and the line `max_width`;
/// else the array on the next line where it fits there; else one range per line.
pub(crate) fn ranges(name: &str, ranges: &[(u32, u32)]) -> String {
    let rows: Vec<String> = ranges
        .iter()
        .map(|(first, last)| format!("({first:#06X}, {last:#06X})"))
        .collect();
    let head = format!("pub(crate) const {name}: &[(u32, u32)] =");
    let array = format!("[{}]", rows.join(", "));
    if array.len() <= ARRAY_WIDTH {
        let line = format!("{head} &{array};");
        if line.len() <= MAX_WIDTH {
            return format!("{line}\n");
        }
        let next = format!("    &{array};");
        if next.len() <= MAX_WIDTH {
            return format!("{head}\n{next}\n");
        }
    }
    let mut text = format!("{head} &[\n");
    for row in &rows {
        text.push_str(&format!("    {row},\n"));
    }
    text.push_str("];\n");
    text
}

/// `items` as the body of an array literal of short items, as rustfmt writes it (its mixed
/// layout): as many to a line, each with its comma, as stay below `max_width` (a line of 100
/// columns is broken; measured with `cargo fmt --check` on `script_extensions.rs`), at the indent
/// `indent`.
pub(crate) fn packed(items: &[String], indent: &str) -> String {
    let mut text = String::new();
    let mut line = String::new();
    for item in items {
        let next = if line.is_empty() {
            format!("{indent}{item},")
        } else {
            format!("{line} {item},")
        };
        if next.len() >= MAX_WIDTH && !line.is_empty() {
            text.push_str(&line);
            text.push('\n');
            line = format!("{indent}{item},");
        } else {
            line = next;
        }
    }
    if !line.is_empty() {
        text.push_str(&line);
        text.push('\n');
    }
    text
}

/// The constant `name` of type `&[<row>]` whose items are short (an identifier or a number of at
/// most 10 characters), as rustfmt writes it: on one line where it fits, else packed.
pub(crate) fn short_items(name: &str, row: &str, items: &[String]) -> String {
    let head = format!("pub(crate) const {name}: &[{row}] = &");
    let array = format!("[{}]", items.join(", "));
    let line = format!("{head}{array};\n");
    if array.len() <= ARRAY_WIDTH && line.trim_end().len() <= MAX_WIDTH {
        return line;
    }
    format!("{head}[\n{}];\n", packed(items, "    "))
}
