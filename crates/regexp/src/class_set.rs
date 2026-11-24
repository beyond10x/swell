//! The classes of the flag `v` in a program (`#sec-compiletocharset` with [+UnicodeSetsMode],
//! `#sec-compileatom`): a class is kept as the pattern writes it, its operations in postfix
//! order (`ast::ClassSetOp`, made [`SetOp`] by the compile step, which copies its characters
//! and ranges, sorted and joined, and its strings of `\q{...}`), and evaluated when it is run
//! for each candidate at the position. No set is built and no table of `swell-unicode` is
//! copied: a property, of code points or of strings, is named and looked up when the class is
//! run (`docs/design/engine.md`, decision 7, "What one step of the matcher is").
//!
//! A candidate is what the class may match at the position, read in the direction of the match:
//! the character there, each string of `\q{...}` found there, and each sequence of a property of
//! strings found there, at most [`StringProperty::longest`] code points; all of them start at
//! the position, so a candidate is known by its count of code points. The class holds a
//! candidate when its operations, evaluated on it, answer true: an operand of characters (its
//! ranges, `\s`, a property) holds a candidate of one character, a string a candidate of its own
//! code points, a property of strings a character or a sequence it has; a union is either, an
//! intersection both, a subtraction the first and not the second, a complement a character the
//! set does not hold (CharacterComplement holds characters only).
//!
//! Under `i` the operands are MaybeSimpleCaseFolding of what they name
//! (`#sec-maybesimplecasefolding`): a character `c` is held by an operand when one of the
//! characters of its canonical value is (at most `swell_unicode::MOST_CASE_FOLD_EQUIVALENTS`), a
//! string when it has the simple case folding of the candidate at each code point, a sequence by
//! [`StringProperty::contains_sequence_folded`]. The complement is taken after the folding, over
//! the characters that are their own folding (CharacterComplement, AllCharacters under `v` with
//! `i`): `\P{Lu}` holds `c` when no character of the canonical value of `c` is in `Lu`, where
//! under `u` with `i` the complement is taken first and the canonical values compared after
//! (`#sec-runtime-semantics-charactersetmatcher-abstract-operation`).
//!
//! What one test of a class costs, in steps (the matcher charges them before the work): a
//! class with no string and no property of strings one step per operand (an operand of
//! characters, `\s`, a property, each a bounded number of binary searches; a union, an
//! intersection, a subtraction or a complement takes none, and there are fewer of them than
//! operands since a complement taken twice is none), the instruction's step the first. A class
//! with `n` operands, the strings `s_1 .. s_S` of `\q{...}` and the properties of strings
//! `p_1 .. p_P`: one step per code point of each string compared (one for the empty string), `L_j
//! - 1` steps for the lookups of the sequences of `p_j` of 2 to `L_j` code points (`L_j` its
//! longest sequence, which the generator states; one lookup a bounded number of binary
//! searches), and `n` steps per candidate evaluated, longest first until one is held:
//! `Σ max(1, |s_i|) + Σ (L_j - 1) + n · C` with `C`, the candidates at the position, at most
//! `S + Σ (L_j - 1) + 1`, the instruction's step the first of them.

use swell_unicode::{LONGEST_STRING, MOST_CASE_FOLD_EQUIVALENTS, Property, StringProperty};

use crate::canonicalize::equivalents;
use crate::program::{Program, is_space};

/// One operation of a class under `v` in a program, in postfix order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetOp {
    /// The ranges `len` from `start` of [`Program::ranges`], sorted and disjoint; with
    /// `negated` their complement. `\d` and `\w` are ranges too.
    Ranges {
        start: u32,
        len: u32,
        negated: bool,
    },
    /// `\s`, or `\S`.
    Space {
        negated: bool,
    },
    /// `\p{...}`, or `\P{...}`.
    Property {
        property: Property,
        negated: bool,
    },
    /// `\p{...}` of a property of strings.
    Strings(StringProperty),
    /// A string of `\q{...}`: the code points `len` from `start` of [`Program::points`], none
    /// or two or more.
    String {
        start: u32,
        len: u32,
    },
    Union,
    Intersection,
    Subtraction,
    Complement,
}

impl SetOp {
    /// Whether it is an operand, which pushes a set.
    pub(crate) fn is_operand(self) -> bool {
        !matches!(
            self,
            SetOp::Union | SetOp::Intersection | SetOp::Subtraction | SetOp::Complement
        )
    }
}

/// A class under `v`: its operations `len` from `start` of [`Program::set_ops`], its count of
/// operands, the steps of its tries (one per code point of each string of `\q{...}`, at least
/// one, and `L - 1` per property of strings), whether it may hold a string, and `fold` under
/// `i`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SetClass {
    pub(crate) start: u32,
    pub(crate) len: u32,
    pub(crate) operands: u32,
    pub(crate) tries: u32,
    pub(crate) strings: bool,
    pub(crate) fold: bool,
}

