//! Generator objects (`#sec-properties-of-generator-instances`), written by
//! `story:generator-functions-and-suspended-frames`: the kind of an object a generator function
//! makes, with [[GeneratorState]] and [[GeneratorContext]], the suspended frame of
//! `interp/suspend.rs` (`docs/design/engine.md`, decision 3, "Suspended frames", item 2): the
//! frame lies in a box in the kind, so the record of every object keeps its size, and the
//! collector traces it at this kind's site, never as a root. [[GeneratorBrand]] is empty for
//! every generator of this slice, so the kind does not hold it. Every internal method of a
//! generator object is the ordinary one.

use crate::heap::collect::Mark;
use crate::interp::suspend::SuspendedFrame;

/// [[GeneratorState]] (`#sec-properties-of-generator-instances`, Table 92).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GeneratorState {
    SuspendedStart,
    SuspendedYield,
    Executing,
    Completed,
}

/// The slots of a generator object.
pub(crate) struct Generator {
    /// [[GeneratorState]].
    pub(crate) state: GeneratorState,
    /// [[GeneratorContext]]: the frame while the generator is suspended; `None` while it runs
    /// (its frame is on the frame stack), once it completed, and after a termination dropped
    /// its frame while it ran.
    pub(crate) context: Option<SuspendedFrame>,
}

impl Generator {
    /// A generator that holds no frame yet: GeneratorStart sets its state and its frame.
    pub(crate) fn new() -> Generator {
        Generator {
            state: GeneratorState::SuspendedStart,
            context: None,
        }
    }

    /// Kind site of the collector: the suspended frame, as `Frame::trace` traces a frame.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/generator.rs Generator::trace");
        if let Some(context) = &self.context {
            context.trace(mark);
        }
    }

    /// A generator with no frame, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> Generator {
        Generator::new()
    }
}

#[cfg(test)]
mod tests {
    use super::super::ObjectRecord;

    /// `size_of::<ObjectRecord>()` on the base of the change, before generators
    /// (`story:generator-functions-and-suspended-frames`).
    const RECORD_BYTES: usize = 200;

    /// "Suspended frames", item 2 (`docs/design/engine.md`): the frame lies in a box in the
    /// generator's kind, so the record of every object keeps its size.
    #[test]
    fn a_generator_keeps_the_object_record_at_its_size() {
        assert_eq!(size_of::<ObjectRecord>(), RECORD_BYTES);
    }
}
