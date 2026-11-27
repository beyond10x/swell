//! The program CompilePattern (`#sec-compilepattern`) makes of a pattern's tree, and the
//! matcher runs (`docs/design/engine.md`, decision 7): one vector of instructions, with the
//! ranges and the property escapes of the classes, the operations and strings of the classes
//! under `v`, the group numbers a named backreference names and the group names in vectors
//! beside it. Every vector was charged through the [`Budget`] before it grew;
//! [`Program::heap_bytes`] is what the budget holds for the program, and [`Program::release`]
//! drops it and credits that.
//!
//! What the matcher keeps while it runs one try (`matcher.rs`): the position, a code unit
//! index into the subject; the captures, two slots per group (its start and its end, or
//! unset), group 0 the match; one counter per quantifier (the iterations run and the
//! position the current one started at); one register per lookaround (the height of the
//! backtrack stack when it was entered); and the backtrack stack, whose entries resume an
//! instruction at a position (under `v` a class with strings below a length too) or restore a
//! slot or a counter. Each instruction below says what
//! it does to that state; one that fails hands control to the backtrack stack.

use alloc::vec::Vec;
use swell_unicode::Property;

use crate::ast::Name;
use crate::budget::{Budget, capacity_bytes};
use crate::canonicalize::equivalents;
use crate::class_set::{SetBounds, SetClass, SetOp};
use crate::flags::Flags;

/// The value of a jump target or a register not yet known while the program is compiled.
pub(crate) const UNPATCHED: u32 = u32::MAX;

/// The largest code point; a class escape's complement reaches it.
pub(crate) const MAX_CODE_POINT: u32 = 0x10FFFF;

