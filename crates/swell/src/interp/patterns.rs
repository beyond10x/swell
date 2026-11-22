//! The instructions of an object pattern (`swell_compiler::bytecode`, "object patterns"), each in
//! a function of its own that the arm of `interp/dispatch.rs` calls, so the frame of the
//! instruction loop stays as it was. Written by `story:object-destructuring`.
//!
//! An ObjectBindingPattern (`#sec-runtime-semantics-bindinginitialization`) and an
//! ObjectAssignmentPattern (`#sec-runtime-semantics-destructuringassignmentevaluation`) first
//! require the value to be coercible to an object (RequireObjectCoercible,
//! `#sec-requireobjectcoercible`), then read each property with GetV (`#sec-getv`): [[Get]] of
//! ToObject of the value, with the value itself as the receiver, in source order.

use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// `RequireObjectCoercible`: a `TypeError` when the value on top is `undefined` or `null`;
    /// the value stays.
    #[inline(never)]
    pub(crate) fn require_object_coercible_pattern(&mut self) -> Completion<()> {
        let which = match self.peek_value(0)? {
            Value::Undefined => "undefined",
            Value::Null => "null",
            _ => return Ok(()),
        };
        Err(self.throw_error(ErrorKind::TypeError, &format!("Cannot destructure {which}")))
    }

    /// `PatternGet { depth, name }`: GetV of the value `depth` entries under the top with the
    /// key of String constant `name`, pushed.
    #[inline(never)]
    pub(crate) fn pattern_get(&mut self, depth: u32, name: u32) -> Completion<()> {
        let base = self.peek_value(usize::try_from(depth).unwrap_or(usize::MAX))?;
        let key = self.constant_key(name)?;
        let value = self.get_value(base, key)?;
        self.push_value(value)
    }
}
