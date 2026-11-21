//! The abstract operations (ECMA-262 7, `#sec-abstract-operations`) and the runtime semantics
//! of the operators (13): type conversion (`conversion`, 7.1), testing and comparison
//! (`testing`, 7.2), operations on objects (`objects`, 7.3) and the operators (`operators`).
//! Each is a method of the agent, since any of them may run code. Written by
//! `story:object-model-and-realms`.

pub(crate) mod conversion;
pub(crate) mod objects;
pub(crate) mod operators;
pub(crate) mod testing;