/// One instruction. `back` is set inside a lookbehind (`#sec-compileatom`, direction
/// backward): a character is read before the position, and the position moves left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inst {
    /// The code unit `unit` at the position: a character without `u`, and under `u` a code
    /// point below U+10000 that is no surrogate, so that reading one code unit reads it whole.
    /// Moves past it; fails otherwise.
    Unit { unit: u16, back: bool },
    /// Under `u`, the code point `point` (a surrogate pair is one code point, a lone surrogate
    /// one code unit). Moves past it; fails otherwise.
    Point { point: u32, back: bool },
    /// Under `i` (the flag or a modifier), a character whose canonical value
    /// (`canonicalize.rs`) is `canonical`: a code unit without `u`, a code point under `u`.
    /// Moves past it; fails otherwise. A character with no other of its canonical value is
    /// [`Inst::Unit`] or [`Inst::Point`] under `i` too.
    Fold { canonical: u32, back: bool },
    /// `.`: any character but a LineTerminator, or any character with `dot_all`
    /// (`#sec-compileatom`, Atom :: `.`).
    Any { dot_all: bool, back: bool },
    /// A character of the class numbered `class` of [`Program::classes`].
    Class { class: u32, back: bool },
    /// Under `v`, the class numbered `class` of [`Program::set_classes`] (`class_set.rs`): the
    /// longest of its candidates at the position that the class holds, a character or a
    /// string, and the position past it; where a shorter candidate is left, it pushes an entry
    /// that resumes this instruction below the length taken (the alternatives of CompileAtom
    /// for a CharSet with strings, longest first). Fails when none is held.
    SetClass { class: u32, back: bool },
    /// `^`: the position is 0, or with `multiline` follows a LineTerminator.
    LineStart { multiline: bool },
    /// `$`: the position is the subject's end, or with `multiline` precedes a LineTerminator.
    LineEnd { multiline: bool },
    /// `\b` (`negated` false) or `\B`: whether the characters on the two sides of the position
    /// differ in IsWordChar (`#sec-runtime-semantics-iswordchar-abstract-operation`). `fold`
    /// under `u` or `v` with `i`, where WordCharacters holds U+017F and U+212A too
    /// (`#sec-wordcharacters`).
    WordBoundary { negated: bool, fold: bool },
    /// A choice: pushes an entry that resumes `second` at the position, and goes on at
    /// `first`. Alternatives in order, and the two orders of a quantifier.
    Split { first: u32, second: u32 },
    /// Goes on at `to`.
    Jump { to: u32 },
    /// Sets the capture slot `slot` (twice the group number for its start, plus one for its
    /// end) to the position; pushes an entry that restores the old value.
    Save { slot: u32 },
    /// Unsets the captures of the groups `first` to `last`, the groups inside a quantified
    /// atom, at each iteration (`#sec-runtime-semantics-repeatmatcher-abstract-operation`,
    /// step 4); pushes an entry per slot it changes that restores it.
    Clear { first: u32, last: u32 },
    /// Sets the counter `counter` to no iteration, started at the position; pushes an entry
    /// that restores its old value.
    RepeatInit { counter: u32 },
    /// The head of a quantifier's loop, RepeatMatcher with `min` and `max` lowered by the
    /// iterations run: at `max` iterations goes on at `exit`; below `min` runs the atom (the
    /// next instruction) and pushes nothing; otherwise greedy pushes an entry for `exit` and
    /// runs the atom, lazy pushes an entry for the atom and goes on at `exit`.
    Repeat {
        counter: u32,
        min: u32,
        max: Option<u32>,
        greedy: bool,
        exit: u32,
    },
    /// The end of one iteration: fails when the iteration started with `min` reached and
    /// matched the empty String (RepeatMatcher, step 2.b); else counts it, records the
    /// position as the start of the next, pushes an entry that restores the counter, and goes
    /// on at `head`.
    RepeatTail { counter: u32, min: u32, head: u32 },
    /// `\n`: the code units the group captured, read at the position in the direction; an
    /// unset group matches the empty String (`#sec-backreference-matcher`). Under `i`
    /// (`fold`) the canonical values of the characters are compared.
    Backref { group: u32, back: bool, fold: bool },
    /// `\k<name>` of a name several groups have: the groups `len` from `start` of
    /// [`Program::named`]; the one of them that is set, or the empty String.
    NamedBackref {
        start: u32,
        len: u32,
        back: bool,
        fold: bool,
    },
    /// The entry of a lookaround: records the height of the backtrack stack in the register
    /// `look`, and pushes a barrier entry with the position; a negative lookaround's barrier
    /// goes on at `end` when its body fails.
    LookStart { look: u32, negative: bool, end: u32 },
    /// The exit of a lookaround whose body matched. Positive: the entries above its barrier
    /// that resume an instruction are dropped and those that restore a slot or a counter kept
    /// (the lookaround is atomic and keeps its captures), the barrier is removed and the
    /// position set back to the barrier's. Negative: every entry down to the barrier is taken
    /// and its restore done, and the lookaround fails.
    LookEnd { look: u32, negative: bool },
    /// The end of the match: the end of group 0 is the position.
    Match,
}

/// A class: the ranges `len` from `start` of [`Program::ranges`], sorted and disjoint, with
/// `\s` and `\S` as two predicates beside them, and the property escapes `properties_len` from
/// `properties` of [`Program::properties`], each tested by a bounded number of binary searches
/// over a table of `swell-unicode` and none copied into the program; `negated` for `[^...]`;
/// `fold` under `i` (the flag or a modifier).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClassSet {
    pub(crate) start: u32,
    pub(crate) len: u32,
    pub(crate) properties: u32,
    pub(crate) properties_len: u32,
    pub(crate) negated: bool,
    pub(crate) space: bool,
    pub(crate) not_space: bool,
    pub(crate) fold: bool,
}

