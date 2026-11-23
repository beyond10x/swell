//! The properties of the property escapes `\p{...}` and `\P{...}` of a pattern under `u`
//! (`#sec-compiletocharset`, CharacterClassEscape :: `p{` UnicodePropertyValueExpression `}`):
//! the lookup of a name and a value as ECMA-262 admits them
//! (`#sec-runtime-semantics-unicodematchproperty-p`,
//! `#sec-runtime-semantics-unicodematchpropertyvalue-p-v`), the test of a code point and the
//! ranges of a property, and `ID_Start` and `ID_Continue` for the identifiers of 12.7
//! (`#sec-names-and-keywords`).
//!
//! A name matches only as written: the names and aliases of ECMA-262's two tables of properties
//! (`#table-nonbinary-unicode-properties`, `#table-binary-unicode-properties`) and the values and
//! aliases of General_Category and Script in `PropertyValueAliases.txt`; no loose matching, no
//! `Is` prefix. A lookup is a binary search of a sorted list of names, each comparison linear in
//! the name.
//!
//! The test of a code point ([`Property::contains`]) is a binary search over sorted, disjoint
//! ranges of a table: one for a value of General_Category, a script, a binary property and
//! `Assigned`; one per member for a group of General_Category, at most seven (`P`); for a value
//! of Script_Extensions one over the ranges `ScriptExtensions.txt` lists and one over the list of
//! scripts of the range found (at most 21 entries), or else one over the script's ranges. No test
//! copies a table or builds a union.

use crate::in_ranges;
use crate::tables::{
    binary_properties, general_category, property_names, script, script_extensions,
};

/// The largest code point.
const MAX_CODE_POINT: u32 = 0x10FFFF;

/// Every code point, the ranges of `Any`.
const ALL: &[(u32, u32)] = &[(0, MAX_CODE_POINT)];

/// U+0000 to U+007F, the ranges of `ASCII`.
const ASCII: &[(u32, u32)] = &[(0, 0x7F)];

/// A property, or a property and one of its values, that a property escape names: a set of code
/// points. Two names or aliases of one property or value give equal properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Property(Kind);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Kind {
    /// Values of General_Category: bit `i` is `general_category::VALUES[i]`; a group has the
    /// bits of its members.
    Categories(u32),
    /// A value of Script: its index in `script::VALUES`.
    Script(u8),
    /// A value of Script_Extensions: the index of its script in `script::VALUES`.
    Extensions(u8),
    /// A binary property: its index in `binary_properties::VALUES`.
    Binary(u8),
    Any,
    Ascii,
    Assigned,
}

/// The value of `name` in `list`, sorted by name as bytes: a binary search, each comparison
/// linear in `name`. `name` is code units; a name of the lists is ASCII, so a code unit outside
/// ASCII matches none.
fn find<T: Copy>(list: &[(&str, T)], name: &[u16]) -> Option<T> {
    list.binary_search_by(|(entry, _)| entry.bytes().map(u16::from).cmp(name.iter().copied()))
        .ok()
        .and_then(|at| list.get(at))
        .map(|&(_, value)| value)
}

/// The property of `\p{name}` (LoneUnicodePropertyNameOrValue): a value or alias of
/// General_Category, or a binary property of ECMA-262's table by its name or alias; `None` for
/// any other name (`#sec-patterns-static-semantics-early-errors`). `name` is the code units of
/// the pattern.
pub fn lone_property(name: &[u16]) -> Option<Property> {
    if let Some(mask) = find(property_names::CATEGORIES, name) {
        return Some(Property(Kind::Categories(mask)));
    }
    let number = find(property_names::BINARY, name)?;
    Some(Property(match number {
        property_names::ANY => Kind::Any,
        property_names::ASCII => Kind::Ascii,
        property_names::ASSIGNED => Kind::Assigned,
        number => Kind::Binary(number),
    }))
}

/// The property of `\p{name=value}`: `name` a name or alias of General_Category, Script or
/// Script_Extensions, and `value` a value or alias of General_Category for the first, of Script
/// for the other two; `None` for any other pair (`#sec-patterns-static-semantics-early-errors`).
/// Both are the code units of the pattern.
pub fn property_value(name: &[u16], value: &[u16]) -> Option<Property> {
    Some(Property(match find(property_names::PROPERTIES, name)? {
        0 => Kind::Categories(find(property_names::CATEGORIES, value)?),
        1 => Kind::Script(find(property_names::SCRIPTS, value)?),
        2 => Kind::Extensions(find(property_names::SCRIPTS, value)?),
        _ => return None,
    }))
}

