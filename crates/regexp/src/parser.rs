//! ParsePattern (`#sec-parsepattern`, amended by B.1.2.9): the grammar of 22.2.1
//! (`#sec-patterns`) with [UnicodeMode] under `u`, and without `u` or `v` the grammar of B.1.2
//! (`#sec-regular-expressions-patterns`), with the early errors of both clauses (`early.rs`).
//!
//! The parser is one loop with explicit stacks (`docs/design/engine.md`, decision 7, "No Rust
//! recursion in the parser and the compiler of a pattern"): one entry per open group, lookaround
//! and modifier group on a stack of its own, the terms of the open alternatives and the finished
//! alternatives of the open levels on two more, each charged before it grows. The level that
//! would pass the caller's nesting limit, counted from zero at the start of the pattern, is the
//! error `nesting deeper than parser_nesting (<limit>)`.
//!
//! Without `u` the parser takes the count of capturing groups first, by a scan of the pattern,
//! since B.1.2 reads `\n` as a backreference only when n is not past that count
//! (AtomEscape :: DecimalEscape "but only if"), and as a legacy octal escape or an identity
//! escape otherwise. A pattern parsed without [NamedCaptureGroups] that has a group name is
//! parsed again with it (B.1.2.9, step 3).
//!
//! The parser charges no step; it polls the interrupt before it reads the pattern and then at
//! least once per 4,096 code units it reads and per 4,096 nodes it makes or visits.

use crate::ast::{
    Assertion, ClassEscape, ClassItem, List, Look, MODIFIER_I, MODIFIER_M, MODIFIER_S, Modifiers,
    Name, Node, NodeId, Pattern, Quantifier,
};
use crate::budget::{Budget, Meter, Stop};
use crate::early::{self, ClassAtom, Names};
use crate::error::{ErrorKind, ParseError, PatternError, Refusal};
use crate::flags::Flags;

/// ParsePattern of `pattern`, its code units, with `flags`, and with `nesting_limit` the most
/// groups, lookarounds and modifier groups open at once (the host value `parser_nesting`), every
/// growth charged to `budget`: the tree, or one error, or the stop of the budget with everything
/// credited. The flag `v` is refused until the story that brings it.
pub fn parse_pattern(
    pattern: &[u16],
    flags: Flags,
    nesting_limit: u32,
    budget: &mut dyn Budget,
) -> Result<Pattern, ParseError> {
    parse_counted(pattern, flags, nesting_limit, budget).map(|(tree, _)| tree)
}

/// For the tests of the cost model (`docs/design/engine.md`, decision 7, "What one step of the
/// matcher is"): the parse of [`parse_pattern`], the tree released to `budget`, answering the
/// count of visits of the group-name check: each name declared or looked up, and each level
/// the union-find of the duplicate check walked.
#[doc(hidden)]
pub fn parse_name_visits(
    pattern: &[u16],
    flags: Flags,
    nesting_limit: u32,
    budget: &mut dyn Budget,
) -> Result<u64, ParseError> {
    let (tree, visits) = parse_counted(pattern, flags, nesting_limit, budget)?;
    tree.release(budget);
    Ok(visits)
}

fn parse_counted(
    pattern: &[u16],
    flags: Flags,
    nesting_limit: u32,
    budget: &mut dyn Budget,
) -> Result<(Pattern, u64), ParseError> {
    if flags.unicode_sets() {
        return Err(ParseError::Pattern(PatternError::new(
            ErrorKind::Refused(Refusal::SetsFlag),
            0,
        )));
    }
    let mut m = Meter::new(budget);
    let parsed = parse_with(pattern, flags, nesting_limit, &mut m);
    match (parsed, m.stopped()) {
        (Ok(mut tree), None) => {
            tree.charged = m.held();
            Ok((tree, m.name_visits()))
        }
        (Ok(tree), Some(stop)) => {
            drop(tree);
            m.credit_all();
            Err(ParseError::Stopped(stop))
        }
        (Err(error), stop) => {
            m.credit_all();
            Err(stop.map_or(error, ParseError::Stopped))
        }
    }
}

fn parse_with(
    pattern: &[u16],
    flags: Flags,
    limit: u32,
    m: &mut Meter<'_>,
) -> Result<Pattern, ParseError> {
    m.poll()?;
    let unicode = flags.either_unicode();
    let groups = if unicode {
        0
    } else {
        count_groups(pattern, m)?
    };
    let first = Parser::new(pattern, flags, unicode, unicode, groups, limit, m)?.run()?;
    if unicode || first.names.is_empty() {
        return Ok(first);
    }
    let bytes = first.capacity_bytes();
    drop(first);
    m.credit(bytes);
    Parser::new(pattern, flags, unicode, true, groups, limit, m)?.run()
}

