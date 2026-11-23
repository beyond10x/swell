//! The early errors of a pattern (`#sec-patterns-static-semantics-early-errors` and
//! `#sec-patterns-static-semantics-early-errors-annexb`), checked as the parser reaches each
//! production, and after the parse for the two that need the whole pattern: a backreference
//! past the count of capturing groups and `\k<name>` naming no group.
//!
//! A group name declared twice is an error unless the two declarations lie in different
//! alternatives of one Disjunction (MightBothParticipate, `#sec-mightbothparticipate`). Of the
//! declarations of one name, every pair is such a pair exactly when every two that follow each
//! other in the text are: for x before y before z, if x and z share an alternative of their
//! nearest common Disjunction, y, which lies between them, shares one with x or with z. So
//! [`Names`] keeps the last declaration of each name and checks the next against it, by a
//! union-find over the levels of the parse (the pattern and every group): a closed level is
//! joined to the level that held it, with the index of the alternative it lay in, and the
//! nearest open level of a declaration is the nearest common Disjunction of it and the next.
//! The work is near linear in the pattern, and no step of it recurses.

use std::collections::BTreeMap;

use crate::ast::{ClassEscape, ClassItem};
use crate::budget::{Meter, Stop};
use crate::error::ErrorKind;

/// Whether the quantifier `{min,max}` is in order: the MV of the digits of `min` is not greater
/// than that of `max` (`#sec-patterns-static-semantics-early-errors`, QuantifierPrefix). The
/// digits are compared as written, so no count is too large to compare.
pub(crate) fn quantifier_in_order(min: &[u16], max: &[u16]) -> bool {
    let (min, max) = (significant(min), significant(max));
    if min.len() != max.len() {
        return min.len() < max.len();
    }
    min <= max
}

/// `digits` without their leading zeros.
fn significant(digits: &[u16]) -> &[u16] {
    let zeros = digits.iter().take_while(|&&d| d == u16::from(b'0')).count();
    digits.get(zeros..).unwrap_or(&[])
}

/// One end of a class range: a character, or a class escape (IsCharacterClass true).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClassAtom {
    Character(u32),
    Escape(ClassEscape),
}

/// The items of the class range `first-second` (NonemptyClassRanges and
/// NonemptyClassRangesNoDash): a class escape at an end is an error under `u` and, without it,
/// the union of both ends and `-` (B.1.2.1 and CharacterRangeOrUnion of B.1.2); two characters
/// out of order are an error in either mode.
pub(crate) fn class_range(
    first: ClassAtom,
    second: ClassAtom,
    unicode: bool,
) -> Result<[Option<ClassItem>; 3], ErrorKind> {
    match (first, second) {
        (ClassAtom::Character(a), ClassAtom::Character(b)) => {
            if a > b {
                return Err(ErrorKind::RangeOutOfOrder);
            }
            Ok([Some(ClassItem::Range(a, b)), None, None])
        }
        _ if unicode => Err(ErrorKind::ClassEscapeInRange),
        _ => Ok([
            Some(item(first)),
            Some(ClassItem::Character(u32::from(b'-'))),
            Some(item(second)),
        ]),
    }
}

fn item(atom: ClassAtom) -> ClassItem {
    match atom {
        ClassAtom::Character(c) => ClassItem::Character(c),
        ClassAtom::Escape(e) => ClassItem::Escape(e),
    }
}

/// The early errors of the two lists of a modifier group: a modifier twice in one list, in
/// both lists, or both lists empty with a `-` (`#sec-patterns-static-semantics-early-errors`,
/// Atom). Each list is a set of bits; `repeated` tells whether a list named one twice.
pub(crate) fn modifiers(add: u8, remove: u8, dash: bool, repeated: bool) -> Option<ErrorKind> {
    if repeated {
        return Some(ErrorKind::RepeatedModifier);
    }
    if dash && add == 0 && remove == 0 {
        return Some(ErrorKind::EmptyModifiers);
    }
    if add & remove != 0 {
        return Some(ErrorKind::ModifierAddedAndRemoved);
    }
    None
}

