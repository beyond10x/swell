//! Async generator objects (`#sec-properties-of-asyncgenerator-intances`), written by
//! `story:async-generators-and-their-request-queue`: the kind of an object an async generator
//! function makes (`docs/design/engine.md`, decision 3, "Async generators on suspended frames",
//! items 1 and 2), with [[AsyncGeneratorState]], [[AsyncGeneratorContext]] (the suspended frame
//! of `interp/suspend.rs`, in a box with the guard of the credit queue beside it, as a
//! generator's), the number of its current wait (an `await`, or the wait of a `return`,
//! AsyncGeneratorAwaitReturn) and [[AsyncGeneratorQueue]] in a box, so the record of every
//! object keeps its size. [[GeneratorBrand]] is empty for every async generator of the slice, so
//! the kind does not hold it. Every internal method of an async generator object is the
//! ordinary one.
//!
//! The queue is a `VecDeque` of [`Request`]s of one fixed size (AsyncGeneratorRequest Records,
//! `#sec-asyncgeneratorrequest-records`): the completion's tag and value, the capability's
//! promise and its two functions. Its capacity grows by doubling, charged before it grows and
//! recorded in the object's `Grown`, so the sweep that frees the object credits it; settling a
//! request credits nothing (item 2, `builtins/async_generator.rs`). Its kind site,
//! [`AsyncGenerator::trace`], traces the frame and every value of every request, never as a
//! root: a queued request keeps nothing alive but through the object (item 1).

use alloc::boxed::Box;
use alloc::collections::VecDeque;

use crate::heap::collect::Mark;
use crate::interp::suspend::SuspendedFrame;
use crate::value::{ObjectRef, Value};

/// [[AsyncGeneratorState]] (`#sec-properties-of-asyncgenerator-intances`, Table 95).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AsyncGeneratorState {
    SuspendedStart,
    SuspendedYield,
    /// Its frame is on the frame stack, or suspended at an `await` (the draft keeps the state
    /// executing there), or a termination dropped it.
    Executing,
    DrainingQueue,
    Completed,
}

/// An AsyncGeneratorRequest Record (`#sec-asyncgeneratorrequest-records`): [[Completion]] as
/// its tag (`TAG_NORMAL`, `TAG_THROW` or `TAG_RETURN`) and value, and [[Capability]], a promise
/// of `%Promise%` with its resolving functions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Request {
    pub(crate) tag: u32,
    pub(crate) value: Value,
    pub(crate) promise: ObjectRef,
    pub(crate) resolve: ObjectRef,
    pub(crate) reject: ObjectRef,
}

/// The bytes one entry of the queue takes: what each unit of its capacity is charged.
pub(crate) fn request_bytes() -> u64 {
    u64::try_from(size_of::<Request>()).unwrap_or(u64::MAX)
}

/// The bytes of the box of the queue, charged with the object as its owned bytes.
pub(crate) fn queue_box_bytes() -> u64 {
    u64::try_from(size_of::<VecDeque<Request>>()).unwrap_or(u64::MAX)
}

/// The slots of an async generator object.
pub(crate) struct AsyncGenerator {
    /// [[AsyncGeneratorState]].
    pub(crate) state: AsyncGeneratorState,
    /// Whether the generator waits now at its wait numbered `wait`: suspended at an `await`
    /// (its frame in `frame`), or waiting on the value of a `return` (with no frame).
    pub(crate) waiting: bool,
    /// The number of the last wait: 0 before the first, raised by each.
    pub(crate) wait: u64,
    /// [[AsyncGeneratorContext]]: the frame while the generator is suspended at its start, a
    /// `yield` or an `await`; `None` while it runs, once it completed, and after a termination
    /// dropped it while it ran.
    pub(crate) frame: Option<SuspendedFrame>,
    /// [[AsyncGeneratorQueue]], in a box: the kind holds one pointer for it, so the record of
    /// every object keeps its size (item 1).
    #[expect(
        clippy::box_collection,
        reason = "the kind holds one pointer for the queue, so the record of every object keeps its size"
    )]
    pub(crate) queue: Box<VecDeque<Request>>,
}

impl AsyncGenerator {
    /// A generator suspended at its start that holds no frame yet and an empty queue:
    /// AsyncGeneratorStart sets its frame.
    pub(crate) fn new() -> AsyncGenerator {
        AsyncGenerator {
            state: AsyncGeneratorState::SuspendedStart,
            waiting: false,
            wait: 0,
            frame: None,
            queue: Box::default(),
        }
    }

    /// Whether the generator waits at its wait numbered `wait`.
    pub(crate) fn waits_at(&self, wait: u64) -> bool {
        self.waiting && self.wait == wait
    }

    /// Kind site of the collector: the suspended frame, as `Frame::trace` traces a frame, and the
    /// completion's value and the capability of every request of the queue.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/async_generator.rs AsyncGenerator::trace");
        if let Some(frame) = &self.frame {
            frame.trace(mark);
        }
        for request in self.queue.iter() {
            mark.value(request.value);
            mark.object(request.promise);
            mark.object(request.resolve);
            mark.object(request.reject);
        }
    }

    /// An async generator with no frame and one request whose values are `object`, for the
    /// tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test(object: ObjectRef) -> AsyncGenerator {
        let mut generator = AsyncGenerator::new();
        generator.queue.push_back(Request {
            tag: 0,
            value: Value::Object(object),
            promise: object,
            resolve: object,
            reject: object,
        });
        generator
    }
}

#[cfg(test)]
mod tests {
    use super::super::ObjectRecord;
    use super::{Request, request_bytes};

    /// `size_of::<ObjectRecord>()` before async generators, as `object/generator.rs` states it:
    /// 200 bytes, 192 since `story:new-target-meta-property` (the arrow function's box in place
    /// of its `Option<Value>`), and 200 again since `story:class-definition-evaluation` (the
    /// class constructor's box): measured.
    const RECORD_BYTES: usize = 200;

    /// The bytes of a [`Request`] on this target, which the integration test
    /// `async_generators::settling_a_request_credits_nothing_until_the_generator_is_freed` reads
    /// as `REQUEST_BYTES`.
    const REQUEST_BYTES: u64 = 56;

    /// "Async generators on suspended frames", item 1: the frame and the queue lie in boxes in
    /// the kind, so the record of every object keeps its size.
    #[test]
    fn an_async_generator_keeps_the_object_record_at_its_size() {
        assert_eq!(size_of::<ObjectRecord>(), RECORD_BYTES);
    }

    /// Item 2: an entry of the queue is of one fixed size, what each unit of the queue's
    /// capacity is charged.
    #[test]
    fn a_request_keeps_its_size() {
        assert_eq!(request_bytes(), REQUEST_BYTES);
        assert_eq!(size_of::<Request>() as u64, REQUEST_BYTES);
    }
}
