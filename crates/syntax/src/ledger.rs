//! The ledger of the frontend (`docs/design/engine.md`, decision 4, "Memory budget", and
//! decision 3, "The step budget"; `docs/design/host-interface.md`, section 12): one trait,
//! [`Ledger`], through which the lexer, the parser, the early errors, the scope analysis and the
//! compile step (`swell-compiler` takes the same trait) charge every growth of their structures
//! before it is made, credit what they drop, and read the host's interrupt. The engine implements
//! it once, over the agent's accounting and its interrupt flag; [`Unlimited`] charges nothing and
//! never stops, for the entries without a ledger.
//!
//! A step shares one [`Meter`] over the ledger, held by a shared reference in the parser's state
//! (never in a recursive frame) and in the walks after it. The meter keeps what the step holds
//! charged ([`Meter::held`]) and the first [`Stop`] the ledger answered: once a charge is refused
//! or a poll stops, every later charge is refused too, the step stops at its next check, and the
//! entry credits everything the meter holds.
//!
//! - **By capacity.** A vector grows by the frontend's own doubling, to at least 4 elements, with
//!   `try_reserve_exact` ([`Meter::grow`]), as the engine's property lists do, so a charge does not
//!   depend on the growth policy of the standard library; a refusal of the allocator is a refused
//!   charge. A copy is made with exactly its length as capacity.
//! - **Map entries.** An entry of a `BTreeMap` or a `BTreeSet` is charged at its insertion as
//!   [`map_entry_bytes`]: its key and value bytes plus a share of a node.
//! - **Polls.** [`Meter::poll`] reads the interrupt through the ledger; the lexer polls at least
//!   once per [`POLL_INTERVAL`] code units it advances, and the walks after the parser poll once
//!   per [`POLL_INTERVAL`] nodes they visit ([`Meter::visit`]).

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

/// Why a step of the frontend stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    /// A charge was refused: the memory budget, or the allocator.
    OverBudget,
    /// The host's interrupt was read.
    Interrupted,
}

/// The ledger the frontend charges through.
pub trait Ledger {
    /// Charge `bytes` before the growth they pay for, or refuse and charge nothing.
    fn charge(&mut self, bytes: usize) -> Result<(), Stop>;
    /// Give back `bytes` charged before.
    fn credit(&mut self, bytes: usize);
    /// Read the host's interrupt: `Err(Stop::Interrupted)` when it is set.
    fn poll(&mut self) -> Result<(), Stop>;
}

/// A ledger that charges nothing and never stops: the entries without a ledger.
#[derive(Clone, Copy, Debug, Default)]
pub struct Unlimited;

impl Ledger for Unlimited {
    fn charge(&mut self, _bytes: usize) -> Result<(), Stop> {
        Ok(())
    }

    fn credit(&mut self, _bytes: usize) {}

    fn poll(&mut self) -> Result<(), Stop> {
        Ok(())
    }
}

/// The most code units the lexer advances, and the most nodes a walk visits, between two polls.
pub const POLL_INTERVAL: usize = 4096;

/// The fewest elements a vector grows to.
const MIN_ELEMENTS: usize = 4;

/// The bytes one entry of a `BTreeMap<K, V>` (or of a `BTreeSet<K>`, `V` = `()`) is charged at
/// its insertion: its key and value bytes plus a share of a node.
///
/// The derivation, from the B-tree of the standard library (B = 6): a node holds at most 11
/// entries and, but for the root, at least 5. A leaf is its parent pointer, its index in the
/// parent and its length (16 bytes) and room for 11 keys and 11 values; an internal node is a
/// leaf and 12 child pointers (96 bytes more). A tree of n entries has at most n / 5 leaves and
/// fewer internal nodes than leaves, so a leaf and an internal node for every 5 entries bound
/// it: per entry, `k + v` plus `(2 × (16 + 11 × (k + v)) + 96) / 5`, rounded up. For a `String`
/// key and no value (24 + 0 bytes on a 64-bit target) that is 24 + 132 = 156 bytes; what the key
/// owns on the heap is charged when the key is made.
pub const fn map_entry_bytes<K, V>() -> usize {
    let entry = size_of::<K>() + size_of::<V>();
    let leaf = 16 + 11 * entry;
    let internal = leaf + 96;
    entry + (leaf + internal).div_ceil(5)
}