/// The most of any class of a program under `v`: the sets on its stack of evaluation, its
/// strings of `\q{...}`, and its candidates at one position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct SetBounds {
    pub(crate) depth: u32,
    pub(crate) strings: u32,
    pub(crate) candidates: u32,
}

/// The characters read from a position in the direction of the match, at most
/// [`LONGEST_STRING`], with the code units each prefix of them spans: the sequences of the
/// properties of strings are looked up among them.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Window {
    pub(crate) points: [u32; LONGEST_STRING],
    pub(crate) units: [usize; LONGEST_STRING],
    pub(crate) len: usize,
    pub(crate) back: bool,
}

impl Window {
    /// The first `k` characters as the subject has them, in its order: read backward, the last
    /// read comes first.
    pub(crate) fn sequence(&self, k: usize) -> Option<[u32; LONGEST_STRING]> {
        if k > self.len {
            return None;
        }
        let mut out = [0u32; LONGEST_STRING];
        for i in 0..k {
            let from = if self.back { k - 1 - i } else { i };
            if let (Some(slot), Some(&c)) = (out.get_mut(i), self.points.get(from)) {
                *slot = c;
            }
        }
        Some(out)
    }

    /// The code units the first `k` characters span.
    pub(crate) fn units_of(&self, k: usize) -> usize {
        match k.checked_sub(1) {
            Some(last) => self.units.get(last).copied().unwrap_or(0),
            None => 0,
        }
    }
}

/// Whether the sequence of `k` code points of `window`, two or more, is one of `property`.
pub(crate) fn sequence_in(property: StringProperty, window: &Window, k: usize, fold: bool) -> bool {
    let Some(points) = window.sequence(k) else {
        return false;
    };
    let Some(sequence) = points.get(..k) else {
        return false;
    };
    if fold {
        property.contains_sequence_folded(sequence)
    } else {
        property.contains_sequence(sequence)
    }
}

/// Whether `c` lies in one of `ranges`, sorted and disjoint.
fn in_ranges(ranges: &[(u32, u32)], c: u32) -> bool {
    ranges
        .binary_search_by(|&(first, last)| {
            if last < c {
                std::cmp::Ordering::Less
            } else if first > c {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Whether the class `class` holds the candidate of `k` code points of `window` (`k` 0 for the
/// empty string, 1 for the character): its operations evaluated on `stack`, which holds as many
/// sets as the class needs ([`SetBounds::depth`]) and is left empty. `matched` tells for each
/// string of `\q{...}` of the class, in order, whether it was found at the position; a string
/// holds the candidate of its count of code points then.
pub(crate) fn holds(
    program: &Program,
    class: &SetClass,
    k: usize,
    window: &Window,
    matched: &[bool],
    stack: &mut Vec<bool>,
) -> bool {
    stack.clear();
    let start = class.start as usize;
    let ops = program
        .set_ops
        .get(start..start.saturating_add(class.len as usize))
        .unwrap_or(&[]);
    // The characters a single-character operand is tested on: `c`, or under `i` those of its
    // canonical value.
    let mut chars = [0u32; MOST_CASE_FOLD_EQUIVALENTS];
    let mut count = 0usize;
    if k == 1
        && let Some(&c) = window.points.first()
    {
        if class.fold {
            for e in equivalents(c, true) {
                if let Some(slot) = chars.get_mut(count) {
                    *slot = e;
                    count += 1;
                }
            }
        } else if let Some(slot) = chars.first_mut() {
            *slot = c;
            count = 1;
        }
    }
    let tested = chars.get(..count).unwrap_or(&[]);
    let mut string = 0usize;
    for &op in ops {
        let value = match op {
            SetOp::Ranges {
                start,
                len,
                negated,
            } => {
                let start = start as usize;
                let own = program
                    .ranges
                    .get(start..start.saturating_add(len as usize))
                    .unwrap_or(&[]);
                k == 1 && tested.iter().any(|&c| in_ranges(own, c)) != negated
            }
            SetOp::Space { negated } => k == 1 && tested.iter().any(|&c| is_space(c)) != negated,
            SetOp::Property { property, negated } => {
                k == 1 && tested.iter().any(|&c| property.contains(c)) != negated
            }
            SetOp::Strings(property) => match k {
                0 => false,
                1 => tested.iter().any(|&c| property.contains(c)),
                _ => sequence_in(property, window, k, class.fold),
            },
            SetOp::String { len, .. } => {
                let found = matched.get(string).copied().unwrap_or(false);
                string += 1;
                found && len as usize == k
            }
            SetOp::Union | SetOp::Intersection | SetOp::Subtraction => {
                let b = stack.pop().unwrap_or(false);
                let a = stack.pop().unwrap_or(false);
                match op {
                    SetOp::Union => a || b,
                    SetOp::Intersection => a && b,
                    _ => a && !b,
                }
            }
            SetOp::Complement => {
                let a = stack.pop().unwrap_or(false);
                k == 1 && !a
            }
        };
        if stack.len() < stack.capacity() {
            stack.push(value);
        }
    }
    stack.pop().unwrap_or(false)
}
