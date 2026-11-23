//! CompilePattern (`#sec-compilepattern`): the program of a pattern's tree (`program.rs`).
//!
//! The compile step is one loop over a worklist (`docs/design/engine.md`, decision 7, "No Rust
//! recursion in the parser and the compiler of a pattern"): a node is emitted by pushing what
//! comes after its children onto the list before its children, and the targets of the jumps
//! that are not known yet are patched when the list reaches them. Each node emits a bounded
//! number of instructions (a quantifier `{n,m}` is a counter, not n copies; the clearing of the
//! captures of a quantified atom names the range of its group numbers, since the groups inside
//! an atom are numbered one after the other), so the program is linear in the tree; the one
//! part above linear is the sort of a class's n ranges, n log n, and of the names of the
//! groups. Every vector grows through the [`Budget`] before it grows; the worklist and the
//! other lists of the step are credited when it ends. The compile step charges no step and
//! polls the interrupt at least once per 4,096 nodes and class items it visits.
//!
//! `\d`, `\w` and `\b` are those of `#sec-compiletocharset` and
//! `#sec-runtime-semantics-iswordchar-abstract-operation`, with WordCharacters
//! (`#sec-wordcharacters`) holding U+017F and U+212A too under `u` or `v` with `i`; `\s` is
//! WhiteSpace and LineTerminator. A property escape `\p{...}` is the property `swell-unicode`
//! gives its name, and `\P{...}` its complement; the class names it and the matcher tests it
//! against the tables of `swell-unicode`, so no range of a table is copied into the program.
//! The modifiers `i`, `m` and `s` of `(?ims-ims:...)` set the mode of their atoms
//! (`#sec-updatemodifiers`).
//!
//! Under `i` (the flag, or a modifier around the atom; `#sec-compilepattern`) a character with
//! another of its canonical value (Canonicalize, `canonicalize.rs`) is [`Inst::Fold`] of that
//! value, a class and a backreference are marked to compare canonical values, and `\b` and
//! `\B` under `u` or `v` read the WordCharacters of `i`. A class under `i` is compiled as
//! without it: its ranges and property escapes are tested against the characters of the
//! canonical value of a character when the class is run, so the compile step builds no
//! closure and its work stays linear in the pattern, also for many classes of wide ranges.

use std::fmt;

use swell_unicode::Property;

use crate::ast::{Assertion, ClassEscape, ClassItem, List, Look, Name, Node, NodeId, Pattern};
use crate::budget::{Budget, Meter, Stop};
use crate::canonicalize::{canonicalize, has_no_other_case};
use crate::program::{ClassSet, Inst, MAX_CODE_POINT, Program, UNPATCHED};

/// A construct CompilePattern refuses until the story that brings it: none since
/// `story:regexp-case-insensitive-matching-and-modifiers` brought the flag `i` and the
/// modifier `i`, so no value of this type exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileRefusal {}

impl CompileRefusal {
    /// The construct the message names.
    pub fn construct(self) -> &'static str {
        match self {}
    }
}

/// Why CompilePattern answered no program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileError {
    /// A construct refused until a later story of slice 5; nothing is charged. CompilePattern
    /// refuses none at present ([`CompileRefusal`]).
    Refused(CompileRefusal),
    /// The budget stopped the compile step; everything it charged is credited.
    Stopped(Stop),
}

impl CompileError {
    /// Whether this is a refusal of a construct a later story of slice 5 brings.
    pub fn is_refusal(&self) -> bool {
        matches!(self, CompileError::Refused(_))
    }

    /// The message: for a refusal that of the `SyntaxError`, `<construct> is not supported
    /// before slice 5`.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Refused(refusal) => match *refusal {},
            CompileError::Stopped(stop) => write!(f, "the compile step stopped: {stop:?}"),
        }
    }
}

impl From<Stop> for CompileError {
    fn from(stop: Stop) -> CompileError {
        CompileError::Stopped(stop)
    }
}