/// The count of left-capturing parentheses of a pattern without `u`: every `(` outside a class
/// and not escaped that is followed by no `?`, or by `?<` and then neither `=` nor `!`. For a
/// pattern the grammar of B.1.2 accepts this is CountLeftCapturingParensWithin; for any other
/// the parse answers an error whatever the count.
fn count_groups(pattern: &[u16], m: &mut Meter<'_>) -> Result<u32, Stop> {
    let mut count = 0u32;
    let mut in_class = false;
    let mut i = 0usize;
    let at = |i: usize| pattern.get(i).copied();
    while let Some(unit) = at(i) {
        m.read(1)?;
        if unit == u16::from(b'\\') {
            i = i.saturating_add(2);
            continue;
        }
        if in_class {
            in_class = unit != u16::from(b']');
        } else if unit == u16::from(b'[') {
            in_class = true;
        } else if unit == u16::from(b'(') {
            let question = at(i + 1) == Some(u16::from(b'?'));
            let named = question
                && at(i + 2) == Some(u16::from(b'<'))
                && at(i + 3) != Some(u16::from(b'='))
                && at(i + 3) != Some(u16::from(b'!'));
            if !question || named {
                count = count.saturating_add(1);
            }
        }
        i = i.saturating_add(1);
    }
    Ok(count)
}

/// The ASCII byte of `unit`, if it is one.
fn ascii(unit: u16) -> Option<u8> {
    u8::try_from(unit).ok().filter(u8::is_ascii)
}

fn is_digit(unit: Option<u16>) -> bool {
    unit.and_then(ascii).is_some_and(|b| b.is_ascii_digit())
}

fn hex_value(unit: Option<u16>) -> Option<u32> {
    unit.and_then(ascii)
        .and_then(|b| char::from(b).to_digit(16))
}

fn is_lead(unit: u32) -> bool {
    (0xD800..=0xDBFF).contains(&unit)
}

fn is_trail(unit: u32) -> bool {
    (0xDC00..=0xDFFF).contains(&unit)
}

fn pair(lead: u32, trail: u32) -> u32 {
    0x10000 + ((lead - 0xD800) << 10) + (trail - 0xDC00)
}

/// IdentifierStartChar (`#sec-names-and-keywords`): `$`, `_` or a code point with `ID_Start`.
/// ASCII is decided without the table; outside it, one binary search of the table.
fn is_identifier_start(c: u32) -> bool {
    match u8::try_from(c).ok().filter(u8::is_ascii) {
        Some(b) => b.is_ascii_alphabetic() || b == b'$' || b == b'_',
        None => swell_unicode::is_id_start(c),
    }
}

/// IdentifierPartChar (`#sec-names-and-keywords`): `$`, U+200C, U+200D or a code point with
/// `ID_Continue`, which holds `_` and the digits. ASCII is decided without the table.
fn is_identifier_part(c: u32) -> bool {
    match u8::try_from(c).ok().filter(u8::is_ascii) {
        Some(b) => b.is_ascii_alphanumeric() || b == b'$' || b == b'_',
        None => matches!(c, 0x200C | 0x200D) || swell_unicode::is_id_continue(c),
    }
}

/// SyntaxCharacter (`#prod-SyntaxCharacter`).
fn is_syntax_character(b: u8) -> bool {
    matches!(
        b,
        b'^' | b'$'
            | b'\\'
            | b'.'
            | b'*'
            | b'+'
            | b'?'
            | b'('
            | b')'
            | b'['
            | b']'
            | b'{'
            | b'}'
            | b'|'
    )
}

/// A level of the parse: the pattern itself or an open group.
#[derive(Clone, Copy)]
struct Level {
    kind: LevelKind,
    offset: usize,
    terms: usize,
    alternatives: usize,
    id: u32,
}

#[derive(Clone, Copy)]
enum LevelKind {
    Pattern,
    Group { number: u32, name: Option<Name> },
    NonCapturing,
    Look(Look),
    Modified { add: Modifiers, remove: Modifiers },
}

/// A reference the parse checks once it has seen the whole pattern.
#[derive(Clone, Copy)]
enum Reference {
    Number(u32, usize),
    Name(Name, usize),
}

/// `{n}`, `{n,}` or `{n,m}` as written at the parser's position.
struct Braced {
    min: u32,
    max: Option<u32>,
    ordered: bool,
    len: usize,
}

struct Parser<'p, 'm, 'b> {
    src: &'p [u16],
    pos: usize,
    flags: Flags,
    unicode: bool,
    named: bool,
    precount: u32,
    limit: u32,
    m: &'m mut Meter<'b>,
    nodes: Vec<Node>,
    lists: Vec<NodeId>,
    items: Vec<ClassItem>,
    names: Vec<(u32, Name)>,
    text: Vec<u8>,
    groups: u32,
    levels: Vec<Level>,
    terms: Vec<NodeId>,
    alternatives: Vec<NodeId>,
    declared: Names,
    references: Vec<Reference>,
}

fn index32(n: usize) -> Result<u32, ParseError> {
    u32::try_from(n).map_err(|_| ParseError::Stopped(Stop::MemoryBudget))
}

fn error(kind: ErrorKind, offset: usize) -> ParseError {
    ParseError::Pattern(PatternError::new(kind, offset))
}

/// The bytes of the group name `name` in `text`, the names of a parse.
fn name_text(text: &[u8], name: Name) -> &[u8] {
    let start = name.start as usize;
    text.get(start..start.saturating_add(name.len as usize))
        .unwrap_or(&[])
}