impl ClassSet {
    /// Whether the character `c` is in the class
    /// (`#sec-runtime-semantics-charactersetmatcher-abstract-operation`): whether one of its
    /// items holds `c`, or under `i` (`fold`) one of the characters of the canonical value of
    /// `c` ([`equivalents`], at most `swell_unicode::MOST_CASE_FOLD_EQUIVALENTS`), and then
    /// `negated` applied. No closure of the class is built: the ranges, `\s`, `\S` and each
    /// property escape are tested against those characters when the class is run.
    pub(crate) fn contains(&self, program: &Program, c: u32) -> bool {
        let found = if self.fold {
            equivalents(c, program.flags.either_unicode()).any(|e| self.holds(program, e))
        } else {
            self.holds(program, c)
        };
        found != self.negated
    }

    /// Whether an item of the class holds `c`: a binary search of its ranges, and one test per
    /// property escape until one holds.
    fn holds(&self, program: &Program, c: u32) -> bool {
        let start = self.start as usize;
        let own = program
            .ranges
            .get(start..start.saturating_add(self.len as usize))
            .unwrap_or(&[]);
        let in_ranges = own
            .binary_search_by(|&(first, last)| {
                if last < c {
                    core::cmp::Ordering::Less
                } else if first > c {
                    core::cmp::Ordering::Greater
                } else {
                    core::cmp::Ordering::Equal
                }
            })
            .is_ok();
        in_ranges
            || (self.space && is_space(c))
            || (self.not_space && !is_space(c))
            || self
                .properties_of(program)
                .iter()
                .any(|&(property, negated)| property.contains(c) != negated)
    }

    /// [`ClassSet::contains`] of `c`, with an inclusive range `(first, last)` of characters
    /// around `c`, `c` among them, for each of which the class answers the same: where its
    /// ranges and each of its property escapes ([`Property::contains_around`]) answer alike. The
    /// same binary searches as `contains`. A class under `i` or with `\s` or `\S` answers for
    /// `c` alone. The matcher keeps the answer with its range and answers a character in the
    /// range from it (`matcher.rs`).
    pub(crate) fn contains_around(&self, program: &Program, c: u32) -> (bool, u32, u32) {
        if self.fold || self.space || self.not_space {
            return (self.contains(program, c), c, c);
        }
        let start = self.start as usize;
        let own = program
            .ranges
            .get(start..start.saturating_add(self.len as usize))
            .unwrap_or(&[]);
        let (mut held, mut first, mut last) = ranges_around(own, c);
        if !held {
            for &(property, negated) in self.properties_of(program) {
                let (has, from, to) = property.contains_around(c);
                if has != negated {
                    (held, first, last) = (true, from, to);
                    break;
                }
                first = first.max(from);
                last = last.min(to);
            }
        }
        (held != self.negated, first, last)
    }

    /// The property escapes of the class, each with `true` for `\P{...}`.
    pub(crate) fn properties_of<'p>(&self, program: &'p Program) -> &'p [(Property, bool)] {
        let start = self.properties as usize;
        program
            .properties
            .get(start..start.saturating_add(self.properties_len as usize))
            .unwrap_or(&[])
    }
}

/// Whether `c` lies in one of `ranges`, sorted and disjoint, with the characters around `c` that
/// answer the same: the range that holds `c`, or the gap between the two ranges beside it (from
/// 0 before the first, to `u32::MAX` after the last). One binary search.
fn ranges_around(ranges: &[(u32, u32)], c: u32) -> (bool, u32, u32) {
    let at = ranges.partition_point(|&(_, last)| last < c);
    let next = ranges.get(at);
    if let Some(&(first, last)) = next.filter(|&&(first, _)| first <= c) {
        return (true, first, last);
    }
    let first = at
        .checked_sub(1)
        .and_then(|before| ranges.get(before))
        .map_or(0, |&(_, last)| last.saturating_add(1));
    let last = next.map_or(u32::MAX, |&(first, _)| first.saturating_sub(1));
    (false, first, last)
}

/// WhiteSpace or LineTerminator (`#sec-white-space`, `#sec-line-terminators`), the characters
/// of `\s` (`#sec-compiletocharset`, CharacterClassEscape :: `s`).
pub(crate) fn is_space(c: u32) -> bool {
    matches!(c, 0x09 | 0x0B | 0x0C | 0xFEFF)
        || is_line_terminator(c)
        || swell_unicode::is_space_separator(c)
}

