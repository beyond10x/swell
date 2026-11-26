//! The instruction that makes an arrow function (`#sec-arrow-function-definitions`), called from
//! one arm of the run loop (`interp/dispatch.rs`), so that the loop's frame holds none of its
//! locals. Written by `story:arrow-functions`.
//!
//! An arrow function's [[ThisMode]] is lexical (`docs/design/engine.md`, decision 3, "`this` and
//! `arguments` in an arrow function"): the function object keeps the `this`, the NewTarget and
//! the function of the frame that makes it in its box (`ArrowContext`, item 4), a call puts
//! them in its own frame and binds no `this`, and the collector traces them with the function
//! (`object/ecmascript_function.rs`). Its `arguments` is a name of the enclosing code, resolved
//! by the compiler as any other name. It has no [[Construct]] and no `prototype` property.

use alloc::rc::Rc;

use super::dispatch::Flow;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::object::ecmascript_function::ArrowContext;
use crate::value::Value;

impl<H: Host> Agent<H> {
    /// `ArrowClosure`: InstantiateArrowFunctionExpression
    /// (`#sec-runtime-semantics-instantiatearrowfunctionexpression`) of template `index` in the
    /// LexicalEnvironment, keeping the running frame's `this`, NewTarget and function, which the
    /// frame roots until the function holds them.
    pub(super) fn arrow_closure(&mut self, index: u32) -> Completion<Flow> {
        let (compiled, script, realm, env, context) = {
            let frame = self.top_frame()?;
            (
                Rc::clone(&frame.compiled),
                frame.script,
                frame.realm,
                frame.env,
                ArrowContext {
                    this: frame.this,
                    new_target: frame.new_target,
                    function: frame.function,
                },
            )
        };
        let function = self.instantiate_function(&compiled, script, realm, index, env)?;
        self.keep_arrow_context(function, context)?;
        self.push_value(Value::Object(function))?;
        Ok(Flow::Next)
    }
}
