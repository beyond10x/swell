//! The work of the instruction of a regular expression literal
//! (`#sec-regular-expression-literals-runtime-semantics-evaluation`), called from one arm of the
//! run loop (`interp/dispatch.rs`), so that the loop's frame holds none of its locals. Written by
//! `story:regexp-objects-exec-and-literals`.
//!
//! A literal is compiled at each evaluation (`docs/design/engine.md`, decision 7, "A regular
//! expression literal: checked at parse time, compiled at each evaluation", rule 2): RegExpCreate
//! of its body and flags in the current realm, the realm of the running code, which parses and
//! compiles the pattern again (`builtins/regexp/mod.rs`). It pays the instruction's step and a
//! step per code unit of the body before the parse, and makes a new object with a program of its
//! own at each evaluation. The parse meets no error of the pattern, which the check at parse
//! found; the compile step may refuse the flag or a modifier `i`, a counted refusal.

use super::dispatch::Flow;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// `NewRegExp { body, flags }`: RegExpCreate (`#sec-regexpcreate`) of the String constants
    /// `body` and `flags`, pushed.
    pub(super) fn new_regexp(&mut self, body: u32, flags: u32) -> Completion<Flow> {
        let body = self.constant_string(body)?;
        let flags = self.constant_string(flags)?;
        let regexp = self.regexp_create(Value::String(body), Value::String(flags))?;
        self.push_value(Value::Object(regexp))?;
        Ok(Flow::Next)
    }
}
