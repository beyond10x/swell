//! RegExp objects (ECMA-262 22.2.7, `#sec-properties-of-regexp-instances`): the kind of an object
//! with a [[RegExpMatcher]] (`docs/design/engine.md`, decision 7, "Where a RegExp object keeps its
//! program"), written by `story:regexp-objects-exec-and-literals`.
//!
//! The record holds [[OriginalSource]] and [[OriginalFlags]] as two String handles and the
//! program of `swell-regexp` as a box of its own: the instructions, the flags of [[RegExpRecord]],
//! the count of capturing groups and their names. The compile step charged the program to the
//! memory budget as it grew; when the object is stored that charge, with the box, becomes the
//! bytes the record owns ([`regexp_owned_bytes`]), which the sweep that frees the record credits.
//! Two objects never share a program.
//!
//! Not kept: [[LegacyFeaturesEnabled]] and the [[Realm]] of RegExpAlloc (`#sec-regexpalloc`),
//! which only `RegExp.prototype.compile` (slice 18) and the legacy static properties
//! (`legacy-regexp`, excluded by `conformance/test262/features.txt`) read.
//!
//! The kind takes every internal method by its ordinary arm (`object/mod.rs`). Its kind site of
//! the collector traces the two Strings; the program holds no handle.

use swell_regexp::Program;

use crate::heap::collect::Mark;
use crate::value::{StringRef, Value};

/// The slots of a RegExp object.
pub(crate) struct RegExpObject {
    /// [[OriginalSource]].
    pub(crate) source: StringRef,
    /// [[OriginalFlags]].
    pub(crate) flags: StringRef,
    /// [[RegExpMatcher]] and [[RegExpRecord]]: the program, with its flags, its count of
    /// capturing groups and their names.
    pub(crate) program: Box<Program>,
}

/// The bytes a RegExp record owns: what the compile step charged for `program`, and its box.
pub(crate) fn regexp_owned_bytes(program: &Program) -> u64 {
    let charged = u64::try_from(program.heap_bytes()).unwrap_or(u64::MAX);
    charged.saturating_add(program_box_bytes())
}

/// The bytes of the box of a program.
pub(crate) fn program_box_bytes() -> u64 {
    u64::try_from(size_of::<Program>()).unwrap_or(u64::MAX)
}

impl RegExpObject {
    /// Kind site of the collector: [[OriginalSource]] and [[OriginalFlags]].
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/regexp.rs RegExpObject::trace");
        mark.value(Value::String(self.source));
        mark.value(Value::String(self.flags));
    }

    /// A RegExp object of the empty pattern whose two Strings are `string`, for the tests of
    /// the collector.
    #[cfg(test)]
    pub(crate) fn for_test(string: StringRef) -> RegExpObject {
        use swell_regexp::{Flags, Unlimited, compile_pattern, parse_pattern};
        let pattern = parse_pattern(&[], Flags::default(), 10, &mut Unlimited).unwrap();
        let program = compile_pattern(&pattern, &mut Unlimited).unwrap();
        RegExpObject {
            source: string,
            flags: string,
            program: Box::new(program),
        }
    }
}
