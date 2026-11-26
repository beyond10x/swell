//! The checks the internal methods of a Proxy make on a trap's answer
//! (`#sec-invariants-of-the-essential-internal-methods`; ECMA-262 10.5.7 to 10.5.10 and 10.5.13),
//! `story:proxy-objects-and-access-traps`: `has`, `get`, `set`, `deleteProperty` and
//! `construct`. `story:proxy-descriptor-prototype-and-extensibility-traps` adds the checks of the
//! seven other traps here, and `story:host-object-overrides` calls these on the answer of a host
//! object's override.
//!
//! Each check is a function of the facts of the target read so far and the trap's answer; no
//! check reads an object itself. The facts are the target's own descriptor of the key, read by
//! its [[GetOwnProperty]], and whether the target is extensible, read by IsExtensible: the
//! caller reads each at the step and under the condition of its clause, never before, and passes
//! only what it has read. Where a clause reads a fact between two checks, the check is two
//! functions: the first answers what the clause does next ([`Next`]), and the caller reads the
//! next fact only when it is asked to, so a read too many is never made.

use super::property::{Property, Slot};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// What a clause does after a check: answer, or read the next fact of the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next {
    /// The clause answers this Boolean, with no further read.
    Answer(bool),
    /// The clause reads the target's descriptor of the key ([[GetOwnProperty]]).
    ReadDescriptor,
    /// The clause reads whether the target is extensible (IsExtensible).
    ReadExtensible,
}

/// An invariant a trap's answer broke: the caller throws a TypeError that names the trap, the
/// key and [`Broken::text`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Broken {
    /// 10.5.8, step 9.a.i: another value for a non-configurable, non-writable data property.
    GetValue,
    /// 10.5.8, step 9.b.i: a value other than `undefined` for a non-configurable accessor
    /// without a getter.
    GetWithoutGetter,
    /// 10.5.9, step 10.a.i: true for another value of a non-configurable, non-writable data
    /// property.
    SetValue,
    /// 10.5.9, step 10.b.i: true for a non-configurable accessor without a setter.
    SetWithoutSetter,
    /// 10.5.7, step 8.b.i: false for a non-configurable own property.
    HasHidesNonConfigurable,
    /// 10.5.7, step 8.b.iii: false for an own property of a target that is not extensible.
    HasHidesOnNonExtensible,
    /// 10.5.10, step 11: true for a non-configurable own property.
    DeleteNonConfigurable,
    /// 10.5.10, step 13: true for an own property of a target that is not extensible.
    DeleteOnNonExtensible,
    /// 10.5.13, step 10: an answer that is not an Object.
    ConstructNotAnObject,
}

impl Broken {
    /// The invariant, for the message of the TypeError.
    pub(crate) fn text(self) -> &'static str {
        match self {
            Broken::GetValue => {
                "the answer differs from the value of a non-configurable, non-writable own data property of the target"
            }
            Broken::GetWithoutGetter => {
                "the answer is not undefined for a non-configurable own accessor property of the target without a getter"
            }
            Broken::SetValue => {
                "true for a value that differs from a non-configurable, non-writable own data property of the target"
            }
            Broken::SetWithoutSetter => {
                "true for a non-configurable own accessor property of the target without a setter"
            }
            Broken::HasHidesNonConfigurable => {
                "false for a non-configurable own property of the target"
            }
            Broken::HasHidesOnNonExtensible => {
                "false for an own property of a target that is not extensible"
            }
            Broken::DeleteNonConfigurable => {
                "true for a non-configurable own property of the target"
            }
            Broken::DeleteOnNonExtensible => {
                "true for an own property of a target that is not extensible"
            }
            Broken::ConstructNotAnObject => "the answer is not an object",
        }
    }
}

/// [[HasProperty]] (`#sec-proxy-object-internal-methods-and-internal-slots-hasproperty-p`), step
/// 8 then 9: a true answer is answered with no read; for false the descriptor is read.
pub(crate) fn has_after_trap(answer: bool) -> Next {
    if answer {
        Next::Answer(true)
    } else {
        Next::ReadDescriptor
    }
}

/// [[HasProperty]], steps 8.a and 8.b, after the descriptor is read: none, the answer false; a
/// non-configurable one is broken (8.b.i) with no further read; a configurable one, IsExtensible
/// is read (8.b.ii).
pub(crate) fn has_after_descriptor(descriptor: Option<Property>) -> Result<Next, Broken> {
    match descriptor {
        None => Ok(Next::Answer(false)),
        Some(property) if !property.configurable => Err(Broken::HasHidesNonConfigurable),
        Some(_) => Ok(Next::ReadExtensible),
    }
}

/// [[HasProperty]], step 8.b.iii, after IsExtensible is read: false is broken, otherwise the
/// answer false (step 9).
pub(crate) fn has_after_extensible(extensible: bool) -> Result<bool, Broken> {
    if extensible {
        Ok(false)
    } else {
        Err(Broken::HasHidesOnNonExtensible)
    }
}

/// [[Get]] (`#sec-proxy-object-internal-methods-and-internal-slots-get-p-receiver`), step 9,
/// after the descriptor is read (step 8, whatever the answer): a non-configurable, non-writable
/// data property reports its value (9.a.i, SameValue, whose comparison of two Strings `agent`
/// charges); a non-configurable accessor without a getter reports `undefined` (9.b.i).
/// IsExtensible is never read.
pub(crate) fn get<H: Host>(
    agent: &mut Agent<H>,
    descriptor: Option<Property>,
    answer: Value,
) -> Completion<Result<(), Broken>> {
    let Some(property) = descriptor else {
        return Ok(Ok(()));
    };
    if property.configurable {
        return Ok(Ok(()));
    }
    Ok(match property.slot {
        Slot::Data {
            value,
            writable: false,
        } => {
            if agent.same_value(answer, value)? {
                Ok(())
            } else {
                Err(Broken::GetValue)
            }
        }
        Slot::Accessor { get: None, .. } if answer != Value::Undefined => {
            Err(Broken::GetWithoutGetter)
        }
        _ => Ok(()),
    })
}

