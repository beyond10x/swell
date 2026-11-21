//! The parts the integration tests of `swell` share: a test host that records every hook call,
//! the limits and seeds the tests use (`host`), and the script helpers of
//! `story:interpreter-and-script-entry-points` (`script`).
//!
//! Each test file declares `mod support;` and uses a part of it, so an item one file does not use
//! is not dead code of the suite.
#![allow(dead_code)]

pub mod host;
pub mod script;
