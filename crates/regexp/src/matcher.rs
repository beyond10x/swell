//! The matcher of 22.2.2 (`#sec-pattern-semantics`) over a [`Program`], and the search of
//! RegExpBuiltinExec (`#sec-regexpbuiltinexec`, steps 11 to 13) that tries each start index.
//!
//! One loop over the instructions with an explicit backtrack stack, no Rust recursion
//! (`docs/design/engine.md`, decision 7; decision 3, "Depth bounds"). The continuations of the
//! clause become entries of the stack: a choice pushes the instruction and position it
//! resumes, and every change to a capture or a counter pushes the value it restores, so that
//! taking the entries back to a choice gives the state the clause's continuation saw.
//!
//! Positions are code unit indices in every mode, `u` and `v` included (decision 7, "How
//! `swell-regexp` charges its work", item 3). The subject is the code units of a String read in
//! place in either stored form of decision 5 ([`Subject`]) and never copied. Under `u` a
//! character is the code point at a position (a surrogate pair two code units, a lone surrogate
//! one), and the search moves from one try to the next by AdvanceStringIndex
//! (`#sec-advancestringindex`); a start index on the second unit of a pair starts the first
//! try at the pair's first unit, the character "obtained from element lastIndex", and the
//! search answers the caller's index beside the captures.
//!
//! Steps (decision 7, "What one step of the matcher is"): one per start index tried, one per
//! instruction run, one per entry taken from the backtrack stack (a lookaround's exit takes
//! the entries above its barrier and the barrier), and one per code unit a backreference
//! compares, charged before the comparison. They are taken by grants of at most
//! [`MAX_GRANT`], a further grant asked when one is spent; a match that ends with a match, no
//! match or a refused charge returns what it was granted and did not run, so the steps
//! charged are the units run and the interrupt, which every grant reads, is read at least once
//! per 4,096 of them. A refused grant stops the match and keeps what was granted.
//!
//! Memory: the captures, the counters, the registers of the lookarounds and the backtrack
//! stack grow by capacity, each growth charged before it is made, and every byte is credited
//! on every way out: a match, no match and a stop. The captures a match answers are the
//! caller's from then on, as the match array is (RegExpBuiltinExec charges it).

use std::ops::Range;

use crate::budget::{Budget, MAX_GRANT, Stop};
use crate::program::{Inst, Program, is_line_terminator, is_word};

/// The code units of a String in one of its two stored forms: Latin-1 bytes, each one code
/// unit, or 16-bit code units.
#[derive(Clone, Copy, Debug)]
pub enum Subject<'s> {
    Latin1(&'s [u8]),
    Utf16(&'s [u16]),
}

impl Subject<'_> {
    /// The count of code units.
    pub fn len(&self) -> usize {
        match self {
            Subject::Latin1(bytes) => bytes.len(),
            Subject::Utf16(units) => units.len(),
        }
    }

    /// Whether it has no code unit.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn unit(&self, index: usize) -> Option<u16> {
        match self {
            Subject::Latin1(bytes) => bytes.get(index).map(|&b| u16::from(b)),
            Subject::Utf16(units) => units.get(index).copied(),
        }
    }
}

/// The value of a capture slot that is unset.
const UNSET: usize = usize::MAX;

/// The captures of a match: for each group, group 0 the match, its range of code units or
/// unset.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Captures {
    slots: Vec<usize>,
}

impl Captures {
    /// The count of groups, group 0 included.
    pub fn len(&self) -> usize {
        self.slots.len() / 2
    }

    /// Whether it has no group; a match has group 0.
    pub fn is_empty(&self) -> bool {
        self.slots.len() < 2
    }

    /// The range of group `n`, or `None` when it is unset or past the last group.
    pub fn get(&self, n: usize) -> Option<Range<usize>> {
        let start = *self.slots.get(n.checked_mul(2)?)?;
        let end = *self.slots.get(n.checked_mul(2)?.checked_add(1)?)?;
        if start == UNSET || end == UNSET {
            return None;
        }
        Some(start..end)
    }

    /// Each group's range in order, group 0 first.
    pub fn iter(&self) -> impl Iterator<Item = Option<Range<usize>>> + '_ {
        (0..self.len()).map(|n| self.get(n))
    }
}