/// CompilePattern of `pattern`, every growth charged to `budget`: the program, or the stop of
/// the budget with everything credited.
pub fn compile_pattern(
    pattern: &Pattern,
    budget: &mut dyn Budget,
) -> Result<Program, CompileError> {
    let mut m = Meter::new(budget);
    let built = build(pattern, &mut m);
    match (built, m.stopped()) {
        (Ok(mut program), None) => {
            program.charged = m.held();
            Ok(program)
        }
        (Ok(program), Some(stop)) => {
            drop(program);
            m.credit_all();
            Err(CompileError::Stopped(stop))
        }
        (Err(error), stop) => {
            m.credit_all();
            Err(stop.map_or(error, CompileError::Stopped))
        }
    }
}

/// The mode an atom is compiled in: `i`, `m` and `s` as the flags and the modifier groups
/// around it set them, and the direction.
#[derive(Clone, Copy)]
struct Mode {
    ignore_case: bool,
    multiline: bool,
    dot_all: bool,
    back: bool,
}

/// One item of the worklist.
#[derive(Clone, Copy)]
enum Task {
    /// Emit the node.
    Node(NodeId, Mode),
    /// Emit one instruction: the end of a group after its body.
    Emit(Inst),
    /// The alternatives of a Disjunction from `next` on; `split` is the choice before the
    /// alternative just emitted, and `chain` the jumps to the Disjunction's end, linked
    /// through their targets.
    Alternatives {
        list: List,
        next: u32,
        mode: Mode,
        split: u32,
        chain: u32,
    },
    /// The tail of a quantifier's loop, after its atom.
    RepeatEnd { head: u32, counter: u32, min: u32 },
    /// The exit of a lookaround, after its body.
    LookEnd {
        start: u32,
        look: u32,
        negative: bool,
    },
}

/// The groups inside a node: the first and the last number, or none.
#[derive(Clone, Copy)]
struct Span {
    first: u32,
    last: u32,
}

const NO_SPAN: Span = Span {
    first: u32::MAX,
    last: 0,
};

impl Span {
    fn join(self, other: Span) -> Span {
        Span {
            first: self.first.min(other.first),
            last: self.last.max(other.last),
        }
    }

    fn is_empty(self) -> bool {
        self.first > self.last
    }
}

fn index32(n: usize) -> Result<u32, Stop> {
    u32::try_from(n).map_err(|_| Stop::MemoryBudget)
}

fn build(pattern: &Pattern, m: &mut Meter<'_>) -> Result<Program, CompileError> {
    let flags = pattern.flags;
    let mut c = Compiler {
        pattern,
        m,
        unicode: flags.either_unicode(),
        code: Vec::new(),
        ranges: Vec::new(),
        classes: Vec::new(),
        properties: Vec::new(),
        named: Vec::new(),
        spans: Vec::new(),
        tasks: Vec::new(),
        sorted: Vec::new(),
        counters: 0,
        looks: 0,
    };
    let result = c.run();
    let Compiler {
        m,
        code,
        ranges,
        classes,
        properties,
        named,
        spans,
        tasks,
        sorted,
        counters,
        looks,
        ..
    } = c;
    m.release(spans);
    m.release(tasks);
    m.release(sorted);
    result?;
    let mut names = Vec::new();
    m.grow_by(&mut names, pattern.names.len())?;
    names.extend_from_slice(&pattern.names);
    let text = m.copy(&pattern.text)?;
    Ok(Program {
        code,
        ranges,
        classes,
        properties,
        named,
        names,
        text,
        groups: pattern.groups,
        counters,
        looks,
        flags,
        charged: 0,
    })
}

struct Compiler<'p, 'm, 'b> {
    pattern: &'p Pattern,
    m: &'m mut Meter<'b>,
    unicode: bool,
    code: Vec<Inst>,
    ranges: Vec<(u32, u32)>,
    classes: Vec<ClassSet>,
    properties: Vec<(Property, bool)>,
    named: Vec<u32>,
    spans: Vec<Span>,
    tasks: Vec<Task>,
    sorted: Vec<(u32, Name)>,
    counters: u32,
    looks: u32,
}