/// LineTerminator (`#sec-line-terminators`): LF, CR, LS and PS.
pub(crate) fn is_line_terminator(c: u32) -> bool {
    matches!(c, 0x0A | 0x0D | 0x2028 | 0x2029)
}

/// A character of WordCharacters (`#sec-wordcharacters`): the 63 of `[A-Za-z0-9_]`, and with
/// `fold` (under `u` or `v` with `i`) U+017F and U+212A, whose simple case foldings are `s`
/// and `k`. Without `u` and `v` Canonicalize maps no character of 128 or above into ASCII,
/// so `i` adds none there.
pub(crate) fn is_word(c: u32, fold: bool) -> bool {
    matches!(c, 0x30..=0x39 | 0x41..=0x5A | 0x5F | 0x61..=0x7A)
        || (fold && matches!(c, 0x017F | 0x212A))
}

/// The program of a pattern.
#[derive(Debug)]
pub struct Program {
    pub(crate) code: Vec<Inst>,
    pub(crate) ranges: Vec<(u32, u32)>,
    pub(crate) classes: Vec<ClassSet>,
    pub(crate) properties: Vec<(Property, bool)>,
    pub(crate) set_classes: Vec<SetClass>,
    pub(crate) set_ops: Vec<SetOp>,
    pub(crate) points: Vec<u32>,
    /// The most of any class under `v`: sets on its stack of evaluation, strings of `\q{...}`,
    /// and candidates at one position. The matcher reserves its lists for them before a match.
    pub(crate) set_bounds: SetBounds,
    pub(crate) named: Vec<u32>,
    pub(crate) names: Vec<(u32, Name)>,
    pub(crate) text: Vec<u8>,
    pub(crate) groups: u32,
    pub(crate) counters: u32,
    pub(crate) looks: u32,
    pub(crate) flags: Flags,
    pub(crate) charged: usize,
}

impl Program {
    /// The count of instructions.
    pub fn len(&self) -> usize {
        self.code.len()
    }

    /// Whether it has no instruction; a program has at least the end of the match.
    pub fn is_empty(&self) -> bool {
        self.code.is_empty()
    }

    /// The count of capturing groups, group 0 not counted.
    pub fn group_count(&self) -> u32 {
        self.groups
    }

    /// The flags of the pattern.
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// Each named group, by its number from the left, with its name; a name declared in
    /// different alternatives appears once per group.
    pub fn group_names(&self) -> impl Iterator<Item = (u32, &str)> {
        self.names.iter().map(|&(number, name)| {
            let start = name.start as usize;
            let text = self
                .text
                .get(start..start.saturating_add(name.len as usize))
                .and_then(|bytes| core::str::from_utf8(bytes).ok())
                .unwrap_or("");
            (number, text)
        })
    }

    /// The bytes the budget holds for this program, which its owner credits when it drops it.
    pub fn heap_bytes(&self) -> usize {
        self.charged
    }

    /// The bytes of the program's vectors by their capacities, summed from the vectors.
    pub fn capacity_bytes(&self) -> usize {
        capacity_bytes(&self.code)
            .saturating_add(capacity_bytes(&self.ranges))
            .saturating_add(capacity_bytes(&self.classes))
            .saturating_add(capacity_bytes(&self.properties))
            .saturating_add(capacity_bytes(&self.set_classes))
            .saturating_add(capacity_bytes(&self.set_ops))
            .saturating_add(capacity_bytes(&self.points))
            .saturating_add(capacity_bytes(&self.named))
            .saturating_add(capacity_bytes(&self.names))
            .saturating_add(capacity_bytes(&self.text))
    }

    /// Drop the program and credit `budget` with what it holds for it.
    pub fn release(self, budget: &mut dyn Budget) {
        let charged = self.charged;
        drop(self);
        if charged > 0 {
            budget.credit(charged);
        }
    }
}