/// The ranges of the value `index` of `values`, or none.
fn table(values: &'static [&'static [(u32, u32)]], index: u8) -> &'static [(u32, u32)] {
    values.get(usize::from(index)).copied().unwrap_or(&[])
}

/// The row of `script_extensions::RANGES` that holds `c`, and its list of scripts.
fn extensions_of(c: u32) -> Option<&'static [u8]> {
    let rows = script_extensions::RANGES;
    let at = rows.partition_point(|&(_, last, _, _)| last < c);
    let &(_, _, start, len) = rows.get(at).filter(|row| row.0 <= c)?;
    let start = usize::from(start);
    script_extensions::LISTS.get(start..start.saturating_add(usize::from(len)))
}

impl Property {
    /// Whether the code point `c` has the property: a bounded number of binary searches (the
    /// module's head). Any `u32` may be asked; a value above U+10FFFF has none.
    pub fn contains(self, c: u32) -> bool {
        match self.0 {
            Kind::Categories(mask) => {
                let mut bits = mask;
                while bits != 0 {
                    let index = bits.trailing_zeros();
                    let ranges = general_category::VALUES
                        .get(index as usize)
                        .copied()
                        .unwrap_or(&[]);
                    if in_ranges(ranges, c) {
                        return true;
                    }
                    bits &= bits - 1;
                }
                false
            }
            Kind::Script(index) => in_ranges(table(script::VALUES, index), c),
            Kind::Extensions(index) => match extensions_of(c) {
                Some(list) => list.binary_search(&index).is_ok(),
                None => in_ranges(table(script::VALUES, index), c),
            },
            Kind::Binary(index) => in_ranges(table(binary_properties::VALUES, index), c),
            Kind::Any => c <= MAX_CODE_POINT,
            Kind::Ascii => c <= 0x7F,
            Kind::Assigned => c <= MAX_CODE_POINT && !in_ranges(general_category::CN, c),
        }
    }

    /// The code points of the property as sorted, disjoint inclusive ranges, two of them never
    /// adjacent: a group of General_Category the union of its members, Script_Extensions by its
    /// rule (the code points whose extensions name the script, and those of the script that
    /// `ScriptExtensions.txt` does not list). The iterator walks the tables and allocates
    /// nothing.
    pub fn ranges(self) -> PropertyRanges {
        let mut sources = [Source::Empty; SOURCES];
        let mut put = |at: usize, source: Source| {
            if let Some(slot) = sources.get_mut(at) {
                *slot = source;
            }
        };
        match self.0 {
            Kind::Categories(mask) => {
                for (at, ranges) in general_category::VALUES.iter().enumerate() {
                    if at < 32 && mask & (1 << at) != 0 {
                        put(at, Source::Slice { ranges, at: 0 });
                    }
                }
            }
            Kind::Script(index) => put(
                0,
                Source::Slice {
                    ranges: table(script::VALUES, index),
                    at: 0,
                },
            ),
            Kind::Extensions(index) => {
                put(
                    0,
                    Source::Named {
                        script: index,
                        at: 0,
                    },
                );
                put(
                    1,
                    Source::Unlisted {
                        ranges: table(script::VALUES, index),
                        at: 0,
                        row: 0,
                        from: 0,
                    },
                );
            }
            Kind::Binary(index) => put(
                0,
                Source::Slice {
                    ranges: table(binary_properties::VALUES, index),
                    at: 0,
                },
            ),
            Kind::Any => put(0, Source::Slice { ranges: ALL, at: 0 }),
            Kind::Ascii => put(
                0,
                Source::Slice {
                    ranges: ASCII,
                    at: 0,
                },
            ),
            Kind::Assigned => put(
                0,
                Source::Complement {
                    ranges: general_category::CN,
                    at: 0,
                    next: 0,
                    done: false,
                },
            ),
        }
        PropertyRanges {
            sources,
            heads: [None; SOURCES],
            started: false,
        }
    }
}

/// The most sources a union of [`PropertyRanges`] reads: one per bit of a mask of values of
/// General_Category.
const SOURCES: usize = 32;

/// One sorted, disjoint stream of ranges that [`PropertyRanges`] joins.
#[derive(Clone, Copy, Debug)]
enum Source {
    Empty,
    /// `ranges` from `at`.
    Slice {
        ranges: &'static [(u32, u32)],
        at: usize,
    },
    /// The ranges of `script_extensions::RANGES` from `at` whose list names `script`.
    Named {
        script: u8,
        at: usize,
    },
    /// `ranges` from `at`, the part below `from` of its range `at` taken, without the code points
    /// `script_extensions::RANGES` lists (read from its row `row`).
    Unlisted {
        ranges: &'static [(u32, u32)],
        at: usize,
        row: usize,
        from: u32,
    },
    /// The code points U+0000 to U+10FFFF that `ranges` does not hold, from `next` on.
    Complement {
        ranges: &'static [(u32, u32)],
        at: usize,
        next: u32,
        done: bool,
    },
}

