//! The work of the instruction of template literals without a tag (`#sec-template-literals`),
//! called from one arm of the run loop (`interp/dispatch.rs`), so that the loop's frame holds
//! none of its locals. Written by `story:template-literals`.
//!
//! A template evaluates `ToString` of each substitution in source order
//! (`#sec-template-literals-runtime-semantics-evaluation`) and concatenates the strings with
//! `Add`, the concatenation of `+` (`ops/operators.rs`): a step per code unit, and a result past
//! `string_length` throws the `RangeError` that `+` throws.

use super::dispatch::Flow;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// `ToString`: ToString (`#sec-tostring`) of the value on top of the operand stack, which
    /// stays there, rooted, while an object's `toString` or `valueOf` runs; then the String
    /// replaces it.
    pub(super) fn substitution_to_string(&mut self) -> Completion<Flow> {
        let value = self.peek_value(0)?;
        let string = self.to_string(value)?;
        self.drop_entries(1)?;
        self.push_value(Value::String(string))?;
        Ok(Flow::Next)
    }
}