impl<'p, 'm, 'b> Parser<'p, 'm, 'b> {
    fn new(
        src: &'p [u16],
        flags: Flags,
        unicode: bool,
        named: bool,
        precount: u32,
        limit: u32,
        m: &'m mut Meter<'b>,
    ) -> Result<Parser<'p, 'm, 'b>, ParseError> {
        let declared = Names::new(m)?;
        Ok(Parser {
            src,
            pos: 0,
            flags,
            unicode,
            named,
            precount,
            limit,
            m,
            nodes: Vec::new(),
            lists: Vec::new(),
            items: Vec::new(),
            names: Vec::new(),
            text: Vec::new(),
            groups: 0,
            levels: Vec::new(),
            terms: Vec::new(),
            alternatives: Vec::new(),
            declared,
            references: Vec::new(),
        })
    }

    // ----- reading -----

    fn at(&self, offset: usize) -> Option<u16> {
        self.src.get(self.pos.saturating_add(offset)).copied()
    }

    fn is(&self, offset: usize, b: u8) -> bool {
        self.at(offset) == Some(u16::from(b))
    }

    fn bump(&mut self, units: usize) -> Result<(), ParseError> {
        self.pos = self.pos.saturating_add(units);
        self.m.read(units)?;
        Ok(())
    }

    /// The code point at the position, a surrogate pair one code point, read past.
    fn read_code_point(&mut self) -> Result<u32, ParseError> {
        let first = u32::from(self.at(0).unwrap_or(0));
        if is_lead(first)
            && let Some(second) = self.at(1).map(u32::from)
            && is_trail(second)
        {
            self.bump(2)?;
            return Ok(pair(first, second));
        }
        self.bump(1)?;
        Ok(first)
    }

    /// The character at the position, read past: a code point under `u`, a code unit without.
    fn read_char(&mut self) -> Result<u32, ParseError> {
        if self.unicode {
            return self.read_code_point();
        }
        let unit = u32::from(self.at(0).unwrap_or(0));
        self.bump(1)?;
        Ok(unit)
    }

    /// The decimal digits at `offset` from the position: their value, at most 2^32 - 1, and
    /// their count. Nothing is read past.
    fn decimal(&mut self, offset: usize) -> Result<(u32, usize), ParseError> {
        let mut value = 0u32;
        let mut len = 0usize;
        while let Some(d) = self
            .at(offset.saturating_add(len))
            .and_then(ascii)
            .filter(u8::is_ascii_digit)
        {
            value = value.saturating_mul(10).saturating_add(u32::from(d - b'0'));
            len += 1;
            self.m.read(1)?;
        }
        Ok((value, len))
    }

    // ----- building -----

    fn node(&mut self, node: Node) -> Result<NodeId, ParseError> {
        self.m.visit()?;
        let id = index32(self.nodes.len())?;
        self.m.push(&mut self.nodes, node)?;
        Ok(NodeId(id))
    }

    fn term(&mut self, id: NodeId) -> Result<(), ParseError> {
        self.m.push(&mut self.terms, id)?;
        Ok(())
    }

    /// The terms from `start` on, moved into a list of the tree.
    fn terms_list(&mut self, start: usize) -> Result<List, ParseError> {
        let moved = self.terms.get(start..).unwrap_or(&[]);
        let len = index32(moved.len())?;
        let at = index32(self.lists.len())?;
        self.m.grow_by(&mut self.lists, moved.len())?;
        self.lists.extend_from_slice(moved);
        self.terms.truncate(start);
        Ok(List { start: at, len })
    }

    /// The alternatives from `start` on, moved into a list of the tree.
    fn alternatives_list(&mut self, start: usize) -> Result<List, ParseError> {
        let moved = self.alternatives.get(start..).unwrap_or(&[]);
        let len = index32(moved.len())?;
        let at = index32(self.lists.len())?;
        self.m.grow_by(&mut self.lists, moved.len())?;
        self.lists.extend_from_slice(moved);
        self.alternatives.truncate(start);
        Ok(List { start: at, len })
    }

    fn top(&self) -> Option<Level> {
        self.levels.last().copied()
    }

    /// The alternative open in `level`, made a node.
    fn alternative(&mut self, level: Level) -> Result<NodeId, ParseError> {
        let list = self.terms_list(level.terms)?;
        self.node(Node::Alternative(list))
    }

    /// The body of `level`: its one alternative, or the Disjunction of its alternatives.
    fn body(&mut self, level: Level) -> Result<NodeId, ParseError> {
        let last = self.alternative(level)?;
        if self.alternatives.len() == level.alternatives {
            return Ok(last);
        }
        self.m.push(&mut self.alternatives, last)?;
        let list = self.alternatives_list(level.alternatives)?;
        self.node(Node::Disjunction(list))
    }

    // ----- the loop -----

    fn run(mut self) -> Result<Pattern, ParseError> {
        let root = Level {
            kind: LevelKind::Pattern,
            offset: 0,
            terms: 0,
            alternatives: 0,
            id: 0,
        };
        self.m.push(&mut self.levels, root)?;
        while let Some(unit) = self.at(0) {
            match ascii(unit) {
                Some(b'|') => {
                    self.bump(1)?;
                    self.bar()?;
                }
                Some(b')') => {
                    if self.levels.len() <= 1 {
                        return Err(error(ErrorKind::UnmatchedParenthesis, self.pos));
                    }
                    self.bump(1)?;
                    let (node, quantifiable) = self.close()?;
                    if quantifiable {
                        self.quantify(node)?;
                    } else {
                        self.term(node)?;
                    }
                }
                Some(b'(') => self.open()?,
                Some(b'^') => self.assertion(Assertion::Start, 1)?,
                Some(b'$') => self.assertion(Assertion::End, 1)?,
                Some(b'\\') if self.is(1, b'b') => self.assertion(Assertion::WordBoundary, 2)?,
                Some(b'\\') if self.is(1, b'B') => {
                    self.assertion(Assertion::NotWordBoundary, 2)?;
                }
                Some(b'*' | b'+' | b'?') => {
                    return Err(error(ErrorKind::NothingToRepeat, self.pos));
                }
                Some(b'{') => {
                    let braced = self.braced()?;
                    if self.unicode {
                        let kind = if braced.is_some() {
                            ErrorKind::NothingToRepeat
                        } else {
                            ErrorKind::LoneQuantifierBracket
                        };
                        return Err(error(kind, self.pos));
                    }
                    if braced.is_some() {
                        return Err(error(ErrorKind::InvalidBracedQuantifier, self.pos));
                    }
                    let atom = self.atom()?;
                    self.quantify(atom)?;
                }
                _ => {
                    let atom = self.atom()?;
                    self.quantify(atom)?;
                }
            }
        }
        if let Some(open) = self.top().filter(|level| level.id != 0) {
            return Err(error(ErrorKind::UnterminatedGroup, open.offset));
        }
        let level = self.levels.pop().unwrap_or(root);
        let root = self.body(level)?;
        self.check_references()?;
        Ok(self.finish(root))
    }

    fn assertion(&mut self, assertion: Assertion, units: usize) -> Result<(), ParseError> {
        self.bump(units)?;
        let node = self.node(Node::Assertion(assertion))?;
        self.term(node)
    }

    /// A `|`: the alternative open in the innermost level ends.
    fn bar(&mut self) -> Result<(), ParseError> {
        let Some(level) = self.top() else {
            return Ok(());
        };
        let node = self.alternative(level)?;
        self.m.push(&mut self.alternatives, node)?;
        self.declared.alternative(level.id);
        Ok(())
    }

    /// A `(`: a new level, unless it would pass the nesting limit.
    fn open(&mut self) -> Result<(), ParseError> {
        let start = self.pos;
        if self.levels.len() > self.limit as usize {
            return Err(error(ErrorKind::Nesting { limit: self.limit }, start));
        }
        let outer = self.top().map_or(0, |level| level.id);
        self.bump(1)?;
        let kind = if self.is(0, b'?') {
            match self.at(1).and_then(ascii) {
                Some(b':') => {
                    self.bump(2)?;
                    LevelKind::NonCapturing
                }
                Some(b'=') => {
                    self.bump(2)?;
                    LevelKind::Look(Look::Ahead)
                }
                Some(b'!') => {
                    self.bump(2)?;
                    LevelKind::Look(Look::NotAhead)
                }
                Some(b'<') if self.is(2, b'=') => {
                    self.bump(3)?;
                    LevelKind::Look(Look::Behind)
                }
                Some(b'<') if self.is(2, b'!') => {
                    self.bump(3)?;
                    LevelKind::Look(Look::NotBehind)
                }
                Some(b'<') => {
                    self.bump(2)?;
                    let name = self.group_name(start)?;
                    let number = self.next_group(start)?;
                    let text = name_text(&self.text, name);
                    if !self.declared.declare(self.m, text, outer)? {
                        return Err(error(ErrorKind::DuplicateGroupName, start));
                    }
                    self.m.push(&mut self.names, (number, name))?;
                    LevelKind::Group {
                        number,
                        name: Some(name),
                    }
                }
                Some(b'i' | b'm' | b's' | b'-') => {
                    self.bump(1)?;
                    self.modifiers(start)?
                }
                _ => return Err(error(ErrorKind::InvalidGroup, start)),
            }
        } else {
            let number = self.next_group(start)?;
            LevelKind::Group { number, name: None }
        };
        let id = self.declared.open(self.m, outer)?;
        let level = Level {
            kind,
            offset: start,
            terms: self.terms.len(),
            alternatives: self.alternatives.len(),
            id,
        };
        self.m.push(&mut self.levels, level)?;
        Ok(())
    }

    /// The number of the next capturing group; 2^32 - 1 groups are an error
    /// (`#sec-patterns-static-semantics-early-errors`, Pattern).
    fn next_group(&mut self, start: usize) -> Result<u32, ParseError> {
        let number = self.groups.saturating_add(1);
        if number == u32::MAX {
            return Err(error(ErrorKind::TooManyGroups, start));
        }
        self.groups = number;
        Ok(number)
    }

    /// The two lists of `(?ims-ims:`, the parser past `(?`.
    fn modifiers(&mut self, start: usize) -> Result<LevelKind, ParseError> {
        let mut repeated = false;
        let mut lists = [0u8; 2];
        let mut dash = false;
        for (index, list) in lists.iter_mut().enumerate() {
            if index == 1 {
                if !self.is(0, b'-') {
                    break;
                }
                dash = true;
                self.bump(1)?;
            }
            loop {
                let bit = match self.at(0).and_then(ascii) {
                    Some(b'i') => MODIFIER_I,
                    Some(b'm') => MODIFIER_M,
                    Some(b's') => MODIFIER_S,
                    _ => break,
                };
                repeated |= *list & bit != 0;
                *list |= bit;
                self.bump(1)?;
            }
        }
        if !self.is(0, b':') {
            return Err(error(ErrorKind::InvalidGroup, start));
        }
        self.bump(1)?;
        let [add, remove] = lists;
        if let Some(kind) = early::modifiers(add, remove, dash, repeated) {
            return Err(error(kind, start));
        }
        Ok(LevelKind::Modified {
            add: Modifiers { bits: add },
            remove: Modifiers { bits: remove },
        })
    }

    /// A `)`: the innermost level closes; its node and whether a quantifier may follow it.
    fn close(&mut self) -> Result<(NodeId, bool), ParseError> {
        let Some(level) = self.top() else {
            return Err(error(ErrorKind::UnmatchedParenthesis, self.pos));
        };
        let body = self.body(level)?;
        self.levels.pop();
        let outer = self.top().map_or(0, |level| level.id);
        self.declared.close(level.id, outer);
        let (node, quantifiable) = match level.kind {
            LevelKind::Pattern => (Node::Alternative(List { start: 0, len: 0 }), true),
            LevelKind::Group { number, name } => (Node::Group { number, name, body }, true),
            LevelKind::NonCapturing => (Node::NonCapturing { body }, true),
            LevelKind::Look(look) => {
                let quantifiable = !self.unicode && matches!(look, Look::Ahead | Look::NotAhead);
                (Node::Look { look, body }, quantifiable)
            }
            LevelKind::Modified { add, remove } => (Node::Modified { add, remove, body }, true),
        };
        Ok((self.node(node)?, quantifiable))
    }

    /// `{n}`, `{n,}` or `{n,m}` at the position, or `None`; nothing is read past.
    fn braced(&mut self) -> Result<Option<Braced>, ParseError> {
        let first = self.pos.saturating_add(1);
        let (min, min_len) = self.decimal(1)?;
        if min_len == 0 {
            return Ok(None);
        }
        let mut len = 1 + min_len;
        if self.is(len, b'}') {
            return Ok(Some(Braced {
                min,
                max: Some(min),
                ordered: true,
                len: len + 1,
            }));
        }
        if !self.is(len, b',') {
            return Ok(None);
        }
        len += 1;
        if self.is(len, b'}') {
            return Ok(Some(Braced {
                min,
                max: None,
                ordered: true,
                len: len + 1,
            }));
        }
        let (max, max_len) = self.decimal(len)?;
        if max_len == 0 || !self.is(len + max_len, b'}') {
            return Ok(None);
        }
        let second = self.pos.saturating_add(len);
        let digits = |from: usize, n: usize| self.src.get(from..from.saturating_add(n));
        let ordered = early::quantifier_in_order(
            digits(first, min_len).unwrap_or(&[]),
            digits(second, max_len).unwrap_or(&[]),
        );
        Ok(Some(Braced {
            min,
            max: Some(max),
            ordered,
            len: len + max_len + 1,
        }))
    }

    /// `atom` and the quantifier after it, if any, as a term.
    fn quantify(&mut self, atom: NodeId) -> Result<(), ParseError> {
        let start = self.pos;
        let (min, max, len) = match self.at(0).and_then(ascii) {
            Some(b'*') => (0, None, 1),
            Some(b'+') => (1, None, 1),
            Some(b'?') => (0, Some(1), 1),
            Some(b'{') => match self.braced()? {
                Some(braced) => {
                    if !braced.ordered {
                        return Err(error(ErrorKind::QuantifierOutOfOrder, start));
                    }
                    (braced.min, braced.max, braced.len)
                }
                None if self.unicode => {
                    return Err(error(ErrorKind::LoneQuantifierBracket, start));
                }
                None => return self.term(atom),
            },
            _ => return self.term(atom),
        };
        self.bump(len)?;
        let greedy = !self.is(0, b'?');
        if !greedy {
            self.bump(1)?;
        }
        let node = self.node(Node::Quantified {
            body: atom,
            quantifier: Quantifier { min, max, greedy },
        })?;
        self.term(node)
    }

    /// An atom that is no group: `.`, a class, an escape or a pattern character.
    fn atom(&mut self) -> Result<NodeId, ParseError> {
        let start = self.pos;
        match self.at(0).and_then(ascii) {
            Some(b'.') => {
                self.bump(1)?;
                self.node(Node::Dot)
            }
            Some(b'[') => self.class(),
            Some(b'\\') => self.atom_escape(),
            Some(b']' | b'}') if self.unicode => {
                Err(error(ErrorKind::LoneQuantifierBracket, start))
            }
            _ => {
                let c = self.read_char()?;
                self.node(Node::Character(c))
            }
        }
    }

    fn class_escape_kind(b: u8) -> Option<ClassEscape> {
        Some(match b {
            b'd' => ClassEscape::Digit,
            b'D' => ClassEscape::NotDigit,
            b's' => ClassEscape::Space,
            b'S' => ClassEscape::NotSpace,
            b'w' => ClassEscape::Word,
            b'W' => ClassEscape::NotWord,
            _ => return None,
        })
    }

    /// `\p{...}` or `\P{...}` under `u`, the parser at the `p` or `P` past the `\` at `start`
    /// (CharacterClassEscape :: `p{` UnicodePropertyValueExpression `}`): the escape, read past.
    /// The expression is `UnicodePropertyName=UnicodePropertyValue` or
    /// LoneUnicodePropertyNameOrValue, each a run of ASCII letters, digits and `_`, looked up as
    /// written (`#sec-runtime-semantics-unicodematchproperty-p`): a name or value that is no name
    /// or alias ECMA-262 admits, a binary property with a value, and any other text between the
    /// braces are the early error, as `\p` or `\P` without `{` is an invalid escape. The lookup is
    /// a binary search of sorted names, linear in the name.
    fn property(&mut self, start: usize) -> Result<ClassEscape, ParseError> {
        let negated = self.is(0, b'P');
        if !self.is(1, b'{') {
            return Err(error(ErrorKind::InvalidEscape, start));
        }
        let name_len = self.property_characters(2)?;
        let name_start = self.pos.saturating_add(2);
        let name_end = name_start.saturating_add(name_len);
        let name = self.src.get(name_start..name_end).unwrap_or(&[]);
        let (property, len) = if self.is(name_len.saturating_add(2), b'=') {
            let value_len = self.property_characters(name_len.saturating_add(3))?;
            let value_start = name_end.saturating_add(1);
            let value = self
                .src
                .get(value_start..value_start.saturating_add(value_len))
                .unwrap_or(&[]);
            (
                swell_unicode::property_value(name, value),
                name_len.saturating_add(value_len).saturating_add(3),
            )
        } else {
            (
                swell_unicode::lone_property(name),
                name_len.saturating_add(2),
            )
        };
        let Some(property) = property.filter(|_| self.is(len, b'}')) else {
            return Err(error(ErrorKind::InvalidPropertyName, start));
        };
        self.bump(len.saturating_add(1))?;
        Ok(ClassEscape::Property { property, negated })
    }

    /// The count of ASCII letters, digits and `_` (UnicodePropertyValueCharacter) from `offset`
    /// past the position; nothing is read past.
    fn property_characters(&mut self, offset: usize) -> Result<usize, ParseError> {
        let mut len = 0usize;
        while self
            .at(offset.saturating_add(len))
            .and_then(ascii)
            .is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
        {
            len += 1;
            self.m.read(1)?;
        }
        Ok(len)
    }

    /// `\` AtomEscape (and, without `u`, the `\` of ExtendedAtom :: `\` [lookahead = c]).
    fn atom_escape(&mut self) -> Result<NodeId, ParseError> {
        let start = self.pos;
        self.bump(1)?;
        let Some(unit) = self.at(0) else {
            return Err(error(ErrorKind::TrailingBackslash, start));
        };
        let b = ascii(unit);
        if let Some(escape) = b.and_then(Self::class_escape_kind) {
            self.bump(1)?;
            return self.node(Node::Escape(escape));
        }
        match b {
            Some(b'p' | b'P') if self.unicode => {
                let escape = self.property(start)?;
                self.node(Node::Escape(escape))
            }
            Some(b'1'..=b'9') => {
                let (n, len) = self.decimal(0)?;
                if self.unicode || n <= self.precount {
                    self.bump(len)?;
                    self.m
                        .push(&mut self.references, Reference::Number(n, start))?;
                    return self.node(Node::Backreference(n));
                }
                let c = self.character_escape(start, false)?;
                self.node(Node::Character(c))
            }
            Some(b'k') if self.named => {
                self.bump(1)?;
                if !self.is(0, b'<') {
                    return Err(error(ErrorKind::InvalidNamedReference, start));
                }
                self.bump(1)?;
                let name = self.group_name(start)?;
                self.m
                    .push(&mut self.references, Reference::Name(name, start))?;
                self.node(Node::NamedBackreference(name))
            }
            _ => {
                let c = self.character_escape(start, false)?;
                self.node(Node::Character(c))
            }
        }
    }

    /// CharacterEscape, the parser past the `\` at `start`: its CharacterValue. Without `u`
    /// a `\c` with no control letter is the `\` alone (ExtendedAtom and ClassAtomNoDash ::
    /// `\` [lookahead = c]), and the parser stays at the `c`.
    fn character_escape(&mut self, start: usize, in_class: bool) -> Result<u32, ParseError> {
        let unit = self.at(0).unwrap_or(0);
        let invalid = || error(ErrorKind::InvalidEscape, start);
        let value = match ascii(unit) {
            Some(b'f') => 0x0C,
            Some(b'n') => 0x0A,
            Some(b'r') => 0x0D,
            Some(b't') => 0x09,
            Some(b'v') => 0x0B,
            Some(b'c') => {
                let next = self.at(1).and_then(ascii);
                let letter = next.filter(u8::is_ascii_alphabetic);
                let control = next.filter(|b| b.is_ascii_digit() || *b == b'_');
                if let Some(b) = letter.or(control.filter(|_| in_class && !self.unicode)) {
                    self.bump(2)?;
                    return Ok(u32::from(b) % 32);
                }
                if self.unicode {
                    return Err(invalid());
                }
                return Ok(u32::from(b'\\'));
            }
            Some(b'0') if !is_digit(self.at(1)) => 0,
            Some(b'0'..=b'9') if self.unicode => return Err(invalid()),
            Some(b'8' | b'9') => u32::from(unit),
            Some(b'0'..=b'7') => return self.legacy_octal(),
            Some(b'x') => match (hex_value(self.at(1)), hex_value(self.at(2))) {
                (Some(high), Some(low)) => {
                    self.bump(3)?;
                    return Ok(high * 16 + low);
                }
                _ if self.unicode => return Err(invalid()),
                _ => u32::from(b'x'),
            },
            Some(b'u') => match self.unicode_escape(self.unicode)? {
                Some((value, len)) => {
                    self.bump(len)?;
                    return Ok(value);
                }
                None if self.unicode => {
                    return Err(error(ErrorKind::InvalidUnicodeEscape, start));
                }
                None => u32::from(b'u'),
            },
            Some(b) if self.unicode => {
                if !is_syntax_character(b) && b != b'/' {
                    return Err(invalid());
                }
                u32::from(b)
            }
            _ if self.unicode => return Err(invalid()),
            _ => u32::from(unit),
        };
        self.bump(1)?;
        Ok(value)
    }

    /// LegacyOctalEscapeSequence (B.1.2, `#prod-annexB-LegacyOctalEscapeSequence`) at the
    /// position: up to three octal digits, the first of three at most 3; its MV.
    fn legacy_octal(&mut self) -> Result<u32, ParseError> {
        let octal = |unit: Option<u16>| {
            unit.and_then(ascii)
                .filter(|b| (b'0'..=b'7').contains(b))
                .map(|b| u32::from(b - b'0'))
        };
        let first = octal(self.at(0)).unwrap_or(0);
        let most = if first <= 3 { 3 } else { 2 };
        let mut value = first;
        let mut len = 1;
        while len < most {
            let Some(d) = octal(self.at(len)) else {
                break;
            };
            value = value * 8 + d;
            len += 1;
        }
        self.bump(len)?;
        Ok(value)
    }

    /// RegExpUnicodeEscapeSequence at the position, its `u`: the value and the code units it
    /// spans from the `u`, or `None` when there is none. With [UnicodeMode] (`unicode_mode`)
    /// `\u{...}` and a pair of two escaped surrogates are one code point.
    fn unicode_escape(&mut self, unicode_mode: bool) -> Result<Option<(u32, usize)>, ParseError> {
        if unicode_mode && self.is(1, b'{') {
            let mut value = 0u32;
            let mut len = 2;
            while let Some(d) = hex_value(self.at(len)) {
                value = value.saturating_mul(16).saturating_add(d);
                len += 1;
                self.m.read(1)?;
            }
            if len > 2 && self.is(len, b'}') && value <= 0x10FFFF {
                return Ok(Some((value, len + 1)));
            }
            return Ok(None);
        }
        let Some(lead) = self.hex4(1) else {
            return Ok(None);
        };
        if unicode_mode
            && is_lead(lead)
            && self.is(5, b'\\')
            && self.is(6, b'u')
            && let Some(trail) = self.hex4(7)
            && is_trail(trail)
        {
            return Ok(Some((pair(lead, trail), 11)));
        }
        Ok(Some((lead, 5)))
    }

    fn hex4(&self, offset: usize) -> Option<u32> {
        (0..4).try_fold(0u32, |value, i| {
            hex_value(self.at(offset + i)).map(|d| value * 16 + d)
        })
    }

    /// RegExpIdentifierName (`#prod-RegExpIdentifierName`) and its `>`, the parser past the
    /// `<`: the name, kept in the tree as the UTF-8 text of its code points, so that two names
    /// compare by their code points. Each code point is read once, written (a surrogate pair one
    /// code point, in both modes) or as `\` RegExpUnicodeEscapeSequence[+UnicodeMode], and
    /// tested once: an IdentifierStartChar first and an IdentifierPartChar after, or the early
    /// error (`#sec-patterns-static-semantics-early-errors`, RegExpIdentifierStart and
    /// RegExpIdentifierPart).
    fn group_name(&mut self, start: usize) -> Result<Name, ParseError> {
        let name_start = self.text.len();
        loop {
            let at = self.pos;
            let Some(unit) = self.at(0) else {
                return Err(error(ErrorKind::InvalidGroupName, start));
            };
            if unit == u16::from(b'>') {
                if self.text.len() == name_start {
                    return Err(error(ErrorKind::InvalidGroupName, start));
                }
                self.bump(1)?;
                break;
            }
            let c = if unit == u16::from(b'\\') {
                if !self.is(1, b'u') {
                    return Err(error(ErrorKind::InvalidGroupName, at));
                }
                self.bump(1)?;
                match self.unicode_escape(true)? {
                    Some((value, len)) => {
                        self.bump(len)?;
                        value
                    }
                    None => return Err(error(ErrorKind::InvalidUnicodeEscape, at)),
                }
            } else {
                self.read_code_point()?
            };
            let valid = if self.text.len() == name_start {
                is_identifier_start(c)
            } else {
                is_identifier_part(c)
            };
            let Some(c) = char::from_u32(c).filter(|_| valid) else {
                return Err(error(ErrorKind::InvalidGroupName, at));
            };
            let mut bytes = [0u8; 4];
            for &b in c.encode_utf8(&mut bytes).as_bytes() {
                self.m.push(&mut self.text, b)?;
            }
        }
        Ok(Name {
            start: index32(name_start)?,
            len: index32(self.text.len().saturating_sub(name_start))?,
        })
    }

    /// CharacterClass, the parser at its `[`.
    fn class(&mut self) -> Result<NodeId, ParseError> {
        let start = self.pos;
        self.bump(1)?;
        let negated = self.is(0, b'^');
        if negated {
            self.bump(1)?;
        }
        let first = index32(self.items.len())?;
        loop {
            let Some(unit) = self.at(0) else {
                return Err(error(ErrorKind::UnterminatedClass, start));
            };
            if unit == u16::from(b']') {
                self.bump(1)?;
                break;
            }
            let at = self.pos;
            let a = self.class_atom()?;
            if self.is(0, b'-') && self.at(1).is_some() && !self.is(1, b']') {
                self.bump(1)?;
                let b = self.class_atom()?;
                let made =
                    early::class_range(a, b, self.unicode).map_err(|kind| error(kind, at))?;
                for item in made.into_iter().flatten() {
                    self.m.push(&mut self.items, item)?;
                }
            } else {
                let item = match a {
                    ClassAtom::Character(c) => ClassItem::Character(c),
                    ClassAtom::Escape(e) => ClassItem::Escape(e),
                };
                self.m.push(&mut self.items, item)?;
            }
        }
        let len = index32(self.items.len())?.saturating_sub(first);
        self.node(Node::Class {
            negated,
            items: List { start: first, len },
        })
    }

    /// ClassAtom.
    fn class_atom(&mut self) -> Result<ClassAtom, ParseError> {
        if self.is(0, b'\\') {
            return self.class_escape();
        }
        Ok(ClassAtom::Character(self.read_char()?))
    }

    /// `\` ClassEscape (and, without `u`, ClassAtomNoDash :: `\` [lookahead = c]).
    fn class_escape(&mut self) -> Result<ClassAtom, ParseError> {
        let start = self.pos;
        self.bump(1)?;
        let Some(unit) = self.at(0) else {
            return Err(error(ErrorKind::TrailingBackslash, start));
        };
        let b = ascii(unit);
        if let Some(escape) = b.and_then(Self::class_escape_kind) {
            self.bump(1)?;
            return Ok(ClassAtom::Escape(escape));
        }
        match b {
            Some(b'b') => {
                self.bump(1)?;
                Ok(ClassAtom::Character(0x08))
            }
            Some(b'-') => {
                self.bump(1)?;
                Ok(ClassAtom::Character(u32::from(b'-')))
            }
            Some(b'p' | b'P') if self.unicode => Ok(ClassAtom::Escape(self.property(start)?)),
            Some(b'B') if self.unicode => Err(error(ErrorKind::InvalidEscape, start)),
            Some(b'k') if self.unicode || self.named => Err(error(ErrorKind::InvalidEscape, start)),
            _ => Ok(ClassAtom::Character(self.character_escape(start, true)?)),
        }
    }

    // ----- the end -----

    /// The early errors that need the whole pattern: a backreference past the count of
    /// capturing groups, and `\k<name>` where no group has the name.
    fn check_references(&mut self) -> Result<(), ParseError> {
        for index in 0..self.references.len() {
            self.m.visit()?;
            match self.references.get(index).copied() {
                Some(Reference::Number(n, at)) if n > self.groups => {
                    return Err(error(ErrorKind::BackreferencePastGroups, at));
                }
                Some(Reference::Name(name, at))
                    if !self.declared.has(name_text(&self.text, name)) =>
                {
                    return Err(error(ErrorKind::UnknownGroupName, at));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The tree, the parser's own stacks dropped and credited.
    fn finish(self, root: NodeId) -> Pattern {
        let Parser {
            m,
            nodes,
            lists,
            items,
            names,
            text,
            groups,
            levels,
            terms,
            alternatives,
            declared,
            references,
            flags,
            ..
        } = self;
        m.release(levels);
        m.release(terms);
        m.release(alternatives);
        m.release(references);
        declared.release(m);
        Pattern {
            nodes,
            lists,
            items,
            names,
            text,
            root,
            groups,
            flags,
            charged: 0,
        }
    }
}