/// The bytes a value owns on the heap beyond its own size, by capacity: what a drop of it
/// credits besides the slot that holds it.
pub trait HeapBytes {
    fn heap_bytes(&self) -> usize;
}

macro_rules! no_heap {
    ($($t:ty),* $(,)?) => {
        $(impl HeapBytes for $t {
            fn heap_bytes(&self) -> usize {
                0
            }
        })*
    };
}

no_heap!(
    (),
    bool,
    u8,
    u16,
    u32,
    u64,
    usize,
    i32,
    f64,
    crate::ast::ExprId,
    crate::ast::StmtId,
    crate::ast::FunctionId,
    crate::ast::Span,
);

impl HeapBytes for String {
    fn heap_bytes(&self) -> usize {
        self.capacity()
    }
}

impl<T: HeapBytes> HeapBytes for Vec<T> {
    fn heap_bytes(&self) -> usize {
        vec_capacity_bytes(self)
    }
}

impl<T: HeapBytes> HeapBytes for Option<T> {
    fn heap_bytes(&self) -> usize {
        self.as_ref().map_or(0, HeapBytes::heap_bytes)
    }
}

impl<A: HeapBytes, B: HeapBytes> HeapBytes for (A, B) {
    fn heap_bytes(&self) -> usize {
        self.0.heap_bytes().saturating_add(self.1.heap_bytes())
    }
}

impl<A: HeapBytes, B: HeapBytes, C: HeapBytes> HeapBytes for (A, B, C) {
    fn heap_bytes(&self) -> usize {
        self.0
            .heap_bytes()
            .saturating_add(self.1.heap_bytes())
            .saturating_add(self.2.heap_bytes())
    }
}

impl<K: HeapBytes, V: HeapBytes> HeapBytes for BTreeMap<K, V> {
    fn heap_bytes(&self) -> usize {
        self.iter().fold(
            self.len().saturating_mul(map_entry_bytes::<K, V>()),
            |sum, (k, v)| {
                sum.saturating_add(k.heap_bytes())
                    .saturating_add(v.heap_bytes())
            },
        )
    }
}

impl<K: HeapBytes> HeapBytes for BTreeSet<K> {
    fn heap_bytes(&self) -> usize {
        self.iter().fold(
            self.len().saturating_mul(map_entry_bytes::<K, ()>()),
            |sum, k| sum.saturating_add(k.heap_bytes()),
        )
    }
}

/// The bytes of the buffer of `v` by its capacity, and what its elements own.
pub fn vec_capacity_bytes<T: HeapBytes>(v: &Vec<T>) -> usize {
    v.iter()
        .fold(v.capacity().saturating_mul(size_of::<T>()), |sum, e| {
            sum.saturating_add(e.heap_bytes())
        })
}

/// The ledger of one step, shared by a reference: what the step holds charged and the first
/// [`Stop`] it met. Neither the meter nor its helpers allocate anything they do not charge.
pub struct Meter<'a> {
    ledger: Cell<Option<&'a mut dyn Ledger>>,
    held: Cell<usize>,
    stop: Cell<Option<Stop>>,
    visits: Cell<usize>,
}

