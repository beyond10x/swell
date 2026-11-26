//! The instruction that makes an arrow function (`#sec-arrow-function-definitions`), called from
//! one arm of the run loop (`interp/dispatch.rs`), so that the loop's frame holds none of its
//! locals. Written by `story:arrow-functions`.
//!
//! An arrow function's [[ThisMode]] is lexical (`docs/design/engine.md`, decision 3, "`this` and
//! `arguments` in an arrow function"): the function object keeps the `this` of the frame that
//! makes it, a call puts that value in its own frame and binds none, and the collector traces it
//! with the function (`object/ecmascript_function.rs`). Its `arguments` is a name of the
//! enclosing code, resolved by the compiler as any other name. It has no [[Construct]] and no
//! `prototype` property.

use alloc::rc::Rc;

use super::dispatch::Flow;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// `ArrowClosure`: InstantiateArrowFunctionExpression
    /// (`#sec-runtime-semantics-instantiatearrowfunctionexpression`) of template `index` in the
    /// LexicalEnvironment, keeping the running frame's `this`, which the frame roots until the
    /// function holds it.
    pub(super) fn arrow_closure(&mut self, index: u32) -> Completion<Flow> {
        let (compiled, script, realm, env, this) = {
            let frame = self.top_frame()?;
            (
                Rc::clone(&frame.compiled),
                frame.script,
                frame.realm,
                frame.env,
                frame.this,
            )
        };
        let function = self.instantiate_function(&compiled, script, realm, index, env)?;
        self.keep_lexical_this(function, this)?;
        self.push_value(Value::Object(function))?;
        Ok(Flow::Next)
    }
}
