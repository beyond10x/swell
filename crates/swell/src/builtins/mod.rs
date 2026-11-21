//! The builtins, one module per clause of ECMA-262 (`docs/design/engine.md`, decision 2). Slice
//! 1 has the ones the harness files call: `global`, `error`, `boolean` and `number` are
//! `story:object-model-and-realms`; `object`, `function`, `array`, `string`, `math` and `json`
//! are `story:harness-builtins`. Later slices add their files here.

pub(crate) mod array;
pub(crate) mod boolean;
pub(crate) mod error;
pub(crate) mod function;
pub(crate) mod global;
pub(crate) mod json;
pub(crate) mod math;
pub(crate) mod number;
pub(crate) mod object;
pub(crate) mod string;