impl<'a> Meter<'a> {
    pub fn new(ledger: &'a mut dyn Ledger) -> Meter<'a> {
        Meter {
            ledger: Cell::new(Some(ledger)),
            held: Cell::new(0),
            stop: Cell::new(None),
            visits: Cell::new(0),
        }
    }

    /// The first stop the step met, if any.
    pub fn stopped(&self) -> Option<Stop> {
        self.stop.get()
    }

    /// `Err` once the step has stopped.
    pub fn check(&self) -> Result<(), Stop> {
        match self.stop.get() {
            Some(stop) => Err(stop),
            None => Ok(()),
        }
    }

    /// What the step holds charged: charged and not credited.
    pub fn held(&self) -> usize {
        self.held.get()
    }

    fn with<T>(&self, f: impl FnOnce(&mut dyn Ledger) -> T) -> Option<T> {
        let ledger = self.ledger.take()?;
        let answer = f(&mut *ledger);
        self.ledger.set(Some(ledger));
        Some(answer)
    }

    fn halt(&self, stop: Stop) -> Stop {
        if self.stop.get().is_none() {
            self.stop.set(Some(stop));
        }
        self.stop.get().unwrap_or(stop)
    }

    /// Charge `bytes`; refused once the step has stopped, and then the step stops.
    pub fn charge(&self, bytes: usize) -> Result<(), Stop> {
        self.check()?;
        if bytes == 0 {
            return Ok(());
        }
        match self.with(|ledger| ledger.charge(bytes)) {
            Some(Ok(())) => {
                self.held.set(self.held.get().saturating_add(bytes));
                Ok(())
            }
            Some(Err(stop)) => Err(self.halt(stop)),
            None => Err(self.halt(Stop::OverBudget)),
        }
    }

    /// Give back `bytes` the step held.
    pub fn credit(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }
        let bytes = bytes.min(self.held.get());
        self.held.set(self.held.get().saturating_sub(bytes));
        self.with(|ledger| ledger.credit(bytes));
    }

    /// Credit everything the step holds: the end of a step that stopped or made nothing it
    /// keeps.
    pub fn credit_all(&self) {
        self.credit(self.held.get());
    }

    /// Read the interrupt through the ledger.
    pub fn poll(&self) -> Result<(), Stop> {
        self.check()?;
        match self.with(|ledger| ledger.poll()) {
            Some(Ok(())) => Ok(()),
            Some(Err(stop)) => Err(self.halt(stop)),
            None => Ok(()),
        }
    }

    /// One node of a walk visited: a poll every [`POLL_INTERVAL`] of them; `Err` once the step
    /// has stopped.
    pub fn visit(&self) -> Result<(), Stop> {
        let visits = self.visits.get().saturating_add(1);
        if visits >= POLL_INTERVAL {
            self.visits.set(0);
            return self.poll();
        }
        self.visits.set(visits);
        self.check()
    }

    /// Room for one more element in `v`: when it is full, its capacity doubles (at least 4),
    /// charged before the allocation.
    pub fn grow<T>(&self, v: &mut Vec<T>) -> Result<(), Stop> {
        if v.len() < v.capacity() {
            return Ok(());
        }
        let wanted = v.capacity().saturating_mul(2).max(MIN_ELEMENTS);
        self.reserve_to(v, wanted)
    }

    /// Room for `additional` more elements in `v`, grown by doubling.
    pub fn grow_by<T>(&self, v: &mut Vec<T>, additional: usize) -> Result<(), Stop> {
        let needed = v.len().saturating_add(additional);
        if needed <= v.capacity() {
            return Ok(());
        }
        let mut wanted = v.capacity().max(MIN_ELEMENTS);
        while wanted < needed {
            wanted = wanted.saturating_mul(2);
        }
        self.reserve_to(v, wanted)
    }

    /// The capacity of `v` raised to `wanted`, the growth charged first.
    fn reserve_to<T>(&self, v: &mut Vec<T>, wanted: usize) -> Result<(), Stop> {
        let had = v.capacity();
        if wanted <= had {
            return Ok(());
        }
        let bytes = wanted.saturating_sub(had).saturating_mul(size_of::<T>());
        self.charge(bytes)?;
        if v.try_reserve_exact(wanted.saturating_sub(v.len())).is_err() {
            self.credit(bytes);
            return Err(self.halt(Stop::OverBudget));
        }
        self.settle(had, wanted, v.capacity(), size_of::<T>());
        Ok(())
    }