impl Source {
    fn next(&mut self) -> Option<(u32, u32)> {
        match self {
            Source::Empty => None,
            Source::Slice { ranges, at } => {
                let range = ranges.get(*at).copied();
                *at += 1;
                range
            }
            Source::Named { script, at } => {
                let rows = script_extensions::RANGES;
                while let Some(&(first, last, start, len)) = rows.get(*at) {
                    *at += 1;
                    let start = usize::from(start);
                    let named = script_extensions::LISTS
                        .get(start..start.saturating_add(usize::from(len)))
                        .is_some_and(|list| list.binary_search(script).is_ok());
                    if named {
                        return Some((first, last));
                    }
                }
                None
            }
            Source::Unlisted {
                ranges,
                at,
                row,
                from,
            } => {
                let rows = script_extensions::RANGES;
                loop {
                    let &(first, last) = ranges.get(*at)?;
                    let low = first.max(*from);
                    if low > last {
                        *at += 1;
                        continue;
                    }
                    while rows.get(*row).is_some_and(|&(_, end, _, _)| end < low) {
                        *row += 1;
                    }
                    match rows.get(*row) {
                        Some(&(start, end, _, _)) if start <= low => {
                            *from = end.saturating_add(1);
                        }
                        Some(&(start, _, _, _)) if start <= last => {
                            *from = start;
                            return Some((low, start - 1));
                        }
                        _ => {
                            *at += 1;
                            return Some((low, last));
                        }
                    }
                }
            }
            Source::Complement {
                ranges,
                at,
                next,
                done,
            } => {
                while !*done {
                    match ranges.get(*at) {
                        Some(&(first, last)) => {
                            *at += 1;
                            let gap = (*next < first).then(|| (*next, first - 1));
                            *next = last.saturating_add(1);
                            if gap.is_some() {
                                return gap;
                            }
                        }
                        None => {
                            *done = true;
                            if *next <= MAX_CODE_POINT {
                                return Some((*next, MAX_CODE_POINT));
                            }
                        }
                    }
                }
                None
            }
        }
    }
}

/// The ranges of a [`Property`] ([`Property::ranges`]): sorted, disjoint inclusive ranges
/// `(first, last)`, two of them never adjacent.
#[derive(Clone, Debug)]
pub struct PropertyRanges {
    sources: [Source; SOURCES],
    /// The next range of each source, read ahead.
    heads: [Option<(u32, u32)>; SOURCES],
    started: bool,
}

impl Iterator for PropertyRanges {
    type Item = (u32, u32);

    fn next(&mut self) -> Option<(u32, u32)> {
        if !self.started {
            self.started = true;
            for (head, source) in self.heads.iter_mut().zip(self.sources.iter_mut()) {
                *head = source.next();
            }
        }
        let (at, mut current) = self
            .heads
            .iter()
            .enumerate()
            .filter_map(|(at, head)| head.map(|range| (at, range)))
            .min_by_key(|&(_, (first, _))| first)?;
        self.advance(at);
        loop {
            let mut grown = false;
            for at in 0..SOURCES {
                while let Some(Some((first, last))) = self.heads.get(at).copied()
                    && first <= current.1.saturating_add(1)
                {
                    current.1 = current.1.max(last);
                    self.advance(at);
                    grown = true;
                }
            }
            if !grown {
                return Some(current);
            }
        }
    }
}

impl PropertyRanges {
    /// The head of source `at` taken: the next range of the source read ahead.
    fn advance(&mut self, at: usize) {
        if let (Some(head), Some(source)) = (self.heads.get_mut(at), self.sources.get_mut(at)) {
            *head = source.next();
        }
    }
}

/// Whether `c` has the property `ID_Start` (`DerivedCoreProperties.txt`), which
/// IdentifierStartChar reads (`#sec-names-and-keywords`, UAX #31). Any `u32` may be asked; a lone
/// surrogate and a value above U+10FFFF have none.
pub fn is_id_start(c: u32) -> bool {
    in_ranges(binary_properties::ID_START, c)
}

/// Whether `c` has the property `ID_Continue` (`DerivedCoreProperties.txt`), which
/// IdentifierPartChar reads (`#sec-names-and-keywords`, UAX #31). Any `u32` may be asked; a lone
/// surrogate and a value above U+10FFFF have none.
pub fn is_id_continue(c: u32) -> bool {
    in_ranges(binary_properties::ID_CONTINUE, c)
}