/// A match the search found: the start index of the try that matched, as the caller counts
/// it (RegExpBuiltinExec's `lastIndex`), and the captures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub index: usize,
    pub captures: Captures,
}

/// The match of `program` against `subject` from the code unit index `at`, one try: the
/// captures, or `None` for no match (also when `at` is past the subject's end, which tries
/// nothing), or the stop of the budget with everything credited.
pub fn match_at(
    program: &Program,
    subject: Subject<'_>,
    at: usize,
    budget: &mut dyn Budget,
) -> Result<Option<Captures>, Stop> {
    if at > subject.len() {
        return Ok(None);
    }
    let mut machine = Machine::new(program, subject, budget);
    let result = machine.prepare().and_then(|()| machine.attempt(at));
    let result = result.map(|matched| matched.then(|| machine.captures()));
    machine.finish(result)
}

/// The search of RegExpBuiltinExec from the code unit index `start`: a try at each index from
/// there, moved by AdvanceStringIndex, or only at `start` when the program's flags have `y`;
/// the first match, or `None`, or the stop of the budget with everything credited.
pub fn search(
    program: &Program,
    subject: Subject<'_>,
    start: usize,
    budget: &mut dyn Budget,
) -> Result<Option<Found>, Stop> {
    if start > subject.len() {
        return Ok(None);
    }
    let mut machine = Machine::new(program, subject, budget);
    let result = machine.prepare().and_then(|()| machine.search(start));
    let result = result.map(|index| {
        index.map(|index| Found {
            index,
            captures: machine.captures(),
        })
    });
    machine.finish(result)
}

/// The iterations a quantifier's loop has run, and the position the current one started at.
#[derive(Clone, Copy, Debug)]
struct Counter {
    count: u32,
    start: usize,
}

/// One entry of the backtrack stack.
#[derive(Clone, Copy, Debug)]
enum Entry {
    /// Resume the instruction `pc` at `pos`.
    Resume { pc: u32, pos: usize },
    /// Restore the capture slot `slot` to `old`.
    Capture { slot: u32, old: usize },
    /// Restore the counter `counter`.
    Counter { counter: u32, old: Counter },
    /// The barrier of a lookaround entered at `pos`: taken back after its body failed, a
    /// negative one goes on at `pc` and a positive one fails.
    Look { negative: bool, pc: u32, pos: usize },
}

/// What one instruction answers: go on at an instruction and a position, or fail.
type Next = Option<(u32, usize)>;

fn is_lead(unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&unit)
}

fn is_trail(unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&unit)
}

fn pair(lead: u16, trail: u16) -> u32 {
    0x10000 + ((u32::from(lead) - 0xD800) << 10) + (u32::from(trail) - 0xDC00)
}

struct Machine<'p, 's, 'b> {
    program: &'p Program,
    subject: Subject<'s>,
    unicode: bool,
    budget: &'b mut dyn Budget,
    held: usize,
    left: u64,
    slots: Vec<usize>,
    counters: Vec<Counter>,
    looks: Vec<usize>,
    stack: Vec<Entry>,
}