/// [[Set]] (`#sec-proxy-object-internal-methods-and-internal-slots-set-p-v-receiver`), step 8: a
/// false answer is answered with no read; after a true one the descriptor is read (step 9).
pub(crate) fn set_after_trap(answer: bool) -> Next {
    if answer {
        Next::ReadDescriptor
    } else {
        Next::Answer(false)
    }
}

/// [[Set]], step 10, after the descriptor is read: true for a non-configurable, non-writable
/// data property only with its value (10.a.i, SameValue of `value`, the value given), and never
/// for a non-configurable accessor without a setter (10.b.i). IsExtensible is never read.
pub(crate) fn set<H: Host>(
    agent: &mut Agent<H>,
    descriptor: Option<Property>,
    value: Value,
) -> Completion<Result<(), Broken>> {
    let Some(property) = descriptor else {
        return Ok(Ok(()));
    };
    if property.configurable {
        return Ok(Ok(()));
    }
    Ok(match property.slot {
        Slot::Data {
            value: current,
            writable: false,
        } => {
            if agent.same_value(value, current)? {
                Ok(())
            } else {
                Err(Broken::SetValue)
            }
        }
        Slot::Accessor { set: None, .. } => Err(Broken::SetWithoutSetter),
        _ => Ok(()),
    })
}

/// [[Delete]] (`#sec-proxy-object-internal-methods-and-internal-slots-delete-p`), step 8: a
/// false answer is answered with no read; after a true one the descriptor is read (step 9).
pub(crate) fn delete_after_trap(answer: bool) -> Next {
    if answer {
        Next::ReadDescriptor
    } else {
        Next::Answer(false)
    }
}

/// [[Delete]], steps 10 to 12, after the descriptor is read: none, the answer true (step 10); a
/// non-configurable one is broken (step 11) with no further read; otherwise IsExtensible is read
/// (step 12).
pub(crate) fn delete_after_descriptor(descriptor: Option<Property>) -> Result<Next, Broken> {
    match descriptor {
        None => Ok(Next::Answer(true)),
        Some(property) if !property.configurable => Err(Broken::DeleteNonConfigurable),
        Some(_) => Ok(Next::ReadExtensible),
    }
}

/// [[Delete]], step 13, after IsExtensible is read: false is broken, otherwise the answer true
/// (step 14).
pub(crate) fn delete_after_extensible(extensible: bool) -> Result<bool, Broken> {
    if extensible {
        Ok(true)
    } else {
        Err(Broken::DeleteOnNonExtensible)
    }
}

/// [[Construct]] (`#sec-proxy-object-internal-methods-and-internal-slots-construct-argumentslist-newtarget`),
/// step 10: the answer must be an Object. No fact of the target is read; [[Call]] checks
/// nothing.
pub(crate) fn construct(answer: Value) -> Result<ObjectRef, Broken> {
    match answer {
        Value::Object(object) => Ok(object),
        _ => Err(Broken::ConstructNotAnObject),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Broken, Next, construct, delete_after_descriptor, delete_after_extensible,
        delete_after_trap, has_after_descriptor, has_after_extensible, has_after_trap,
        set_after_trap,
    };
    use crate::object::property::Property;
    use crate::value::Value;

    /// The order of reads of the checks with no SameValue: each answers, breaks or asks for the
    /// next fact as its step says, and never asks for a fact its clause does not read there.
    #[test]
    fn each_check_asks_for_the_next_fact_only_where_its_clause_reads_it() {
        let configurable = Property::data(Value::Null, true, true, true);
        let fixed = Property::data(Value::Null, true, true, false);
        assert_eq!(has_after_trap(true), Next::Answer(true));
        assert_eq!(has_after_trap(false), Next::ReadDescriptor);
        assert_eq!(has_after_descriptor(None), Ok(Next::Answer(false)));
        assert_eq!(
            has_after_descriptor(Some(fixed)),
            Err(Broken::HasHidesNonConfigurable)
        );
        assert_eq!(
            has_after_descriptor(Some(configurable)),
            Ok(Next::ReadExtensible)
        );
        assert_eq!(has_after_extensible(true), Ok(false));
        assert_eq!(
            has_after_extensible(false),
            Err(Broken::HasHidesOnNonExtensible)
        );
        assert_eq!(set_after_trap(false), Next::Answer(false));
        assert_eq!(set_after_trap(true), Next::ReadDescriptor);
        assert_eq!(delete_after_trap(false), Next::Answer(false));
        assert_eq!(delete_after_trap(true), Next::ReadDescriptor);
        assert_eq!(delete_after_descriptor(None), Ok(Next::Answer(true)));
        assert_eq!(
            delete_after_descriptor(Some(fixed)),
            Err(Broken::DeleteNonConfigurable)
        );
        assert_eq!(
            delete_after_descriptor(Some(configurable)),
            Ok(Next::ReadExtensible)
        );
        assert_eq!(delete_after_extensible(true), Ok(true));
        assert_eq!(
            delete_after_extensible(false),
            Err(Broken::DeleteOnNonExtensible)
        );
        assert_eq!(
            construct(Value::Number(1.0)),
            Err(Broken::ConstructNotAnObject)
        );
    }
}
