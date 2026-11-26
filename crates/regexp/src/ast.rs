//! The tree ParsePattern answers (`#sec-parsepattern`): the nodes of a Pattern in one vector,
//! each naming its children by index, so that dropping a tree of any depth does not recurse
//! (`docs/design/engine.md`, decision 7, "No Rust recursion in the parser and the compiler of a
//! pattern"). The lists of children, the items of the classes, the operations and strings of
//! the classes under `v` and the group names live in vectors of their own beside the nodes.
//! Every vector was charged through the [`Budget`] before it grew; [`Pattern::charged`] is what
//! the budget holds for the tree, and [`Pattern::release`] drops the tree and credits it.
//!
//! A character is a code point under `u` (the pattern is read as code points, a surrogate pair
//! one character) and a code unit without it (`#sec-regexpinitialize`: "each of pattern's 16-bit
//! elements as a Unicode BMP code point").

use crate::budget::{Budget, capacity_bytes};
use crate::flags::Flags;
use alloc::vec::Vec;

/// The index of a node of a [`Pattern`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub(crate) u32);

impl NodeId {
    /// The index of the node in [`Pattern::nodes`].
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// A run of children or of class items: where it starts and how many it holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct List {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

impl List {
    /// How many it holds.
    pub fn len(self) -> usize {
        self.len as usize
    }

    /// Whether it holds none.
    pub fn is_empty(self) -> bool {
        self.len == 0
    }

    fn range(self) -> core::ops::Range<usize> {
        let start = self.start as usize;
        start..start.saturating_add(self.len as usize)
    }
}

/// A group name: the UTF-8 text of its code points kept in the tree
/// (`#sec-static-semantics-capturinggroupname`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Name {
    pub(crate) start: u32,
    pub(crate) len: u32,
}

/// `^`, `$`, `\b`, `\B`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assertion {
    Start,
    End,
    WordBoundary,
    NotWordBoundary,
}

/// `(?=`, `(?!`, `(?<=`, `(?<!`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    Ahead,
    NotAhead,
    Behind,
    NotBehind,
}

/// `\d \D \s \S \w \W` and, under `u`, `\p{...}` and `\P{...}` (CharacterClassEscape).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassEscape {
    Digit,
    NotDigit,
    Space,
    NotSpace,
    Word,
    NotWord,
    /// `\p{...}` (`negated` false) or `\P{...}`: the code points of `property`, or the rest
    /// (`#sec-compiletocharset`, CharacterClassEscape :: `p{` and `P{`).
    Property {
        property: swell_unicode::Property,
        negated: bool,
    },
}

/// A quantifier: `*` is `{0,}`, `+` `{1,}`, `?` `{0,1}`; `max` is `None` for no bound. A count
/// written past 2^32 - 1 is kept as 2^32 - 1; the early error of a minimum past its maximum
/// compares the digits as written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Quantifier {
    pub min: u32,
    pub max: Option<u32>,
    pub greedy: bool,
}

/// The modifiers `i`, `m` and `s` of one list of a modifier group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub(crate) bits: u8,
}

pub(crate) const MODIFIER_I: u8 = 1;
pub(crate) const MODIFIER_M: u8 = 2;
pub(crate) const MODIFIER_S: u8 = 4;

impl Modifiers {
    pub fn ignore_case(self) -> bool {
        self.bits & MODIFIER_I != 0
    }

    pub fn multiline(self) -> bool {
        self.bits & MODIFIER_M != 0
    }

    pub fn dot_all(self) -> bool {
        self.bits & MODIFIER_S != 0
    }

    pub fn is_empty(self) -> bool {
        self.bits == 0
    }
}

/// One item of a character class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassItem {
    /// One character.
    Character(u32),
    /// The characters from the first to the second, both included; the first is not greater.
    Range(u32, u32),
    /// A class escape. Without `u` a range with a class escape at an end is the union of its
    /// ends and `-` (B.1.2, CharacterRangeOrUnion), kept as those three items.
    Escape(ClassEscape),
}

/// One operation of a class under `v` (ClassSetExpression, `#sec-compiletocharset`), in
/// postfix order: an operand pushes a set, a union, an intersection or a subtraction takes two
/// and pushes one, a complement takes one and pushes one. The class is kept as the pattern
/// writes it and evaluated when it is run (`class_set.rs`); no set is built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClassSetOp {
    /// The characters and ranges of one operand in [`Pattern::class_items`] (only
    /// [`ClassItem::Character`] and [`ClassItem::Range`]): the ClassSetCharacters and
    /// ClassSetRanges of a ClassUnion taken together, or one ClassSetCharacter of an
    /// intersection or a subtraction; with `negated` their complement.
    Items {
        items: List,
        negated: bool,
    },
    /// `\d`, `\s`, `\w` or `\p{...}` (its `negated` false), with `negated` its complement
    /// (`\D`, `\S`, `\W`, `\P{...}`, or a class `[^...]` of the escape alone).
    Escape {
        escape: ClassEscape,
        negated: bool,
    },
    /// `\p{...}` of a property of strings (`#table-binary-unicode-properties-of-strings`).
    Strings(swell_unicode::StringProperty),
    /// One string of `\q{...}` of no code point or of two or more, its code points in
    /// [`Pattern::class_string`]; a string of one code point is a character of
    /// [`ClassSetOp::Items`].
    String(List),
    Union,
    Intersection,
    Subtraction,
    /// The complement of a set of single characters (`[^...]`; CharacterComplement).
    Complement,
}

