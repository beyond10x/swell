//! The parts the integration tests of `swell` share: a test host that records every hook call,
//! the limits and seeds the tests use (`host`), the script helpers of
//! `story:interpreter-and-script-entry-points` (`script`), and the helpers of the stack tests
//! through builtins that call back into script (`stack`, `story:builtin-group-modules`), and the
//! one host type of the tests whose hooks are values (`scripted`,
//! `story:slow-lane-holds-its-bound-on-the-slower-runner`).
//!
//! Each test file is a module of `tests/main.rs` and reaches this one with `use crate::support;`.
//! A file uses a part of it, so an item one file does not use is not dead code of the suite.
#![expect(
    dead_code,
    reason = "each test file uses a part of the support module, so the suite leaves some items unused"
)]

pub mod host;
pub mod scripted;
pub mod script;
pub mod stack;
