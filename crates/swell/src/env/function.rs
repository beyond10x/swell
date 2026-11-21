//! Function Environment Records (ECMA-262 9.1.1.3, `#sec-function-environment-records`). Filled
//! by `story:interpreter-and-script-entry-points`.
//!
//! The compiler gives a function an environment only when a nested function captures one of its
//! bindings (`FunctionTemplate::captured` of `swell-compiler`); its other bindings are local slots
//! of the frame. The record a call makes is then a declarative record of `captured.len()` slots
//! whose [[OuterEnv]] is the function's [[Environment]] (NewFunctionEnvironment, steps 2 and 6).
//!
//! The fields a Function Environment Record adds, [[ThisValue]], [[ThisBindingStatus]],
//! [[FunctionObject]] and [[NewTarget]], are held by the function's frame (`interp/frame.rs`):
//! in slice 1 only the function's own code reads `this`, since arrow functions, `super` and
//! `new.target`, which read them from an outer function's environment, come with slices 6 and 8.
//! Those slices move them here when a nested function must reach them.

use crate::heap::arena::Handle;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;

impl<H: Host> Agent<H> {
    /// NewFunctionEnvironment (`#sec-newfunctionenvironment`) of a function whose
    /// [[Environment]] is `outer`, with `slots` captured bindings, each `undefined` until
    /// FunctionDeclarationInstantiation stores the parameters and the code the functions.
    pub(crate) fn new_function_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
    ) -> Completion<Handle> {
        self.new_environment(outer, slots)
    }
}
