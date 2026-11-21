//! Values at the boundary (`docs/design/host-interface.md`, decision 4; `docs/design/engine.md`,
//! decision 4, "Value"): the ECMAScript language types of ECMA-262 6.1
//! (`#sec-ecmascript-language-types`) as a `Copy` enum over typed handles, property keys
//! (`#sec-object-type`: "a property key is either a String or a Symbol"), and the roots through
//! which the host keeps a value, a realm, a script or a module across calls.
//!
//! Every handle carries its agent's key, a slot index and a generation. A value handed to host
//! code is valid for the scope it was handed in (a `with_cx` closure, a host function call, a
//! hook); to keep it longer the host makes a [`Root`]. A handle kept past its record's life is
//! answered `Err(HostError::StaleHandle)`, one of another agent `Err(HostError::ForeignHandle)`.
//! Written by `story:heap-values-and-host-surface`.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use crate::heap::arena::Handle;

/// An ECMAScript language value (6.1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Boolean(bool),
    Number(f64),
    String(StringRef),
    Symbol(SymbolRef),
    BigInt(BigIntRef),
    Object(ObjectRef),
}

/// A handle to an Object (6.1.7, `#sec-object-type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectRef {
    pub(crate) handle: Handle,
}

/// A handle to a String (6.1.4, `#sec-ecmascript-language-types-string-type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StringRef {
    pub(crate) handle: Handle,
}

/// A handle to a Symbol (6.1.5, `#sec-ecmascript-language-types-symbol-type`). No Symbol exists
/// before the well-known symbols of `story:object-model-and-realms`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SymbolRef {
    pub(crate) handle: Handle,
}

/// A handle to a BigInt (6.1.6.2, `#sec-ecmascript-language-types-bigint-type`). No BigInt
/// exists before slice 12.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BigIntRef {
    pub(crate) handle: Handle,
}

/// A property key (6.1.7, `#sec-object-type`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PropertyKey {
    String(StringRef),
    Symbol(SymbolRef),
}

/// A value the host keeps across calls. Neither `Clone` nor `Copy`; dropping it releases it at
/// the agent's next safe point. `Agent::duplicate` and `Cx::duplicate` make a second one.
pub struct Root {
    pub(crate) key: u32,
    pub(crate) index: u32,
    pub(crate) generation: u32,
    pub(crate) released: Rc<Cell<Vec<(u32, u32)>>>,
}

impl Drop for Root {
    fn drop(&mut self) {
        let mut queue = self.released.take();
        queue.push((self.index, self.generation));
        self.released.set(queue);
    }
}

impl fmt::Debug for Root {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Root")
            .field("key", &self.key)
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

/// A realm (9.3, `#sec-code-realms`), held through a root. Made by `create_realm`
/// (`story:object-model-and-realms`).
#[derive(Debug)]
pub struct Realm(#[expect(dead_code, reason = "realms: story:object-model-and-realms")] Root);

/// A parsed script (16.1, `#sec-scripts`), held through a root. Made by `parse_script`
/// (`story:interpreter-and-script-entry-points`).
#[derive(Debug)]
pub struct Script(
    #[expect(
        dead_code,
        reason = "scripts: story:interpreter-and-script-entry-points"
    )]
    Root,
);

/// A module (16.2), held through a root. Made by `parse_module` (slice 11).
#[derive(Debug)]
pub struct Module(#[expect(dead_code, reason = "modules: slice 11")] Root);

/// A realm, as a scoped handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RealmRef {
    pub(crate) handle: Handle,
}

/// A script, as a scoped handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScriptRef {
    pub(crate) handle: Handle,
}

/// A module, as a scoped handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ModuleRef {
    pub(crate) handle: Handle,
}

#[cfg(test)]
mod tests {
    use super::Value;

    /// The epic's item 14 and `engine.md` decision 4: a `Value` is 16 bytes (an 8-byte payload
    /// and a tag, aligned to 8; a handle is three `u32`s and fits beside the tag).
    #[test]
    fn value_is_16_bytes() {
        assert_eq!(std::mem::size_of::<Value>(), 16);
    }
}