    /// The allocator gave more room than asked: the rest is charged too, so that what the step
    /// holds is the capacity.
    fn settle(&self, had: usize, wanted: usize, got: usize, size: usize) {
        if got > wanted {
            let extra = got.saturating_sub(wanted.max(had)).saturating_mul(size);
            if self.charge(extra).is_err() {
                self.held.set(self.held.get().saturating_add(extra));
            }
        }
    }

    /// Push `item` onto `v`, grown first; on a refusal `item` is dropped.
    pub fn push<T>(&self, v: &mut Vec<T>, item: T) -> Result<(), Stop> {
        self.grow(v)?;
        v.push(item);
        Ok(())
    }

    /// An empty vector with room for exactly `n` elements, charged first.
    pub fn with_capacity<T>(&self, n: usize) -> Result<Vec<T>, Stop> {
        let mut v = Vec::new();
        if n > 0 {
            self.reserve_to(&mut v, n)?;
        }
        Ok(v)
    }

    /// A copy of `items` with exactly their number as capacity.
    pub fn copy<T: Clone>(&self, items: &[T]) -> Result<Vec<T>, Stop> {
        let mut v = self.with_capacity(items.len())?;
        v.extend_from_slice(items);
        Ok(v)
    }

    /// The UTF-16 code units of `text`, with exactly their number as capacity.
    pub fn utf16(&self, text: &str) -> Result<Vec<u16>, Stop> {
        let mut v = self.with_capacity(text.encode_utf16().count())?;
        v.extend(text.encode_utf16());
        Ok(v)
    }

    /// A copy of `text` with exactly its length as capacity.
    pub fn string(&self, text: &str) -> Result<String, Stop> {
        let mut s = self.string_with_capacity(text.len())?;
        s.push_str(text);
        Ok(s)
    }

    /// An empty string with room for exactly `bytes` bytes, charged first.
    pub fn string_with_capacity(&self, bytes: usize) -> Result<String, Stop> {
        let mut s = String::new();
        if bytes > 0 {
            self.charge(bytes)?;
            if s.try_reserve_exact(bytes).is_err() {
                self.credit(bytes);
                return Err(self.halt(Stop::OverBudget));
            }
            self.settle(0, bytes, s.capacity(), 1);
        }
        Ok(s)
    }

    /// Push the ASCII character `c` onto `s`, its capacity doubled (at least 4) when full.
    pub fn push_char(&self, s: &mut String, c: char) -> Result<(), Stop> {
        let needed = s.len().saturating_add(c.len_utf8());
        if needed > s.capacity() {
            let had = s.capacity();
            let mut wanted = had.saturating_mul(2).max(MIN_ELEMENTS);
            while wanted < needed {
                wanted = wanted.saturating_mul(2);
            }
            let bytes = wanted.saturating_sub(had);
            self.charge(bytes)?;
            if s.try_reserve_exact(wanted.saturating_sub(s.len())).is_err() {
                self.credit(bytes);
                return Err(self.halt(Stop::OverBudget));
            }
            self.settle(had, wanted, s.capacity(), 1);
        }
        s.push(c);
        Ok(())
    }

    /// A boxed `value`, its bytes charged first.
    pub fn boxed<T>(&self, value: T) -> Result<Box<T>, Stop> {
        self.charge(size_of::<T>())?;
        Ok(Box::new(value))
    }

    /// Drop `value` and credit what it owns on the heap.
    pub fn release<T: HeapBytes>(&self, value: T) {
        let bytes = value.heap_bytes();
        drop(value);
        self.credit(bytes);
    }

    /// Insert `key` into `set`, an entry charged first when it is new; whether it was new. A key
    /// not inserted (there already, or refused) is dropped and what it owns credited.
    pub fn insert<K: Ord + HeapBytes>(&self, set: &mut BTreeSet<K>, key: K) -> Result<bool, Stop> {
        if set.contains(&key) {
            self.release(key);
            return Ok(false);
        }
        if let Err(stop) = self.charge(map_entry_bytes::<K, ()>()) {
            self.release(key);
            return Err(stop);
        }
        set.insert(key);
        Ok(true)
    }

