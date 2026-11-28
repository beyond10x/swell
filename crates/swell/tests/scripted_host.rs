//! The scripted host type of `support/scripted.rs`
//! (`story:slow-lane-holds-its-bound-on-the-slower-runner`): its accessor of the test's state
//! refuses another type, naming both, and a required hook left to `Hooks::PROVIDED` stops the
//! test, naming the hook, when the engine calls it.
#![cfg(test)]

use crate::support;

use support::host::{create_realm, limits, seeds};
use support::scripted::{Hooks, Scripted};
use swell::{Agent, Outcome, Parsed, Source};

/// The accessor of a state of another type panics, naming the type held and the type asked.
#[test]
#[should_panic(expected = "the scripted host holds a u8, not a u16")]
fn the_accessor_of_another_type_panics_naming_both() {
    let host = Scripted::new(1u8, Hooks::PROVIDED);
    let _ = host.state::<u16>();
}

/// A script that reads the clock of a scripted host whose table leaves the required clock hook
/// to `Hooks::PROVIDED` stops with the panic that names the hook.
#[test]
#[should_panic(
    expected = "the test gave the scripted host no host_system_utc_epoch_nanoseconds: a required hook has no default"
)]
fn a_required_hook_left_to_the_provided_table_panics_naming_it() {
    let mut agent = Agent::new(Scripted::new((), Hooks::PROVIDED), limits(), seeds()).unwrap();
    let Ok(Outcome::Normal(realm)) = create_realm(&mut agent, None, None) else {
        panic!("create_realm did not answer Normal");
    };
    let Ok(Parsed::Parsed(script)) = agent.parse_script(&realm, Source::Utf8("Date.now();"), None)
    else {
        panic!("the script did not parse");
    };
    let _ = agent.evaluate_script(&script);
}
