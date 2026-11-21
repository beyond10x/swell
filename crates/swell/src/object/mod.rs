//! Objects (ECMA-262 10.1 to 10.4, `#sec-ordinary-and-exotic-objects-behaviours`, without bound
//! and ECMAScript function objects): the record every object has, its kind, and the
//! internal-method dispatch over the kinds. `story:heap-values-and-host-surface` writes the
//! record with [[Prototype]], the kinds, and the collector's kind site with its host object arm;
//! `story:object-model-and-realms` adds the properties, the internal methods (6.1.7.2,
//! `#sec-object-internal-methods-and-internal-slots`) and their dispatch arms, and one marking
//! arm per kind beside them.

pub(crate) mod array;
pub(crate) mod bound_function;
pub(crate) mod builtin_function;
pub(crate) mod ecmascript_function;
pub(crate) mod ordinary;
pub(crate) mod property;
pub(crate) mod string;

use crate::heap::collect::Mark;
use crate::host::agent::Agent;
use crate::host::host_object::HostObject;
use crate::host::{Abrupt, Completion, Host, Termination};
use crate::value::ObjectRef;
use array::Array;
use bound_function::BoundFunction;
use builtin_function::BuiltinFunction;
use ecmascript_function::EcmascriptFunction;
use ordinary::Ordinary;
use string::StringObject;

/// The record of every object.
pub(crate) struct ObjectRecord {
    /// [[Prototype]].
    pub(crate) prototype: Option<ObjectRef>,
    pub(crate) kind: ObjectKind,
}

/// What kind of object a record is, with the slots of that kind.
pub(crate) enum ObjectKind {
    /// An ordinary object (10.1), and the Boolean and Number wrappers.
    #[cfg_attr(not(test), expect(dead_code, reason = "story:object-model-and-realms"))]
    Ordinary(Ordinary),
    /// An Array exotic object (10.4.2).
    #[cfg_attr(not(test), expect(dead_code, reason = "story:object-model-and-realms"))]
    Array(Array),
    /// A String exotic object (10.4.3).
    #[cfg_attr(not(test), expect(dead_code, reason = "story:object-model-and-realms"))]
    String(StringObject),
    /// A built-in function object (10.3).
    #[cfg_attr(not(test), expect(dead_code, reason = "story:object-model-and-realms"))]
    BuiltinFunction(BuiltinFunction),
    /// A bound function exotic object (10.4.1).
    #[cfg_attr(not(test), expect(dead_code, reason = "story:harness-builtins"))]
    BoundFunction(BoundFunction),
    /// An ECMAScript function object (10.2).
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "story:interpreter-and-script-entry-points")
    )]
    EcmascriptFunction(EcmascriptFunction),
    /// A host object (`docs/design/host-interface.md`, decision 15).
    Host(HostObject),
}

impl ObjectRecord {
    /// Kind site of the collector: [[Prototype]], then the slots of the object's kind.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/mod.rs ObjectRecord::trace");
        if let Some(prototype) = self.prototype {
            mark.object(prototype);
        }
        match &self.kind {
            ObjectKind::Ordinary(ordinary) => ordinary.trace(mark),
            ObjectKind::Array(array) => array.trace(mark),
            ObjectKind::String(string) => string.trace(mark),
            ObjectKind::BuiltinFunction(function) => function.trace(mark),
            ObjectKind::BoundFunction(function) => function.trace(mark),
            ObjectKind::EcmascriptFunction(function) => function.trace(mark),
            ObjectKind::Host(host) => host.trace(mark),
        }
    }
}

impl<H: Host> Agent<H> {
    /// The record of `object`, for the engine's own use inside an entry. A handle that reaches
    /// no record here is an engine bug: the run ends in `Terminated(EngineFault)` and the agent
    /// is faulted (`docs/design/engine.md`, decision 4).
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the internal methods: story:object-model-and-realms"
        )
    )]
    pub(crate) fn engine_object(&mut self, object: ObjectRef) -> Completion<&ObjectRecord> {
        if self.heap.objects.get(object.handle).is_err() {
            return Err(self.fault());
        }
        // The check above passed, so this answers the record.
        self.heap
            .objects
            .get(object.handle)
            .map_err(|_| Abrupt::Terminated(Termination::EngineFault))
    }
}
