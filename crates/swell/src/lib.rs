#![forbid(unsafe_code)]
//! swell, an ECMAScript engine (`docs/design/engine.md`). This crate is the engine and its host
//! interface (`docs/design/host-interface.md`, written against its two specifications, the ESS
//! model under `spec/` and the `HOST-<n>` requirements).
//!
//! The public interface is the glob re-export of `value` and `host` below and nothing else: a
//! type becomes public by being `pub` in `value.rs` or in a file of `host/`, so no later story
//! edits this file. Every other item of the crate is `pub(crate)`.
//!
//! The layout, one module per clause or decision, and the story that fills each:
//!
//! - `value`: values, property keys, handles and roots (6.1; decision 4).
//! - `host`: `Agent`, `Cx`, `HookCx`, limits, seeds, outcomes, the interrupt handle, host
//!   objects and the `Host` trait.
//! - `heap`: arenas, accounting, the collector, roots, strings and atoms, the keyed map.
//! - `object`: the object record and its kinds (10.1 to 10.4).
//! - `ops`: the abstract operations (7) and the operators (13).
//! - `realm`: realms and their intrinsics (9.3).
//! - `env`: Environment Records (9.1).
//! - `interp`: the frame stack, the instruction loop, the budgets.
//! - `script`: Script Records (16.1).
//! - `dynamic_code`: source text compiled at run time, `eval` and the `Function` constructor
//!   (19.2.1, 20.2.1).
//! - `builtins`: one module per clause of the builtins.
//! - `compile_fail`: the requirements the compiler holds, as `compile_fail` doc tests.

mod builtins;
mod compile_fail;
mod dynamic_code;
mod env;
mod heap;
mod host;
mod interp;
mod object;
mod ops;
mod realm;
mod script;
mod value;

pub use host::*;
pub use value::*;
