//! Function Environment Records (ECMA-262 9.1.1.3, `#sec-function-environment-records`). Filled
//! by `story:interpreter-and-script-entry-points`.
//!
//! The compiler gives a function an environment only when a nested function captures one of its
//! bindings, a lookup by name from a `with` body or from eval code may reach them, or a mapped
//! arguments object reads and writes its parameters (`FunctionTemplate::captured` of
//! `swell-compiler`); its other bindings are local slots of the frame. The record a call makes is
//! then a declarative record of `captured.len()` slots whose [[OuterEnv]] is the function's
//! [[Environment]] (NewFunctionEnvironment, steps 2 and 6). It is the call's
//! VariableEnvironment too (`interp/frame.rs`), where a direct `eval` of non-strict code adds its
//! bindings: a function whose code contains one always makes it, its arguments object at least
//! living there (decision 3, "Names", item 4).
//!
//! The fields a Function Environment Record adds, [[ThisValue]], [[FunctionObject]] and
//! [[NewTarget]], stay on the function's frame (`interp/frame.rs`: `this`, `function` and
//! `new_target`) and in an arrow function's box (`ArrowContext`, `object/ecmascript_function.rs`),
//! which takes the three from the frame that makes the arrow, so the arrow's frame and the
//! frame of direct eval code in it answer GetThisEnvironment's values without a walk of the
//! environments (`docs/design/engine.md`, decision 3, "`this` and `arguments` in an arrow
//! function", items 4 and 5). A derived constructor's [[ThisBindingStatus]] is a hidden binding
//! (item 6 of "Classes").

use crate::env::SlotNames;
use crate::heap::arena::Handle;
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;

impl<H: Host> Agent<H> {
    /// NewFunctionEnvironment (`#sec-newfunctionenvironment`) of a function whose
    /// [[Environment]] is `outer`, with `slots` captured bindings, each `undefined` until
    /// FunctionDeclarationInstantiation stores the parameters and the code the functions. The
    /// slots are named by the template's `captured` (`names`), which a lookup by name reads.
    pub(crate) fn new_function_environment(
        &mut self,
        outer: Option<Handle>,
        slots: u32,
        names: SlotNames,
    ) -> Completion<Handle> {
        self.new_environment(outer, slots, Some(names))
    }
}