    /// Insert `key` with `value` into `map`, an entry charged first when the key is new; a value
    /// replaced, and a key or value not inserted, are dropped and what they own credited.
    pub fn insert_entry<K: Ord + HeapBytes, V: HeapBytes>(
        &self,
        map: &mut BTreeMap<K, V>,
        key: K,
        value: V,
    ) -> Result<(), Stop> {
        if let Some(slot) = map.get_mut(&key) {
            let old = std::mem::replace(slot, value);
            self.release(old);
            self.release(key);
            return Ok(());
        }
        if let Err(stop) = self.charge(map_entry_bytes::<K, V>()) {
            self.release(key);
            self.release(value);
            return Err(stop);
        }
        map.insert(key, value);
        Ok(())
    }

    /// Remove `key` from `map`: its value, with the entry and the key credited.
    pub fn remove_entry<K: Ord + HeapBytes, V: HeapBytes>(
        &self,
        map: &mut BTreeMap<K, V>,
        key: &K,
    ) -> Option<V> {
        let (k, v) = map.remove_entry(key)?;
        self.credit(map_entry_bytes::<K, V>());
        self.release(k);
        Some(v)
    }
}

/// A ledger for the tests of the frontend: it counts what it is asked, refuses a charge past
/// `limit`, and answers `Interrupted` from the poll numbered `interrupt_at` (from 1).
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct Counting {
    pub(crate) charged: usize,
    pub(crate) credited: usize,
    pub(crate) high: usize,
    pub(crate) polls: usize,
    pub(crate) limit: Option<usize>,
    pub(crate) interrupt_at: Option<usize>,
}

#[cfg(test)]
impl Counting {
    /// What is charged now.
    pub(crate) fn held(&self) -> usize {
        self.charged.saturating_sub(self.credited)
    }
}

