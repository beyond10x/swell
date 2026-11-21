//! The host interface (`docs/design/host-interface.md`; `docs/design/engine.md`, decision 9): the
//! contexts through which host code reaches a running agent, and `with_cx`. Every other file of
//! `host/` is public through the glob below, so a later story makes a type public by making it
//! `pub` in its file, with no edit of this file or of `lib.rs`.
//!
//! Two contexts (decision 1): `Cx` can run ECMAScript code, `HookCx` can read, create values,
//! parse and finish module loads but runs no code. Both have the methods of this file; the calls
//! that run code are added to `Cx` alone by `story:object-model-and-realms` and
//! `story:interpreter-and-script-entry-points`. Written by `story:heap-values-and-host-surface`.

pub(crate) mod agent;
pub(crate) mod hooks;
pub(crate) mod host_object;
pub(crate) mod interrupt;
pub(crate) mod limits;
pub(crate) mod outcome;

pub use agent::*;
pub use hooks::*;
pub use host_object::*;
pub use interrupt::*;
pub use limits::*;
pub use outcome::*;

use crate::heap::arena::Access;
use crate::object::{ObjectKind, ObjectRecord};
use crate::value::{ObjectRef, Root, StringRef, Value};

/// The running agent and its host, while host code runs inside an entry: a `with_cx` closure or
/// a host function. Calls made through it can run code.
pub struct Cx<'a, H: Host> {
    pub(crate) agent: &'a mut Agent<H>,
}

/// The agent inside a hook that runs no code (decision 1): it has no method that calls a
/// function, constructs or evaluates.
pub struct HookCx<'a, H: Host> {
    pub(crate) agent: &'a mut Agent<H>,
}

