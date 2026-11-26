//! The errors of ParsePattern (`#sec-parsepattern`) and of the flags (`#sec-regexpinitialize`):
//! one error with the offset of the code unit it names, or a stop of the budget.
//!
//! Two kinds of message, as `swell_syntax::message_kind` tells them apart from the message
//! alone:
//!
//! - a grammar or early error of ECMA-262 (`#sec-patterns-static-semantics-early-errors`,
//!   `#sec-patterns-static-semantics-early-errors-annexb`), worded here;
//! - the nesting limit (decision 7, "No Rust recursion in the parser and the compiler of a
//!   pattern"): `nesting deeper than parser_nesting (<limit>)`. It is not a refusal.
//!
//! ParsePattern refuses no construct since
//! `story:regexp-v-flag-class-sets-and-properties-of-strings` brought the flag `v`, the last
//! (`docs/design/engine.md`, decision 7, "Refusals until the later stories of slice 5").

use alloc::string::String;
use alloc::string::ToString;
use core::fmt;

use crate::budget::Stop;

/// A construct ParsePattern refuses until the story that brings it: none since
/// `story:regexp-v-flag-class-sets-and-properties-of-strings` brought the flag `v`, so no value
/// of this type exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {}

impl Refusal {
    /// The construct the message names.
    pub fn construct(self) -> &'static str {
        match self {}
    }
}

/// What is wrong with a pattern or its flags.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// A flag that is not one of `d g i m s u v y`.
    UnknownFlag,
    /// A flag written twice.
    RepeatedFlag,
    /// `u` and `v` together (`#sec-parsepattern`, step 1).
    UnicodeWithSets,
    /// A quantifier with no atom before it, or after an assertion it cannot follow.
    NothingToRepeat,
    /// `{n}`, `{n,}` or `{n,m}` where an atom stands (`InvalidBracedQuantifier`, B.1.2.1).
    InvalidBracedQuantifier,
    /// `{` that starts no quantifier, or `}` or `]` alone, under `u`.
    LoneQuantifierBracket,
    /// `{n,m}` with n greater than m.
    QuantifierOutOfOrder,
    /// A `(` with no `)`.
    UnterminatedGroup,
    /// A `)` with no `(`.
    UnmatchedParenthesis,
    /// `(?` followed by none of the forms of a group.
    InvalidGroup,
    /// A group name that is not a RegExpIdentifierName between `<` and `>`.
    InvalidGroupName,
    /// A `[` with no `]`.
    UnterminatedClass,
    /// A `\` at the end of the pattern.
    TrailingBackslash,
    /// An escape the grammar of the mode does not have.
    InvalidEscape,
    /// `\u` that is no RegExpUnicodeEscapeSequence where one is needed, or a code point past
    /// U+10FFFF.
    InvalidUnicodeEscape,
    /// `\k` not followed by a group name where the pattern has group names, or under `u`.
    InvalidNamedReference,
    /// A class range whose first end is greater than its second.
    RangeOutOfOrder,
    /// A class escape at an end of a class range under `u`.
    ClassEscapeInRange,
    /// `\p{...}` or `\P{...}` whose text is no UnicodePropertyValueExpression, or names a
    /// property or a value ECMA-262 does not admit (`#sec-patterns-static-semantics-early-errors`,
    /// UnicodePropertyValueExpression).
    InvalidPropertyName,
    /// A backreference past the count of capturing groups.
    BackreferencePastGroups,
    /// `\k<name>` where no group has that name.
    UnknownGroupName,
    /// A group name declared twice where both groups might participate.
    DuplicateGroupName,
    /// A modifier written twice in one list of a modifier group.
    RepeatedModifier,
    /// A modifier both added and removed.
    ModifierAddedAndRemoved,
    /// `(?-:`: both lists of a modifier group empty.
    EmptyModifiers,
    /// 2^32 - 1 capturing groups or more.
    TooManyGroups,
    /// A group, a lookaround, a modifier group or under `v` a class that would pass the nesting
    /// limit the caller gave.
    Nesting { limit: u32 },
    /// Under `v`, a ClassSetReservedDoublePunctuator (`&&` where no operation may stand, `!!`,
    /// `##` and the others) in a class, or `&` after `&&`.
    ReservedDoublePunctuator,
    /// Under `v`, a ClassUnion, a ClassIntersection and a ClassSubtraction mixed in one class,
    /// or a range in an intersection or a subtraction.
    MixedClassSetOperations,
    /// Under `v`, a ClassSetSyntaxCharacter (`( ) [ ] { } / - \ |`) in a class where it does
    /// not start a nested class, a range or an operation.
    ClassSetSyntaxCharacter,
    /// Under `v`, `&&` or `--` with no operand after it.
    MissingClassSetOperand,
    /// Under `v`, `[^...]` or `\P{...}` of a set whose MayContainStrings is true
    /// (`#sec-patterns-static-semantics-early-errors`, NestedClass, CharacterClassEscape).
    NegatedStrings,
    /// A construct refused until a later story of slice 5; none is left ([`Refusal`]).
    Refused(Refusal),
}

