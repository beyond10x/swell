//! The activation of an `async` function (`docs/design/engine.md`, decision 3, "`async`
//! functions on suspended frames", item 1), written by `story:async-functions-and-await`: the
//! holder of the frame of one call of an `async` function or arrow function while it is suspended
//! at an `await`. It is an internal kind of object with a null [[Prototype]], which no
//! instruction and no read hands to a script or a host: the code of the call keeps it in a local
//! slot it reserves, and the two built-in functions of each `await` hold it in their slots
//! (`object/builtin_function.rs`, `Slots::Await`). Its kind holds the slot for one suspended
//! frame (`interp/suspend.rs`), in a box, so the record of every object keeps its size; the
//! state; the number of the last suspension; and the promise of the call with the resolving
//! functions that settle it. Every internal method of an activation is the ordinary one.
//!
//! Its kind site of the collector, [`AsyncActivation::trace`], traces the suspended frame, the
//! promise and its resolving functions, never as a root: an activation lives exactly as long as
//! something that will resume it (a reaction of the promise it awaits, or the job of that
//! reaction) or its running frame reaches it, and the sweep that frees it while it is suspended
//! queues the charge of its frame for the agent to credit.
//!
//! The holder in the slots of the two functions of an `await` is an activation or, in an async
//! generator, the async generator object (`object/async_generator.rs`,
//! `story:async-generators-and-their-request-queue`), which holds the frame and the number of its
//! wait itself; an async generator has no activation.

use crate::heap::collect::Mark;
use crate::interp::suspend::SuspendedFrame;
use crate::value::ObjectRef;

/// The state of an activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActivationState {
    /// Its frame is on the frame stack, or a termination dropped it there.
    Executing,
    /// Its frame is in the activation, suspended at an `await` whose number is the activation's
    /// `suspension`.
    Suspended,
    /// Its body returned or threw; it holds no frame.
    Completed,
}

/// The slots of an activation.
pub(crate) struct AsyncActivation {
    pub(crate) state: ActivationState,
    /// The number of the last suspension: 0 before the first `await`, raised by each.
    pub(crate) suspension: u64,
    /// The frame while the activation is suspended.
    pub(crate) frame: Option<SuspendedFrame>,
    /// The promise of the call, and its resolve and reject functions.
    pub(crate) promise: ObjectRef,
    pub(crate) resolve: ObjectRef,
    pub(crate) reject: ObjectRef,
}

impl AsyncActivation {
    /// An executing activation of the call whose promise is `promise`, with no frame.
    pub(crate) fn new(promise: ObjectRef, resolve: ObjectRef, reject: ObjectRef) -> Self {
        AsyncActivation {
            state: ActivationState::Executing,
            suspension: 0,
            frame: None,
            promise,
            resolve,
            reject,
        }
    }

    /// Whether the activation is suspended at the suspension numbered `suspension`.
    pub(crate) fn suspended_at(&self, suspension: u64) -> bool {
        self.state == ActivationState::Suspended && self.suspension == suspension
    }

    /// Kind site of the collector: the suspended frame, as `Frame::trace` traces a frame, the
    /// promise of the call and its resolving functions.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/async_function.rs AsyncActivation::trace");
        if let Some(frame) = &self.frame {
            frame.trace(mark);
        }
        mark.object(self.promise);
        mark.object(self.resolve);
        mark.object(self.reject);
    }

    /// An activation of `object` with no frame, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(object: ObjectRef) -> Self {
        AsyncActivation::new(object, object, object)
    }
}
