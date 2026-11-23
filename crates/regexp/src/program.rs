//! The program CompilePattern (`#sec-compilepattern`) makes of a pattern's tree, and the
//! matcher runs (`docs/design/engine.md`, decision 7): one vector of instructions, with the
//! ranges of the classes, the group numbers a named backreference names and the group names
//! in vectors beside it. Every vector was charged through the [`Budget`] before it grew;
//! [`Program::heap_bytes`] is what the budget holds for the program, and [`Program::release`]
//! drops it and credits that.
//!
//! What the matcher keeps while it runs one try (`matcher.rs`): the position, a code unit
//! index into the subject; the captures, two slots per group (its start and its end, or
//! unset), group 0 the match; one counter per quantifier (the iterations run and the
//! position the current one started at); one register per lookaround (the height of the
//! backtrack stack when it was entered); and the backtrack stack, whose entries resume an
//! instruction at a position or restore a slot or a counter. Each instruction below says what
//! it does to that state; one that fails hands control to the backtrack stack.

use crate::ast::Name;
use crate::budget::{Budget, capacity_bytes};
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
    /// `.`: any character but a LineTerminator, or any character with `dot_all`
    /// (`#sec-compileatom`, Atom :: `.`).
    Any { dot_all: bool, back: bool },
    /// A character of the class numbered `class` of [`Program::classes`].
    Class { class: u32, back: bool },
    /// `^`: the position is 0, or with `multiline` follows a LineTerminator.
    LineStart { multiline: bool },
    /// `$`: the position is the subject's end, or with `multiline` precedes a LineTerminator.
    LineEnd { multiline: bool },
    /// `\b` (`negated` false) or `\B`: whether the characters on the two sides of the position
    /// differ in IsWordChar (`#sec-runtime-semantics-iswordchar-abstract-operation`).
    WordBoundary { negated: bool },
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
    /// unset group matches the empty String (`#sec-backreference-matcher`).
    Backref { group: u32, back: bool },
    /// `\k<name>` of a name several groups have: the groups `len` from `start` of
    /// [`Program::named`]; the one of them that is set, or the empty String.
    NamedBackref { start: u32, len: u32, back: bool },
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
/// `\s` and `\S` as two predicates beside them; `negated` for `[^...]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ClassSet {
    pub(crate) start: u32,
    pub(crate) len: u32,
    pub(crate) negated: bool,
    pub(crate) space: bool,
    pub(crate) not_space: bool,
}

impl ClassSet {
    /// Whether the character `c` is in the class.
    pub(crate) fn contains(&self, ranges: &[(u32, u32)], c: u32) -> bool {
        let start = self.start as usize;
        let own = ranges
            .get(start..start.saturating_add(self.len as usize))
            .unwrap_or(&[]);
        let in_ranges = own
            .binary_search_by(|&(first, last)| {
                if last < c {
                    std::cmp::Ordering::Less
                } else if first > c {
                    std::cmp::Ordering::Greater
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .is_ok();
        let found = in_ranges || (self.space && is_space(c)) || (self.not_space && !is_space(c));
        found != self.negated
    }
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

/// A character of WordCharacters without the flag `i` (`#sec-wordcharacters`): the 63 of
/// `[A-Za-z0-9_]`.
pub(crate) fn is_word(c: u32) -> bool {
    matches!(c, 0x30..=0x39 | 0x41..=0x5A | 0x5F | 0x61..=0x7A)
}

/// The program of a pattern.
#[derive(Debug)]
pub struct Program {
    pub(crate) code: Vec<Inst>,
    pub(crate) ranges: Vec<(u32, u32)>,
    pub(crate) classes: Vec<ClassSet>,
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
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
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
