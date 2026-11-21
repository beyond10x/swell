//! The value properties of the global object (ECMA-262 19.1,
//! `#sec-value-properties-of-the-global-object`): `Infinity` (19.1.1), `NaN` (19.1.3) and
//! `undefined` (19.1.4). `globalThis` (19.1.2) is slice 3 (`HOST-155`). Defined on each global
//! object by SetDefaultGlobalBindings (`realm/global_object.rs`). Written by
//! `story:object-model-and-realms`.

use crate::value::Value;

/// The value properties of 19.1 this part defines, by name.
pub(crate) fn value_properties() -> [(&'static str, Value); 3] {
    [
        ("Infinity", Value::Number(f64::INFINITY)),
        ("NaN", Value::Number(f64::NAN)),
        ("undefined", Value::Undefined),
    ]
}