impl<'p, 's, 'b> Machine<'p, 's, 'b> {
    fn new(
        program: &'p Program,
        subject: Subject<'s>,
        budget: &'b mut dyn Budget,
    ) -> Machine<'p, 's, 'b> {
        Machine {
            program,
            subject,
            unicode: program.flags.either_unicode(),
            budget,
            held: 0,
            left: 0,
            slots: Vec::new(),
            counters: Vec::new(),
            looks: Vec::new(),
            stack: Vec::new(),
        }
    }

    // ----- the budget -----

    /// Charge `bytes`, held until the match ends.
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        if bytes == 0 {
            return Ok(());
        }
        self.budget.charge(bytes)?;
        self.held = self.held.saturating_add(bytes);
        Ok(())
    }

    /// The capacity of `v` raised to `wanted`, the growth charged first.
    fn reserve<T>(&mut self, v: &mut Vec<T>, wanted: usize) -> Result<(), Stop> {
        let had = v.capacity();
        if wanted <= had {
            return Ok(());
        }
        let bytes = wanted.saturating_sub(had).saturating_mul(size_of::<T>());
        self.charge(bytes)?;
        if v.try_reserve_exact(wanted.saturating_sub(v.len())).is_err() {
            self.held = self.held.saturating_sub(bytes);
            self.budget.credit(bytes);
            return Err(Stop::MemoryBudget);
        }
        let got = v.capacity();
        if got > wanted {
            self.charge(got.saturating_sub(wanted).saturating_mul(size_of::<T>()))?;
        }
        Ok(())
    }

    /// Push `entry` onto the backtrack stack, its capacity doubled (at least 4) first when it
    /// is full.
    fn push(&mut self, entry: Entry) -> Result<(), Stop> {
        if self.stack.len() == self.stack.capacity() {
            let wanted = self.stack.capacity().saturating_mul(2).max(4);
            let mut stack = std::mem::take(&mut self.stack);
            let grown = self.reserve(&mut stack, wanted);
            self.stack = stack;
            grown?;
        }
        self.stack.push(entry);
        Ok(())
    }

    /// One unit of work, from the grant or from a new one.
    fn step(&mut self) -> Result<(), Stop> {
        if self.left == 0 {
            self.left = u64::from(self.budget.grant(MAX_GRANT)?);
        }
        self.left = self.left.saturating_sub(1);
        Ok(())
    }

    /// `n` units of work, further grants taken first when what is left is less.
    fn steps(&mut self, n: usize) -> Result<(), Stop> {
        let n = n as u64;
        while self.left < n {
            let granted = self.budget.grant(MAX_GRANT)?;
            self.left = self.left.saturating_add(u64::from(granted));
        }
        self.left -= n;
        Ok(())
    }

    /// The captures, the counters and the registers of the lookarounds, charged before they
    /// are made.
    fn prepare(&mut self) -> Result<(), Stop> {
        let slots = (self.program.groups as usize)
            .saturating_add(1)
            .saturating_mul(2);
        let mut v = Vec::new();
        let made = self.reserve(&mut v, slots);
        v.resize(v.capacity().min(slots), UNSET);
        self.slots = v;
        made?;
        let mut v = Vec::new();
        let counters = self.program.counters as usize;
        let made = self.reserve(&mut v, counters);
        v.resize(v.capacity().min(counters), Counter { count: 0, start: 0 });
        self.counters = v;
        made?;
        let mut v = Vec::new();
        let looks = self.program.looks as usize;
        let made = self.reserve(&mut v, looks);
        v.resize(v.capacity().min(looks), 0);
        self.looks = v;
        made
    }

    /// The captures of the match, moved out; their bytes are credited with the rest.
    fn captures(&mut self) -> Captures {
        Captures {
            slots: std::mem::take(&mut self.slots),
        }
    }

    /// The end of the match: the steps granted and not run returned, unless a grant was
    /// refused, and every byte credited.
    fn finish<T>(mut self, result: Result<T, Stop>) -> Result<T, Stop> {
        let refused_grant = matches!(result, Err(Stop::StepBudget | Stop::Interrupted));
        if !refused_grant {
            while self.left > 0 {
                let back = u32::try_from(self.left).unwrap_or(u32::MAX);
                self.budget.refund(back);
                self.left -= u64::from(back);
            }
        }
        let bytes = self.held;
        self.held = 0;
        drop(std::mem::take(&mut self.stack));
        drop(std::mem::take(&mut self.slots));
        drop(std::mem::take(&mut self.counters));
        drop(std::mem::take(&mut self.looks));
        if bytes > 0 {
            self.budget.credit(bytes);
        }
        result
    }

    // ----- the search -----

    /// The tries from `start`: the caller's index of the one that matched, or `None`.
    fn search(&mut self, start: usize) -> Result<Option<usize>, Stop> {
        let len = self.subject.len();
        let sticky = self.program.flags.sticky();
        let mut last = start;
        while last <= len {
            let at = if self.unicode {
                self.character_start(last)
            } else {
                last
            };
            if self.attempt(at)? {
                return Ok(Some(last));
            }
            if sticky {
                break;
            }
            last = self.advance(last);
        }
        Ok(None)
    }

    /// The index of the character obtained from element `index` under `u`: the first unit of
    /// the pair whose second unit `index` is.
    fn character_start(&self, index: usize) -> usize {
        if index > 0
            && self.subject.unit(index).is_some_and(is_trail)
            && self.subject.unit(index - 1).is_some_and(is_lead)
        {
            return index - 1;
        }
        index
    }

    /// AdvanceStringIndex.
    fn advance(&self, index: usize) -> usize {
        if !self.unicode || index.saturating_add(1) >= self.subject.len() {
            return index.saturating_add(1);
        }
        let paired = self.subject.unit(index).is_some_and(is_lead)
            && self.subject.unit(index + 1).is_some_and(is_trail);
        index.saturating_add(if paired { 2 } else { 1 })
    }

    // ----- one try -----

    /// One try at `at`: whether it matched.
    fn attempt(&mut self, at: usize) -> Result<bool, Stop> {
        self.step()?;
        self.slots.fill(UNSET);
        if let Some(start) = self.slots.first_mut() {
            *start = at;
        }
        self.stack.clear();
        let mut pc = 0u32;
        let mut pos = at;
        loop {
            self.step()?;
            let Some(&inst) = self.program.code.get(pc as usize) else {
                return Ok(false);
            };
            if inst == Inst::Match {
                if let Some(end) = self.slots.get_mut(1) {
                    *end = pos;
                }
                return Ok(true);
            }
            match self.run(inst, pc, pos)? {
                Some((next, at)) => {
                    pc = next;
                    pos = at;
                }
                None => match self.backtrack()? {
                    Some((next, at)) => {
                        pc = next;
                        pos = at;
                    }
                    None => return Ok(false),
                },
            }
        }
    }

    /// Take entries from the backtrack stack until one resumes an instruction: that
    /// instruction and its position, or `None` when the stack is empty and the try failed.
    fn backtrack(&mut self) -> Result<Next, Stop> {
        while !self.stack.is_empty() {
            self.step()?;
            let Some(entry) = self.stack.pop() else { break };
            match entry {
                Entry::Resume { pc, pos } => return Ok(Some((pc, pos))),
                Entry::Look {
                    negative: true,
                    pc,
                    pos,
                } => return Ok(Some((pc, pos))),
                entry => self.restore(entry),
            }
        }
        Ok(None)
    }

    /// Undo the change an entry records; an entry that resumes nothing does nothing.
    fn restore(&mut self, entry: Entry) {
        match entry {
            Entry::Capture { slot, old } => {
                if let Some(value) = self.slots.get_mut(slot as usize) {
                    *value = old;
                }
            }
            Entry::Counter { counter, old } => {
                if let Some(value) = self.counters.get_mut(counter as usize) {
                    *value = old;
                }
            }
            Entry::Resume { .. } | Entry::Look { .. } => {}
        }
    }

    /// The character at `pos` in the direction, read as a code point under `u` and as a code
    /// unit without, and the position past it.
    fn read(&self, pos: usize, back: bool) -> Option<(u32, usize)> {
        if back {
            let before = pos.checked_sub(1)?;
            let unit = self.subject.unit(before)?;
            if self.unicode
                && is_trail(unit)
                && let Some(lead) = before
                    .checked_sub(1)
                    .and_then(|i| self.subject.unit(i))
                    .filter(|&u| is_lead(u))
            {
                return Some((pair(lead, unit), before - 1));
            }
            return Some((u32::from(unit), before));
        }
        let unit = self.subject.unit(pos)?;
        if self.unicode
            && is_lead(unit)
            && let Some(trail) = self.subject.unit(pos + 1).filter(|&u| is_trail(u))
        {
            return Some((pair(unit, trail), pos + 2));
        }
        Some((u32::from(unit), pos + 1))
    }

    /// IsWordChar of the code unit at `index`; none outside the subject.
    fn word_at(&self, index: Option<usize>) -> bool {
        index
            .and_then(|i| self.subject.unit(i))
            .is_some_and(|unit| is_word(u32::from(unit)))
    }

    fn slot(&self, slot: usize) -> usize {
        self.slots.get(slot).copied().unwrap_or(UNSET)
    }

    fn set_slot(&mut self, slot: u32, value: usize) -> Result<(), Stop> {
        let old = self.slot(slot as usize);
        self.push(Entry::Capture { slot, old })?;
        if let Some(s) = self.slots.get_mut(slot as usize) {
            *s = value;
        }
        Ok(())
    }

    fn counter(&self, counter: u32) -> Counter {
        self.counters
            .get(counter as usize)
            .copied()
            .unwrap_or(Counter {
                count: 0,
                start: UNSET,
            })
    }

    fn set_counter(&mut self, counter: u32, value: Counter) -> Result<(), Stop> {
        let old = self.counter(counter);
        self.push(Entry::Counter { counter, old })?;
        if let Some(c) = self.counters.get_mut(counter as usize) {
            *c = value;
        }
        Ok(())
    }

    /// Run the instruction `inst` at `pc` and `pos`; its step is charged.
    fn run(&mut self, inst: Inst, pc: u32, pos: usize) -> Result<Next, Stop> {
        let next = pc.saturating_add(1);
        let len = self.subject.len();
        Ok(match inst {
            Inst::Unit { unit, back } => {
                if back {
                    pos.checked_sub(1)
                        .filter(|&i| self.subject.unit(i) == Some(unit))
                        .map(|i| (next, i))
                } else {
                    (self.subject.unit(pos) == Some(unit)).then_some((next, pos + 1))
                }
            }
            Inst::Point { point, back } => self
                .read(pos, back)
                .filter(|&(c, _)| c == point)
                .map(|(_, to)| (next, to)),
            Inst::Any { dot_all, back } => self
                .read(pos, back)
                .filter(|&(c, _)| dot_all || !is_line_terminator(c))
                .map(|(_, to)| (next, to)),
            Inst::Class { class, back } => {
                let program = self.program;
                let set = program.classes.get(class as usize);
                self.read(pos, back)
                    .filter(|&(c, _)| set.is_some_and(|set| set.contains(&program.ranges, c)))
                    .map(|(_, to)| (next, to))
            }
            Inst::LineStart { multiline } => {
                let at_start = pos == 0
                    || (multiline
                        && pos
                            .checked_sub(1)
                            .and_then(|i| self.subject.unit(i))
                            .is_some_and(|u| is_line_terminator(u32::from(u))));
                at_start.then_some((next, pos))
            }
            Inst::LineEnd { multiline } => {
                let at_end = pos == len
                    || (multiline
                        && self
                            .subject
                            .unit(pos)
                            .is_some_and(|u| is_line_terminator(u32::from(u))));
                at_end.then_some((next, pos))
            }
            Inst::WordBoundary { negated } => {
                let a = self.word_at(pos.checked_sub(1));
                let b = self.word_at(Some(pos));
                ((a != b) != negated).then_some((next, pos))
            }
            Inst::Split { first, second } => {
                self.push(Entry::Resume { pc: second, pos })?;
                Some((first, pos))
            }
            Inst::Jump { to } => Some((to, pos)),
            Inst::Save { slot } => {
                self.set_slot(slot, pos)?;
                Some((next, pos))
            }
            Inst::Clear { first, last } => {
                let from = (first as usize).saturating_mul(2);
                let to = (last as usize).saturating_mul(2).saturating_add(1);
                for slot in from..=to.min(self.slots.len().saturating_sub(1)) {
                    if self.slot(slot) != UNSET {
                        self.set_slot(slot as u32, UNSET)?;
                    }
                }
                Some((next, pos))
            }
            Inst::RepeatInit { counter } => {
                self.set_counter(
                    counter,
                    Counter {
                        count: 0,
                        start: pos,
                    },
                )?;
                Some((next, pos))
            }
            Inst::Repeat {
                counter,
                min,
                max,
                greedy,
                exit,
            } => {
                let count = self.counter(counter).count;
                if max.is_some_and(|max| count >= max) {
                    Some((exit, pos))
                } else if count < min {
                    Some((next, pos))
                } else if greedy {
                    self.push(Entry::Resume { pc: exit, pos })?;
                    Some((next, pos))
                } else {
                    self.push(Entry::Resume { pc: next, pos })?;
                    Some((exit, pos))
                }
            }
            Inst::RepeatTail { counter, min, head } => {
                let current = self.counter(counter);
                if current.count >= min && pos == current.start {
                    None
                } else {
                    self.set_counter(
                        counter,
                        Counter {
                            count: current.count.saturating_add(1),
                            start: pos,
                        },
                    )?;
                    Some((head, pos))
                }
            }
            Inst::Backref { group, back } => self.backreference(group, back, pc, pos)?,
            Inst::NamedBackref { start, len, back } => {
                let program = self.program;
                let first = start as usize;
                let groups = program
                    .named
                    .get(first..first.saturating_add(len as usize))
                    .unwrap_or(&[]);
                let set = groups.iter().copied().find(|&group| {
                    let slot = (group as usize).saturating_mul(2);
                    self.slot(slot) != UNSET && self.slot(slot.saturating_add(1)) != UNSET
                });
                match set {
                    Some(group) => self.backreference(group, back, pc, pos)?,
                    None => Some((next, pos)),
                }
            }
            Inst::LookStart {
                look,
                negative,
                end,
            } => {
                let height = self.stack.len();
                if let Some(register) = self.looks.get_mut(look as usize) {
                    *register = height;
                }
                self.push(Entry::Look {
                    negative,
                    pc: end,
                    pos,
                })?;
                Some((next, pos))
            }
            Inst::LookEnd { look, negative } => self.look_end(look, negative, next)?,
            Inst::Match => Some((next, pos)),
        })
    }

    /// BackreferenceMatcher (`#sec-backreference-matcher`) of `group` at `pos`: an unset
    /// group matches the empty String; otherwise its code units are compared, one step each
    /// charged before the comparison. Under `u` the end the comparison reaches must not split
    /// a surrogate pair, since the clause compares code points.
    fn backreference(&mut self, group: u32, back: bool, pc: u32, pos: usize) -> Result<Next, Stop> {
        let next = pc.saturating_add(1);
        let slot = (group as usize).saturating_mul(2);
        let (start, end) = (self.slot(slot), self.slot(slot.saturating_add(1)));
        if start == UNSET || end == UNSET {
            return Ok(Some((next, pos)));
        }
        let n = end.saturating_sub(start);
        let from = if back {
            let Some(from) = pos.checked_sub(n) else {
                return Ok(None);
            };
            from
        } else {
            if pos.saturating_add(n) > self.subject.len() {
                return Ok(None);
            }
            pos
        };
        self.steps(n)?;
        let same = (0..n).all(|i| self.subject.unit(start + i) == self.subject.unit(from + i));
        if !same {
            return Ok(None);
        }
        let to = if back { from } else { from + n };
        if self.unicode && n > 0 && self.splits_pair(to) {
            return Ok(None);
        }
        Ok(Some((next, to)))
    }

    /// Whether `index` lies between the two units of a surrogate pair.
    fn splits_pair(&self, index: usize) -> bool {
        index > 0
            && self.subject.unit(index).is_some_and(is_trail)
            && self.subject.unit(index - 1).is_some_and(is_lead)
    }

    /// The exit of a lookaround whose body matched; its register holds its barrier's index.
    fn look_end(&mut self, look: u32, negative: bool, next: u32) -> Result<Next, Stop> {
        let barrier = self.looks.get(look as usize).copied().unwrap_or(0);
        let taken = self.stack.len().saturating_sub(barrier);
        self.steps(taken)?;
        if negative {
            // The body matched, so the lookaround fails: every entry down to the barrier is
            // taken and its change undone.
            while self.stack.len() > barrier {
                if let Some(entry) = self.stack.pop() {
                    self.restore(entry);
                }
            }
            return Ok(None);
        }
        let Some(Entry::Look { pos, .. }) = self.stack.get(barrier).copied() else {
            return Ok(None);
        };
        // Atomic: no entry resumes inside the body; the restores are kept, so that a failure
        // after the lookaround undoes its captures.
        let mut kept = barrier;
        for i in barrier.saturating_add(1)..self.stack.len() {
            let Some(entry) = self.stack.get(i).copied() else {
                break;
            };
            if matches!(entry, Entry::Capture { .. } | Entry::Counter { .. }) {
                if let Some(slot) = self.stack.get_mut(kept) {
                    *slot = entry;
                }
                kept += 1;
            }
        }
        self.stack.truncate(kept);
        Ok(Some((next, pos)))
    }
}