impl ClassSetOp {
    /// Whether it is an operand, which pushes a set.
    pub fn is_operand(self) -> bool {
        matches!(
            self,
            ClassSetOp::Items { .. }
                | ClassSetOp::Escape { .. }
                | ClassSetOp::Strings(_)
                | ClassSetOp::String(_)
        )
    }
}

/// One node of a pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Node {
    /// Two or more alternatives, each an [`Node::Alternative`].
    Disjunction(List),
    /// The terms of one alternative, in order; none for the empty alternative.
    Alternative(List),
    Assertion(Assertion),
    /// A lookaround and its body.
    Look {
        look: Look,
        body: NodeId,
    },
    /// An atom and its quantifier; without `u` the atom may be a lookahead (B.1.2,
    /// QuantifiableAssertion).
    Quantified {
        body: NodeId,
        quantifier: Quantifier,
    },
    Character(u32),
    /// `.`
    Dot,
    /// A class escape outside a class.
    Escape(ClassEscape),
    /// `[...]` or `[^...]`.
    Class {
        negated: bool,
        items: List,
    },
    /// Under `v`, a class (`[...]`, nested classes, `&&`, `--`, `\q{...}`) or a class escape
    /// outside one: its operations in [`Pattern::class_set_ops`], in postfix order.
    ClassSet {
        ops: List,
    },
    /// A capturing group, numbered from 1 by its `(` from the left, with its name if it has one.
    Group {
        number: u32,
        name: Option<Name>,
        body: NodeId,
    },
    /// `(?:...)`.
    NonCapturing {
        body: NodeId,
    },
    /// `(?ims-ims:...)`.
    Modified {
        add: Modifiers,
        remove: Modifiers,
        body: NodeId,
    },
    /// `\n` for a group number n.
    Backreference(u32),
    /// `\k<name>`.
    NamedBackreference(Name),
}

/// The tree of a pattern.
#[derive(Debug)]
pub struct Pattern {
    pub(crate) nodes: Vec<Node>,
    pub(crate) lists: Vec<NodeId>,
    pub(crate) items: Vec<ClassItem>,
    pub(crate) set_ops: Vec<ClassSetOp>,
    pub(crate) points: Vec<u32>,
    pub(crate) names: Vec<(u32, Name)>,
    pub(crate) text: Vec<u8>,
    pub(crate) root: NodeId,
    pub(crate) groups: u32,
    pub(crate) flags: Flags,
    pub(crate) charged: usize,
}

impl Pattern {
    /// The node of the whole pattern: its Disjunction, or its one Alternative.
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// The node `id`.
    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.index())
    }

    /// Every node; a child comes before its parent.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// The children a [`Node::Disjunction`] or a [`Node::Alternative`] names.
    pub fn children(&self, list: List) -> &[NodeId] {
        self.lists.get(list.range()).unwrap_or(&[])
    }

    /// The items a [`Node::Class`] names.
    pub fn class_items(&self, list: List) -> &[ClassItem] {
        self.items.get(list.range()).unwrap_or(&[])
    }

    /// The operations a [`Node::ClassSet`] names.
    pub fn class_set_ops(&self, list: List) -> &[ClassSetOp] {
        self.set_ops.get(list.range()).unwrap_or(&[])
    }

    /// The code points of a string a [`ClassSetOp::String`] names.
    pub fn class_string(&self, list: List) -> &[u32] {
        self.points.get(list.range()).unwrap_or(&[])
    }

    /// The text of a group name.
    pub fn name(&self, name: Name) -> &str {
        let start = name.start as usize;
        self.text
            .get(start..start.saturating_add(name.len as usize))
            .and_then(|bytes| core::str::from_utf8(bytes).ok())
            .unwrap_or("")
    }

    /// The count of capturing groups (CountLeftCapturingParensWithin of the Pattern).
    pub fn group_count(&self) -> u32 {
        self.groups
    }

    /// Each named group, by its number from the left, with its name; a name declared in
    /// different alternatives appears once per group.
    pub fn group_names(&self) -> impl Iterator<Item = (u32, &str)> {
        self.names
            .iter()
            .map(|&(number, name)| (number, self.name(name)))
    }

    /// The flags the pattern was parsed with.
    pub fn flags(&self) -> Flags {
        self.flags
    }

    /// The bytes the budget holds for this tree.
    pub fn charged(&self) -> usize {
        self.charged
    }

    /// The bytes of the tree's vectors by their capacities, summed from the vectors themselves.
    pub fn capacity_bytes(&self) -> usize {
        capacity_bytes(&self.nodes)
            .saturating_add(capacity_bytes(&self.lists))
            .saturating_add(capacity_bytes(&self.items))
            .saturating_add(capacity_bytes(&self.set_ops))
            .saturating_add(capacity_bytes(&self.points))
            .saturating_add(capacity_bytes(&self.names))
            .saturating_add(capacity_bytes(&self.text))
    }

    /// Drop the tree and credit `budget` with what it holds for it.
    pub fn release(self, budget: &mut dyn Budget) {
        let charged = self.charged;
        drop(self);
        if charged > 0 {
            budget.credit(charged);
        }
    }
}