/// One error of a pattern or its flags: what is wrong and the offset, in code units, of the code
/// unit it names in the pattern (in the flags for the errors of the flags).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PatternError {
    pub kind: ErrorKind,
    pub offset: usize,
}

impl PatternError {
    pub(crate) fn new(kind: ErrorKind, offset: usize) -> PatternError {
        PatternError { kind, offset }
    }

    /// Whether this is a refusal of a construct a later story of slice 5 brings: never, since
    /// [`Refusal`] has no value.
    pub fn is_refusal(&self) -> bool {
        matches!(self.kind, ErrorKind::Refused(_))
    }

    /// The message of the `SyntaxError`, without the offset.
    pub fn message(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for PatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self.kind {
            ErrorKind::Nesting { limit } => {
                return write!(f, "nesting deeper than parser_nesting ({limit})");
            }
            ErrorKind::Refused(refusal) => match refusal {},
            ErrorKind::ReservedDoublePunctuator => "a reserved double punctuator in a class",
            ErrorKind::MixedClassSetOperations => "class set operations mixed in one class",
            ErrorKind::ClassSetSyntaxCharacter => {
                "a class set syntax character that is not escaped"
            }
            ErrorKind::MissingClassSetOperand => "a class set operation without its operand",
            ErrorKind::NegatedStrings => "a negated class or \\P of a set that may contain strings",
            ErrorKind::UnknownFlag => "invalid regular expression flag",
            ErrorKind::RepeatedFlag => "repeated regular expression flag",
            ErrorKind::UnicodeWithSets => "the flags u and v together",
            ErrorKind::NothingToRepeat => "nothing to repeat",
            ErrorKind::InvalidBracedQuantifier => "a braced quantifier with nothing to repeat",
            ErrorKind::LoneQuantifierBracket => "lone quantifier bracket",
            ErrorKind::QuantifierOutOfOrder => "numbers out of order in {} quantifier",
            ErrorKind::UnterminatedGroup => "unterminated group",
            ErrorKind::UnmatchedParenthesis => "unmatched ')'",
            ErrorKind::InvalidGroup => "invalid group",
            ErrorKind::InvalidGroupName => "invalid capture group name",
            ErrorKind::UnterminatedClass => "unterminated character class",
            ErrorKind::TrailingBackslash => "\\ at end of pattern",
            ErrorKind::InvalidEscape => "invalid escape",
            ErrorKind::InvalidUnicodeEscape => "invalid Unicode escape",
            ErrorKind::InvalidNamedReference => "invalid named reference",
            ErrorKind::RangeOutOfOrder => "range out of order in character class",
            ErrorKind::ClassEscapeInRange => "a class escape at an end of a class range",
            ErrorKind::InvalidPropertyName => "invalid property name",
            ErrorKind::BackreferencePastGroups => "a backreference past the capturing groups",
            ErrorKind::UnknownGroupName => "a reference to a group name no group has",
            ErrorKind::DuplicateGroupName => "duplicate capture group name",
            ErrorKind::RepeatedModifier => "repeated modifier",
            ErrorKind::ModifierAddedAndRemoved => "a modifier both added and removed",
            ErrorKind::EmptyModifiers => "a modifier group with no modifier",
            ErrorKind::TooManyGroups => "too many capturing groups",
        };
        f.write_str(text)
    }
}

/// Why ParsePattern answered no tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The pattern or its flags have an error, a refusal or the nesting limit.
    Pattern(PatternError),
    /// The budget stopped the parse; everything it charged is credited.
    Stopped(Stop),
}

impl From<Stop> for ParseError {
    fn from(stop: Stop) -> ParseError {
        ParseError::Stopped(stop)
    }
}

impl From<PatternError> for ParseError {
    fn from(error: PatternError) -> ParseError {
        ParseError::Pattern(error)
    }
}