impl<H: Host> Agent<H> {
    /// A context for creating and converting values while no code runs. A call made through
    /// it is an outermost entry (decision 2); the values it hands out are valid until `f`
    /// returns, and a value kept longer needs a `Root`. A termination of a call inside `f` is
    /// sticky until `f` returns; after `StepBudget`, `MemoryBudget` and `Interrupted` the next
    /// entry runs afresh, after `EngineFault` the agent is faulted.
    ///
    /// `with_cx` is a safe point: it releases the dropped roots and may collect first. If that
    /// collection finds a stale handle, the agent is faulted and `f` does not run.
    pub fn with_cx<R>(&mut self, f: impl FnOnce(&mut Cx<'_, H>) -> R) -> Result<R, HostError> {
        self.alive()?;
        self.termination = None;
        if self.safe_point().is_err() {
            self.fault();
            return Err(HostError::Faulted);
        }
        let start = self.scope.len();
        let result = f(&mut Cx { agent: self });
        self.scope.truncate(start);
        if self.state == AgentState::Alive {
            self.termination = None;
        }
        Ok(result)
    }

    fn scoped_root(&mut self, value: Value) -> Result<Root, HostError> {
        self.alive()?;
        self.heap.check_value(value).map_err(host_error)?;
        Ok(self.heap.roots.add(value))
    }

    fn scoped_get(&self, root: &Root) -> Result<Value, HostError> {
        self.alive()?;
        self.heap.roots.get(root)
    }

    fn scoped_new_string(&mut self, text: &str) -> Completion<StringRef> {
        self.running()?;
        match self.heap.new_string_from_str(text) {
            Ok(string) => {
                self.hand_out(Value::String(string));
                Ok(string)
            }
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    fn scoped_new_string_utf16(&mut self, units: &[u16]) -> Completion<StringRef> {
        self.running()?;
        match self.heap.new_string_from_units(units) {
            Ok(string) => {
                self.hand_out(Value::String(string));
                Ok(string)
            }
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    fn scoped_to_utf16(&self, string: StringRef) -> Result<Vec<u16>, HostError> {
        self.alive()?;
        Ok(self
            .heap
            .strings
            .get(string.handle)
            .map_err(host_error)?
            .to_utf16())
    }

    fn scoped_to_rust_string_lossy(&self, string: StringRef) -> Result<String, HostError> {
        self.alive()?;
        Ok(self
            .heap
            .strings
            .get(string.handle)
            .map_err(host_error)?
            .to_rust_string_lossy())
    }

    fn scoped_new_host_object(&mut self, spec: HostObjectSpec) -> Completion<ObjectRef> {
        self.running()?;
        // A stale or foreign prototype: the interface gives this call no `HostError` answer,
        // so the run ends as for a stale handle inside the engine (engine.md, decision 4).
        if let Some(prototype) = spec.prototype
            && self.heap.objects.get(prototype.handle).is_err()
        {
            return Err(self.fault());
        }
        let owned = spec.data.as_deref().map_or(0, size_of_val);
        let owned = u64::try_from(owned).unwrap_or(u64::MAX);
        let record = ObjectRecord {
            prototype: spec.prototype,
            kind: ObjectKind::Host(HostObject {
                brand: spec.brand,
                data: spec.data,
            }),
        };
        match self.heap.new_object(record, owned) {
            Ok(object) => {
                self.hand_out(Value::Object(object));
                Ok(object)
            }
            Err(_) => Err(self.terminate(Termination::MemoryBudget)),
        }
    }

    fn scoped_brand(&self, object: ObjectRef) -> Result<Option<Brand>, HostError> {
        self.alive()?;
        let record = self.heap.objects.get(object.handle).map_err(host_error)?;
        Ok(match &record.kind {
            ObjectKind::Host(host) => Some(host.brand),
            _ => None,
        })
    }

    fn scoped_host_data<T: HostData>(
        &self,
        object: ObjectRef,
        brand: Brand,
    ) -> Result<Option<&T>, HostError> {
        self.alive()?;
        let record = self.heap.objects.get(object.handle).map_err(host_error)?;
        Ok(match &record.kind {
            ObjectKind::Host(host) => host.data::<T>(brand),
            _ => None,
        })
    }

    fn scoped_host_data_mut<T: HostData>(
        &mut self,
        object: ObjectRef,
        brand: Brand,
    ) -> Result<Option<&mut T>, HostError> {
        self.alive()?;
        let record = self
            .heap
            .objects
            .get_mut(object.handle)
            .map_err(host_error)?;
        Ok(match &mut record.kind {
            ObjectKind::Host(host) => host.data_mut::<T>(brand),
            _ => None,
        })
    }
}

/// The host's answer for a handle that reaches no record.
fn host_error(access: Access) -> HostError {
    match access {
        Access::Foreign => HostError::ForeignHandle,
        Access::Stale => HostError::StaleHandle,
    }
}

/// The methods `Cx` and `HookCx` share (decisions 1, 4 and 15).
macro_rules! scoped_methods {
    ($cx:ident) => {
        impl<'a, H: Host> $cx<'a, H> {
            /// The host value.
            pub fn host(&self) -> &H {
                &self.agent.host
            }

            /// The host value, to change.
            pub fn host_mut(&mut self) -> &mut H {
                &mut self.agent.host
            }

            /// A root holding `value`, so that it outlives the scope.
            pub fn root(&mut self, value: Value) -> Result<Root, HostError> {
                self.agent.scoped_root(value)
            }

            /// The value `root` holds.
            pub fn get(&self, root: &Root) -> Result<Value, HostError> {
                self.agent.scoped_get(root)
            }

            /// A second root of what `root` holds.
            pub fn duplicate(&mut self, root: &Root) -> Result<Root, HostError> {
                self.agent.duplicate(root)
            }

            /// A new String of the UTF-16 encoding of `text`, taken without loss (HOST-34).
            pub fn new_string(&mut self, text: &str) -> Completion<StringRef> {
                self.agent.scoped_new_string(text)
            }

            /// A new String of exactly `units`, lone surrogates included.
            pub fn new_string_utf16(&mut self, units: &[u16]) -> Completion<StringRef> {
                self.agent.scoped_new_string_utf16(units)
            }

            /// Every code unit of `string`, exactly (Web IDL's `DOMString`, HOST-32).
            pub fn to_utf16(&self, string: StringRef) -> Result<Vec<u16>, HostError> {
                self.agent.scoped_to_utf16(string)
            }

            /// `string` with each lone surrogate replaced by U+FFFD (Web IDL's `USVString`,
            /// HOST-33).
            pub fn to_rust_string_lossy(&self, string: StringRef) -> Result<String, HostError> {
                self.agent.scoped_to_rust_string_lossy(string)
            }

            /// A new host object (decision 15).
            pub fn new_host_object(&mut self, spec: HostObjectSpec) -> Completion<ObjectRef> {
                self.agent.scoped_new_host_object(spec)
            }

            /// The brand of `object`, or `None` when it is not a host object.
            pub fn brand(&self, object: ObjectRef) -> Result<Option<Brand>, HostError> {
                self.agent.scoped_brand(object)
            }

            /// The host data of `object` when its brand is `brand` and its data is a `T`, else
            /// `None` (HOST-108).
            pub fn host_data<T: HostData>(
                &self,
                object: ObjectRef,
                brand: Brand,
            ) -> Result<Option<&T>, HostError> {
                self.agent.scoped_host_data(object, brand)
            }

            /// As `host_data`, to change.
            pub fn host_data_mut<T: HostData>(
                &mut self,
                object: ObjectRef,
                brand: Brand,
            ) -> Result<Option<&mut T>, HostError> {
                self.agent.scoped_host_data_mut(object, brand)
            }
        }
    };
}

scoped_methods!(Cx);
scoped_methods!(HookCx);