#[cfg(test)]
impl Ledger for Counting {
    fn charge(&mut self, bytes: usize) -> Result<(), Stop> {
        let held = self.held().saturating_add(bytes);
        if self.limit.is_some_and(|limit| held > limit) {
            return Err(Stop::OverBudget);
        }
        self.charged = self.charged.saturating_add(bytes);
        self.high = self.high.max(held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.credited = self.credited.saturating_add(bytes);
    }

    fn poll(&mut self) -> Result<(), Stop> {
        self.polls += 1;
        if self.interrupt_at == Some(self.polls) {
            return Err(Stop::Interrupted);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Counting, Meter, POLL_INTERVAL, Stop};
    use crate::{Failure, ParseOptions, parse_script_with};

    const OPTIONS: ParseOptions = ParseOptions { max_nesting: 200 };

    /// `x = 1;` lines until the source has at least `units` code units.
    fn flat(units: usize) -> Vec<u16> {
        let mut source = Vec::new();
        while source.len() < units {
            source.extend("x = 1;\n".encode_utf16());
        }
        source
    }

    /// A source of many nestings and every node that owns a vector or a string.
    const NESTED: &str = r#"
        "use strict";
        var a = [1, , "two", 3.5e1, /re+/gi, `t${a}u${[a]}v`, `plain`];
        let b = { k: 1, "s": 'qA', 7: a, get g() { return 1; }, set g(v) {}, m(x, y) {}, a };
        const c = a ? b : function named(p, q, r) { return p + q * r - (p, q, r); };
        label: for (let i = 0; i < 10; i++) { if (i) continue label; else break label; }
        for (var k in b) { switch (k) { case "a": c(); break; default: new c(1, 2); } }
        for (const v in b) {}
        try { throw new Error("e"); } catch (e) { a.push(e); } finally { b.k = a?.[0]?.x?.(1); }
        try {} catch {}
        do { a.pop(); } while (a.length > 3);
        while (false) {}
        function outer(n) {
            function inner(m) { return m ? inner(m - 1) : [n, { n }, typeof n, void 0, !n, ~n]; }
            return (inner(n) || outer(n - 1) && null) ?? this;
        }
        a = b = c; a += 1; a ||= 2; a ??= 3; delete b.k; -a; +a; a--; ++a;
        ((((((((((a))))))))));
        [[[[[[[[[[a]]]]]]]]]];
    "#;

    /// Long distinct identifiers, each declared and referred to.
    fn identifiers() -> Vec<u16> {
        let mut text = String::new();
        for i in 0..500 {
            text.push_str(&format!(
                "var identifier_number_{i}_long_enough = other_identifier_{i} + {i};\n"
            ));
        }
        text.encode_utf16().collect()
    }

    /// The flat source of 300,000 code units polls at least once per 4,096 code units.
    #[test]
    fn a_parse_polls_once_per_4096_code_units() {
        let source = flat(300_000);
        let mut ledger = Counting::default();
        let parsed = parse_script_with(&source, OPTIONS, &mut ledger);
        assert!(parsed.is_ok());
        let floor = 300_000 / POLL_INTERVAL;
        assert!(
            ledger.polls >= floor,
            "{} polls for {} code units, at least {floor} wanted",
            ledger.polls,
            source.len()
        );
    }

    /// The third poll answers `Interrupted`: the parse answers that stop, the lexer stopped
    /// within 3 × 4,096 code units of the start, and nothing stays charged.
    #[test]
    fn a_stop_from_poll_ends_the_parse_within_4096_code_units() {
        let source = flat(300_000);
        let mut ledger = Counting {
            interrupt_at: Some(3),
            ..Counting::default()
        };
        {
            let meter = Meter::new(&mut ledger);
            let mut parser = crate::parser::Parser::new(&source, OPTIONS.max_nesting, &meter);
            assert!(parser.script().is_err());
            assert_eq!(meter.stopped(), Some(Stop::Interrupted));
            assert!(
                parser.lexer.pos <= 3 * POLL_INTERVAL,
                "the lexer stopped at {}",
                parser.lexer.pos
            );
        }
        assert_eq!(ledger.polls, 3);
        let mut ledger = Counting {
            interrupt_at: Some(3),
            ..Counting::default()
        };
        let parsed = parse_script_with(&source, OPTIONS, &mut ledger);
        assert_eq!(parsed.err(), Some(Failure::Stopped(Stop::Interrupted)));
        assert_eq!(ledger.polls, 3);
        assert_eq!(ledger.held(), 0);
    }

    /// For a flat, a nested and an identifier-heavy source, what the ledger holds when the parse
    /// returns is the tree's capacity bytes as an independent walk sums them, and the tree says
    /// so.
    #[test]
    fn the_charge_covers_the_tree() {
        let sources = [
            ("flat", flat(20_000)),
            ("nested", NESTED.encode_utf16().collect()),
            ("identifiers", identifiers()),
        ];
        for (name, source) in sources {
            let mut ledger = Counting::default();
            let script = match parse_script_with(&source, OPTIONS, &mut ledger) {
                Ok(script) => script,
                Err(failure) => panic!("{name}: {failure:?}"),
            };
            let walk = script.capacity_bytes();
            println!("{name}: held {} walk {walk}", ledger.held());
            assert!(walk > 0, "{name}");
            assert_eq!(ledger.held(), walk, "{name}: held against the walk");
            assert_eq!(script.charged, walk, "{name}: the tree's own record");
        }
    }

    /// A charge refused half way through the parse stops it with `OverBudget`, makes no error,
    /// and leaves nothing charged.
    #[test]
    fn a_refused_charge_stops_the_parse_with_everything_credited() {
        let source = flat(20_000);
        let mut ledger = Counting::default();
        let script = parse_script_with(&source, OPTIONS, &mut ledger);
        let full = match script {
            Ok(script) => script.charged,
            Err(failure) => panic!("{failure:?}"),
        };
        assert!(ledger.high > 0);
        let mut ledger = Counting {
            limit: Some(ledger.high / 2),
            ..Counting::default()
        };
        let parsed = parse_script_with(&source, OPTIONS, &mut ledger);
        assert_eq!(parsed.err(), Some(Failure::Stopped(Stop::OverBudget)));
        assert!(ledger.charged > 0 && full > 0);
        assert_eq!(ledger.held(), 0, "{ledger:?}");
    }
}