impl Compiler<'_, '_, '_> {
    fn run(&mut self) -> Result<(), Stop> {
        self.spans()?;
        self.names()?;
        let flags = self.pattern.flags;
        let mode = Mode {
            ignore_case: flags.ignore_case(),
            multiline: flags.multiline(),
            dot_all: flags.dot_all(),
            back: false,
        };
        self.push(Task::Node(self.pattern.root, mode))?;
        while let Some(task) = self.tasks.pop() {
            self.m.visit()?;
            match task {
                Task::Node(id, mode) => self.node(id, mode)?,
                Task::Emit(inst) => {
                    self.emit(inst)?;
                }
                Task::Alternatives {
                    list,
                    next,
                    mode,
                    split,
                    chain,
                } => self.alternatives(list, next, mode, split, chain)?,
                Task::RepeatEnd { head, counter, min } => {
                    self.emit(Inst::RepeatTail { counter, min, head })?;
                    let exit = self.here()?;
                    if let Some(Inst::Repeat { exit: target, .. }) =
                        self.code.get_mut(head as usize)
                    {
                        *target = exit;
                    }
                }
                Task::LookEnd {
                    start,
                    look,
                    negative,
                } => {
                    self.emit(Inst::LookEnd { look, negative })?;
                    let end = self.here()?;
                    if let Some(Inst::LookStart { end: target, .. }) =
                        self.code.get_mut(start as usize)
                    {
                        *target = end;
                    }
                }
            }
        }
        self.emit(Inst::Match)?;
        Ok(())
    }

    /// The groups inside each node, in one pass over the nodes: a child comes before its
    /// parent.
    fn spans(&mut self) -> Result<(), Stop> {
        let pattern = self.pattern;
        let nodes = &pattern.nodes;
        self.m.grow_by(&mut self.spans, nodes.len())?;
        for node in nodes {
            self.m.visit()?;
            let of = |spans: &[Span], id: NodeId| spans.get(id.index()).copied().unwrap_or(NO_SPAN);
            let span = match *node {
                Node::Disjunction(list) | Node::Alternative(list) => pattern
                    .children(list)
                    .iter()
                    .fold(NO_SPAN, |span, &child| span.join(of(&self.spans, child))),
                Node::Look { body, .. }
                | Node::Quantified { body, .. }
                | Node::NonCapturing { body }
                | Node::Modified { body, .. } => of(&self.spans, body),
                Node::Group { number, body, .. } => Span {
                    first: number,
                    last: number,
                }
                .join(of(&self.spans, body)),
                _ => NO_SPAN,
            };
            self.spans.push(span);
        }
        Ok(())
    }

    /// The named groups sorted by name and then by number, so that a named backreference
    /// names the run of the groups of its name in [`Program::named`].
    fn names(&mut self) -> Result<(), Stop> {
        let pattern = self.pattern;
        let names = &pattern.names;
        if names.is_empty() {
            return Ok(());
        }
        self.m.grow_by(&mut self.sorted, names.len())?;
        self.sorted.extend_from_slice(names);
        self.sorted
            .sort_unstable_by(|a, b| pattern.name(a.1).cmp(pattern.name(b.1)).then(a.0.cmp(&b.0)));
        self.m.grow_by(&mut self.named, names.len())?;
        self.named
            .extend(self.sorted.iter().map(|&(number, _)| number));
        Ok(())
    }

    fn push(&mut self, task: Task) -> Result<(), Stop> {
        self.m.push(&mut self.tasks, task)
    }

    fn here(&self) -> Result<u32, Stop> {
        index32(self.code.len())
    }

    fn emit(&mut self, inst: Inst) -> Result<u32, Stop> {
        let at = self.here()?;
        self.m.push(&mut self.code, inst)?;
        Ok(at)
    }

    fn node(&mut self, id: NodeId, mode: Mode) -> Result<(), Stop> {
        let Some(&node) = self.pattern.nodes.get(id.index()) else {
            return Ok(());
        };
        let back = mode.back;
        match node {
            Node::Alternative(list) => {
                // The terms in the order they are emitted: left to right, or right to left
                // inside a lookbehind (`#sec-compilesubpattern`, Alternative, direction
                // backward). The worklist takes the last pushed first.
                let pattern = self.pattern;
                let children = pattern.children(list);
                self.m.grow_by(&mut self.tasks, children.len())?;
                if back {
                    for &child in children {
                        self.tasks.push(Task::Node(child, mode));
                    }
                } else {
                    for &child in children.iter().rev() {
                        self.tasks.push(Task::Node(child, mode));
                    }
                }
            }
            Node::Disjunction(list) => self.push(Task::Alternatives {
                list,
                next: 0,
                mode,
                split: UNPATCHED,
                chain: UNPATCHED,
            })?,
            Node::Assertion(assertion) => {
                self.emit(match assertion {
                    Assertion::Start => Inst::LineStart {
                        multiline: mode.multiline,
                    },
                    Assertion::End => Inst::LineEnd {
                        multiline: mode.multiline,
                    },
                    Assertion::WordBoundary => Inst::WordBoundary {
                        negated: false,
                        fold: self.unicode && mode.ignore_case,
                    },
                    Assertion::NotWordBoundary => Inst::WordBoundary {
                        negated: true,
                        fold: self.unicode && mode.ignore_case,
                    },
                })?;
            }
            Node::Look { look, body } => {
                let negative = matches!(look, Look::NotAhead | Look::NotBehind);
                let behind = matches!(look, Look::Behind | Look::NotBehind);
                let register = self.looks;
                self.looks = self.looks.saturating_add(1);
                let start = self.emit(Inst::LookStart {
                    look: register,
                    negative,
                    end: UNPATCHED,
                })?;
                self.push(Task::LookEnd {
                    start,
                    look: register,
                    negative,
                })?;
                self.push(Task::Node(
                    body,
                    Mode {
                        back: behind,
                        ..mode
                    },
                ))?;
            }
            Node::Quantified { body, quantifier } => {
                if quantifier.max == Some(0) {
                    return Ok(());
                }
                let counter = self.counters;
                self.counters = self.counters.saturating_add(1);
                self.emit(Inst::RepeatInit { counter })?;
                let head = self.emit(Inst::Repeat {
                    counter,
                    min: quantifier.min,
                    max: quantifier.max,
                    greedy: quantifier.greedy,
                    exit: UNPATCHED,
                })?;
                let span = self.spans.get(body.index()).copied().unwrap_or(NO_SPAN);
                if !span.is_empty() {
                    self.emit(Inst::Clear {
                        first: span.first,
                        last: span.last,
                    })?;
                }
                self.push(Task::RepeatEnd {
                    head,
                    counter,
                    min: quantifier.min,
                })?;
                self.push(Task::Node(body, mode))?;
            }
            Node::Character(c) => {
                let one_unit = !self.unicode || (c <= 0xFFFF && !(0xD800..=0xDFFF).contains(&c));
                let inst = match u16::try_from(c) {
                    _ if mode.ignore_case && !has_no_other_case(c, self.unicode) => Inst::Fold {
                        canonical: canonicalize(c, self.unicode),
                        back,
                    },
                    Ok(unit) if one_unit => Inst::Unit { unit, back },
                    _ => Inst::Point { point: c, back },
                };
                self.emit(inst)?;
            }
            Node::Dot => {
                self.emit(Inst::Any {
                    dot_all: mode.dot_all,
                    back,
                })?;
            }
            Node::Escape(escape) => {
                let class = self.class(false, &[ClassItem::Escape(escape)], mode.ignore_case)?;
                self.emit(Inst::Class { class, back })?;
            }
            Node::Class { negated, items } => {
                let pattern = self.pattern;
                let class = self.class(negated, pattern.class_items(items), mode.ignore_case)?;
                self.emit(Inst::Class { class, back })?;
            }
            Node::Group { number, body, .. } => {
                // Inside a lookbehind the end of the group is reached first.
                let start = number.saturating_mul(2);
                let end = start.saturating_add(1);
                let (first, last) = if back { (end, start) } else { (start, end) };
                self.emit(Inst::Save { slot: first })?;
                self.push(Task::Emit(Inst::Save { slot: last }))?;
                self.push(Task::Node(body, mode))?;
            }
            Node::NonCapturing { body } => self.push(Task::Node(body, mode))?,
            Node::Modified { add, remove, body } => {
                let inner = Mode {
                    ignore_case: (mode.ignore_case || add.ignore_case()) && !remove.ignore_case(),
                    multiline: (mode.multiline || add.multiline()) && !remove.multiline(),
                    dot_all: (mode.dot_all || add.dot_all()) && !remove.dot_all(),
                    back,
                };
                self.push(Task::Node(body, inner))?;
            }
            Node::Backreference(group) => {
                self.emit(Inst::Backref {
                    group,
                    back,
                    fold: mode.ignore_case,
                })?;
            }
            Node::NamedBackreference(name) => {
                self.named_backreference(name, back, mode.ignore_case)?;
            }
        }
        Ok(())
    }

    /// The alternatives of a Disjunction from `next` on:
    ///
    /// ```text
    ///     Split L1, N1
    /// L1: <alternative 1>
    ///     Jump End
    /// N1: Split L2, N2
    /// L2: <alternative 2>
    ///     Jump End
    /// N2: <alternative n>
    /// End:
    /// ```
    fn alternatives(
        &mut self,
        list: List,
        next: u32,
        mode: Mode,
        split: u32,
        chain: u32,
    ) -> Result<(), Stop> {
        let pattern = self.pattern;
        let children = pattern.children(list);
        let count = index32(children.len())?;
        let mut chain = chain;
        if next > 0 && next < count {
            chain = self.emit(Inst::Jump { to: chain })?;
            let second = self.here()?;
            if let Some(Inst::Split { second: target, .. }) = self.code.get_mut(split as usize) {
                *target = second;
            }
        }
        if next >= count {
            // Every jump of the chain goes to the end.
            let end = self.here()?;
            let mut at = chain;
            while let Some(Inst::Jump { to }) = self.code.get_mut(at as usize) {
                at = *to;
                *to = end;
            }
            return Ok(());
        }
        let Some(&child) = children.get(next as usize) else {
            return Ok(());
        };
        let split = if next + 1 < count {
            let first = self.here()?.saturating_add(1);
            self.emit(Inst::Split {
                first,
                second: UNPATCHED,
            })?
        } else {
            UNPATCHED
        };
        self.push(Task::Alternatives {
            list,
            next: next + 1,
            mode,
            split,
            chain,
        })?;
        self.push(Task::Node(child, mode))
    }

    /// `\k<name>`: the groups of the name, one [`Inst::Backref`] when there is one.
    fn named_backreference(&mut self, name: Name, back: bool, fold: bool) -> Result<(), Stop> {
        let pattern = self.pattern;
        let text = pattern.name(name);
        let first = self
            .sorted
            .partition_point(|&(_, n)| pattern.name(n) < text);
        let end = self
            .sorted
            .partition_point(|&(_, n)| pattern.name(n) <= text);
        let len = end.saturating_sub(first);
        if len == 1
            && let Some(&group) = self.named.get(first)
        {
            self.emit(Inst::Backref { group, back, fold })?;
            return Ok(());
        }
        self.emit(Inst::NamedBackref {
            start: index32(first)?,
            len: index32(len)?,
            back,
            fold,
        })?;
        Ok(())
    }

    /// A class of `items` (`#sec-compiletocharset`): its ranges sorted and joined where they
    /// meet, n log n for n ranges; `\s` and `\S` as predicates beside them; and each property
    /// escape, `\p{...}` or its complement `\P{...}`, as its property and a flag, tested against
    /// the tables of `swell-unicode` when the class is run and never copied into the program.
    /// Under `i` (`fold`) the class is marked to test the characters of a canonical value, and
    /// `\w` and `\W` under `u` or `v` take the WordCharacters of `i`. Its number.
    fn class(&mut self, negated: bool, items: &[ClassItem], fold: bool) -> Result<u32, Stop> {
        let start = self.ranges.len();
        let first_property = self.properties.len();
        let mut set = ClassSet {
            start: index32(start)?,
            len: 0,
            properties: index32(first_property)?,
            properties_len: 0,
            negated,
            space: false,
            not_space: false,
            fold,
        };
        for &item in items {
            self.m.visit()?;
            match item {
                ClassItem::Character(c) => self.range(c, c)?,
                ClassItem::Range(a, b) => self.range(a, b)?,
                ClassItem::Escape(ClassEscape::Space) => set.space = true,
                ClassItem::Escape(ClassEscape::NotSpace) => set.not_space = true,
                ClassItem::Escape(ClassEscape::Property { property, negated }) => {
                    self.m.push(&mut self.properties, (property, negated))?;
                }
                ClassItem::Escape(escape) => {
                    for &(a, b) in escape_ranges(escape, self.unicode && fold) {
                        self.range(a, b)?;
                    }
                }
            }
        }
        let own = self.ranges.get_mut(start..).unwrap_or(&mut []);
        own.sort_unstable();
        // Join in place: `kept` ranges are final, each later one joins the last or follows it.
        let mut kept = 0usize;
        for i in 0..own.len() {
            let Some(&(a, b)) = own.get(i) else { break };
            if kept > 0
                && let Some(last) = own.get_mut(kept - 1)
                && a <= last.1.saturating_add(1)
            {
                last.1 = last.1.max(b);
                continue;
            }
            if let Some(slot) = own.get_mut(kept) {
                *slot = (a, b);
            }
            kept += 1;
        }
        self.ranges.truncate(start.saturating_add(kept));
        set.len = index32(kept)?;
        set.properties_len = index32(self.properties.len().saturating_sub(first_property))?;
        let number = index32(self.classes.len())?;
        self.m.push(&mut self.classes, set)?;
        Ok(number)
    }

    fn range(&mut self, a: u32, b: u32) -> Result<(), Stop> {
        self.m.push(&mut self.ranges, (a, b))
    }
}

