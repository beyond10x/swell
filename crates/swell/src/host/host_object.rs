//! Host objects (`docs/design/host-interface.md`, decision 15; `docs/design/engine.md`, decision 9,
//! "Host objects for the DOM"): objects the host makes with a brand (the Web IDL interface they
//! implement), host data of any Rust type that implements [`HostData`], and the host data's
//! `trace`, which the collector calls so that what the data holds stays alive exactly as long
//! as the object. The host data is dropped by the sweep that frees its object.
//!
//! Kind site of the collector: [`HostObject::trace`]. `overrides` (slice 9) and `is_html_dda`
//! (slice 18) are added to [`HostObjectSpec`] by the slices of their requirements. Written by
//! `story:heap-values-and-host-surface`.

use std::any::Any;

use crate::heap::collect::Mark;
use crate::value::{ObjectRef, Value};

/// The brand of a host object: which Web IDL interface it implements, in the host's numbering
/// (`webidl:is-a-platform-object`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Brand(pub u32);

/// Host data: Rust state a host object owns. `trace` lists every value the data holds, and is
/// called by every collection that reaches the object; a value it does not list may be freed,
/// and a listed handle that is stale ends the collection in `Terminated(EngineFault)`.
pub trait HostData: Any {
    fn trace(&self, tracer: &mut Tracer<'_>);
}

/// What `HostData::trace` reports to: the collector's worklist.
pub struct Tracer<'t> {
    mark: &'t mut Mark,
}

impl Tracer<'_> {
    /// Keep the record `value` holds, if any.
    pub fn value(&mut self, value: Value) {
        self.mark.value(value);
    }

    /// Keep `object`.
    pub fn object(&mut self, object: ObjectRef) {
        self.mark.object(object);
    }
}

/// What `new_host_object` makes.
pub struct HostObjectSpec {
    /// [[Prototype]].
    pub prototype: Option<ObjectRef>,
    pub brand: Brand,
    pub data: Option<Box<dyn HostData>>,
}

/// The kind part of a host object's record.
pub(crate) struct HostObject {
    pub(crate) brand: Brand,
    pub(crate) data: Option<Box<dyn HostData>>,
}

impl HostObject {
    /// Kind site of the collector: what the host data lists.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("host/host_object.rs HostObject::trace");
        if let Some(data) = &self.data {
            data.trace(&mut Tracer { mark });
        }
    }

    /// The data, when the brand is `brand` and the data is a `T`.
    pub(crate) fn data<T: HostData>(&self, brand: Brand) -> Option<&T> {
        if self.brand != brand {
            return None;
        }
        let data: &dyn Any = self.data.as_deref()?;
        data.downcast_ref::<T>()
    }

    /// The data to change, when the brand is `brand` and the data is a `T`.
    pub(crate) fn data_mut<T: HostData>(&mut self, brand: Brand) -> Option<&mut T> {
        if self.brand != brand {
            return None;
        }
        let data: &mut dyn Any = self.data.as_deref_mut()?;
        data.downcast_mut::<T>()
    }

    /// A host object with no data, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> HostObject {
        HostObject {
            brand: Brand(0),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::object::ordinary::Ordinary;
    use crate::object::{ObjectKind, ObjectRecord};
    use crate::{Agent, Brand, Host, HostData, Limits, Seeds, Tracer, Value};

    struct NoHooks;

    impl Host for NoHooks {}

    struct Data;

    impl HostData for Data {
        fn trace(&self, _tracer: &mut Tracer<'_>) {}
    }

    /// HOST-108's last case, which no script can make before `story:object-model-and-realms`:
    /// an object that is not a host object has no brand and no host data.
    #[test]
    fn an_object_that_is_not_a_host_object_has_no_brand_and_no_data() {
        let limits = Limits {
            steps_per_entry: 1_000,
            memory_bytes: 1 << 20,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        };
        let mut agent = Agent::new(
            NoHooks,
            limits,
            Seeds {
                hash_key: [1; 16],
                random_seed: [2; 32],
            },
        )
        .unwrap();
        let record = ObjectRecord {
            prototype: None,
            kind: ObjectKind::Ordinary(Ordinary::for_test()),
        };
        let object = agent.heap.new_object(record, 0).unwrap();
        let _root = agent.heap.roots.add(Value::Object(object));
        agent
            .with_cx(|cx| {
                assert_eq!(cx.brand(object), Ok(None));
                assert!(cx.host_data::<Data>(object, Brand(0)).unwrap().is_none());
                assert!(
                    cx.host_data_mut::<Data>(object, Brand(0))
                        .unwrap()
                        .is_none()
                );
            })
            .unwrap();
    }
}