/// The group names of a parse and the levels the duplicate check reads.
pub(crate) struct Names {
    /// Per level, the level it was joined to when it closed; itself while it is open.
    parent: Vec<u32>,
    /// Per level, the index of the alternative of `parent` it lies in.
    alternative: Vec<u32>,
    /// Per level, the index of its alternative now open.
    current: Vec<u32>,
    /// Per name, the level and the alternative of its last declaration.
    last: BTreeMap<Vec<u8>, (u32, u32)>,
}

impl Names {
    /// The names of a parse, with the level of the pattern itself, numbered 0.
    pub(crate) fn new(m: &mut Meter<'_>) -> Result<Names, Stop> {
        let mut names = Names {
            parent: Vec::new(),
            alternative: Vec::new(),
            current: Vec::new(),
            last: BTreeMap::new(),
        };
        names.push(m, 0)?;
        Ok(names)
    }

    /// A new open level, lying in the alternative `alternative` of the level that holds it.
    fn push(&mut self, m: &mut Meter<'_>, alternative: u32) -> Result<u32, Stop> {
        let id = u32::try_from(self.parent.len()).map_err(|_| Stop::MemoryBudget)?;
        m.push(&mut self.parent, id)?;
        m.push(&mut self.alternative, alternative)?;
        m.push(&mut self.current, 0)?;
        Ok(id)
    }

    /// A level opened inside the open level `outer`: its number.
    pub(crate) fn open(&mut self, m: &mut Meter<'_>, outer: u32) -> Result<u32, Stop> {
        let at = self.current_of(outer);
        self.push(m, at)
    }

    /// A `|` in the open level `level`.
    pub(crate) fn alternative(&mut self, level: u32) {
        if let Some(current) = self.current.get_mut(level as usize) {
            *current = current.saturating_add(1);
        }
    }

    /// The level `level` closed inside the open level `outer`.
    pub(crate) fn close(&mut self, level: u32, outer: u32) {
        if let Some(parent) = self.parent.get_mut(level as usize) {
            *parent = outer;
        }
    }

    fn current_of(&self, level: u32) -> u32 {
        self.current.get(level as usize).copied().unwrap_or(0)
    }

    fn parent_of(&self, level: u32) -> u32 {
        self.parent.get(level as usize).copied().unwrap_or(level)
    }

    /// The nearest open level that holds `level`, and the alternative of it that holds
    /// `level`, or `None` when `level` is open itself. The path is compressed on the way.
    fn find(&mut self, level: u32) -> Option<(u32, u32)> {
        if self.parent_of(level) == level {
            return None;
        }
        let mut below = level;
        let mut at = self.parent_of(level);
        while self.parent_of(at) != at {
            below = at;
            at = self.parent_of(at);
        }
        let root = at;
        let index = self.alternative.get(below as usize).copied().unwrap_or(0);
        let mut node = level;
        while self.parent_of(node) != root && self.parent_of(node) != node {
            let next = self.parent_of(node);
            if let Some(parent) = self.parent.get_mut(node as usize) {
                *parent = root;
            }
            if let Some(alternative) = self.alternative.get_mut(node as usize) {
                *alternative = index;
            }
            node = next;
        }
        Some((root, index))
    }

    /// The group name `name` declared in the open level `level`: `Ok(false)` when an earlier
    /// declaration of it might participate together with this one, the duplicate early error.
    pub(crate) fn declare(
        &mut self,
        m: &mut Meter<'_>,
        name: &[u8],
        level: u32,
    ) -> Result<bool, Stop> {
        let here = (level, self.current_of(level));
        if let Some(&(earlier, at)) = self.last.get(name) {
            let (open, index) = self.find(earlier).unwrap_or((earlier, at));
            if self.current_of(open) == index {
                return Ok(false);
            }
            if let Some(last) = self.last.get_mut(name) {
                *last = here;
            }
            return Ok(true);
        }
        let key = m.copy(name)?;
        m.insert(&mut self.last, key, here)?;
        Ok(true)
    }

    /// Whether some group has the name `name` (GroupSpecifiersThatMatch is not empty).
    pub(crate) fn has(&self, name: &[u8]) -> bool {
        self.last.contains_key(name)
    }

    /// Drop the names and credit them.
    pub(crate) fn release(self, m: &mut Meter<'_>) {
        m.release(self.parent);
        m.release(self.alternative);
        m.release(self.current);
        m.release_map(self.last);
    }
}