/// The ranges of `\d`, `\D`, `\w` and `\W` (`#sec-compiletocharset`, CharacterClassEscape),
/// the complements up to U+10FFFF; `\s` and `\S` are predicates of the class. With `fold`
/// (under `u` or `v` with `i`) WordCharacters holds U+017F and U+212A too
/// (`#sec-wordcharacters`), and `\W`, its complement, neither.
fn escape_ranges(escape: ClassEscape, fold: bool) -> &'static [(u32, u32)] {
    const DIGIT: &[(u32, u32)] = &[(0x30, 0x39)];
    const NOT_DIGIT: &[(u32, u32)] = &[(0, 0x2F), (0x3A, MAX_CODE_POINT)];
    const WORD: &[(u32, u32)] = &[(0x30, 0x39), (0x41, 0x5A), (0x5F, 0x5F), (0x61, 0x7A)];
    const NOT_WORD: &[(u32, u32)] = &[
        (0, 0x2F),
        (0x3A, 0x40),
        (0x5B, 0x5E),
        (0x60, 0x60),
        (0x7B, MAX_CODE_POINT),
    ];
    const WORD_FOLDED: &[(u32, u32)] = &[
        (0x30, 0x39),
        (0x41, 0x5A),
        (0x5F, 0x5F),
        (0x61, 0x7A),
        (0x017F, 0x017F),
        (0x212A, 0x212A),
    ];
    const NOT_WORD_FOLDED: &[(u32, u32)] = &[
        (0, 0x2F),
        (0x3A, 0x40),
        (0x5B, 0x5E),
        (0x60, 0x60),
        (0x7B, 0x017E),
        (0x0180, 0x2129),
        (0x212B, MAX_CODE_POINT),
    ];
    match escape {
        ClassEscape::Digit => DIGIT,
        ClassEscape::NotDigit => NOT_DIGIT,
        ClassEscape::Word if fold => WORD_FOLDED,
        ClassEscape::Word => WORD,
        ClassEscape::NotWord if fold => NOT_WORD_FOLDED,
        ClassEscape::NotWord => NOT_WORD,
        ClassEscape::Space | ClassEscape::NotSpace | ClassEscape::Property { .. } => &[],
    }
}
