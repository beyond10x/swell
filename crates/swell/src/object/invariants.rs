//! The checks the internal methods of a Proxy make on a trap's answer
//! (`#sec-invariants-of-the-essential-internal-methods`; ECMA-262 10.5.1 to 10.5.11 and 10.5.13).
//! `story:proxy-objects-and-access-traps` wrote those of `has`, `get`, `set`, `deleteProperty`
//! and `construct`; `story:proxy-descriptor-prototype-and-extensibility-traps` adds those of the
//! seven other traps, the 21 invariants it numbers I1 to I21 (each [`Broken`] variant names its
//! number), IsCompatiblePropertyDescriptor and CompletePropertyDescriptor.
//! `story:host-object-overrides` calls these on the answer of a host object's override, with the
//! facts the engine recorded for the host object in place of a target's.
//!
//! Each check is a function of the facts of the target read so far and the trap's answer; no
//! check reads an object itself. The facts are the target's prototype ([[GetPrototypeOf]]),
//! whether it is extensible (IsExtensible), its own descriptor of the key ([[GetOwnProperty]])
//! and its own keys with whether each is configurable ([[OwnPropertyKeys]], then
//! [[GetOwnProperty]] of each): the caller reads each at the step and under the condition of its
//! clause, never before, and passes only what it has read. Where a clause reads a fact between
//! two checks, the check is two functions: the first answers what the clause does next
//! ([`Next`]), and the caller reads the next fact only when it is asked to, so a read too many is
//! never made. The checks that compare values (SameValue inside IsCompatiblePropertyDescriptor)
//! take the agent, whose comparison of two Strings is charged; they read no object either.

use super::ordinary::applied_property;
use super::property::{Descriptor, Key, Property, Slot};
use crate::host::Host;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// What a clause does after a check: answer, or read the next fact of the target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Next {
    /// The clause answers this Boolean, with no further read.
    Answer(bool),
    /// The clause answers the trap's result as it is, with no further read.
    TrapResult,
    /// The clause reads the target's descriptor of the key ([[GetOwnProperty]]).
    ReadDescriptor,
    /// The clause reads whether the target is extensible (IsExtensible).
    ReadExtensible,
    /// The clause reads the target's prototype ([[GetPrototypeOf]]).
    ReadPrototype,
    /// The clause goes on to its next check of the lists it holds, with no read.
    Check,
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
    /// I1, 10.5.1, step 8: an answer of `getPrototypeOf` that is neither an Object nor null.
    PrototypeNotAnObjectOrNull,
    /// I2, 10.5.1, step 12: for a target that is not extensible, an answer that is not
    /// SameValue with the target's prototype.
    PrototypeOfNonExtensible,
    /// I3, 10.5.2, step 12: true for a target that is not extensible and a value that is not
    /// SameValue with its prototype.
    SetPrototypeOfNonExtensible,
    /// I4, 10.5.3, step 9: an answer that differs from the target's IsExtensible.
    ExtensibleDiffers,
    /// I5, 10.5.4, step 8.b: true for a target that is still extensible.
    PreventedButExtensible,
    /// I6, 10.5.5, step 8: an answer of `getOwnPropertyDescriptor` that is neither an Object
    /// nor undefined.
    DescriptorNotAnObjectOrUndefined,
    /// I7, 10.5.5, step 10.b: undefined for a non-configurable own property of the target.
    HidesNonConfigurable,
    /// I8, 10.5.5, step 10.d: undefined for an own property of a target that is not
    /// extensible.
    HidesOnNonExtensible,
    /// I9, 10.5.5, step 15: a descriptor not compatible with the target's property
    /// (IsCompatiblePropertyDescriptor).
    DescriptorIncompatible,
    /// I10, 10.5.5, step 16.a: non-configurable for a property the target lacks or has
    /// configurable.
    ReportsNonConfigurable,
    /// I11, 10.5.5, step 16.b.ii: non-configurable and non-writable for a writable property of
    /// the target.
    ReportsNonWritable,
    /// I12, 10.5.6, step 14.a: true for a property a target that is not extensible lacks.
    DefinesOnNonExtensible,
    /// I13, 10.5.6, step 14.b: true for `configurable: false` and a property the target lacks.
    DefinesNonConfigurableAbsent,
    /// I14, 10.5.6, step 15.a: true for a descriptor not compatible with the target's property.
    DefinesIncompatible,
    /// I15, 10.5.6, step 15.b: true for `configurable: false` and a configurable property of
    /// the target.
    DefinesNonConfigurable,
    /// I16, 10.5.6, step 15.c.i: true for `writable: false` and a non-configurable, writable
    /// data property of the target.
    DefinesNonWritable,
    /// I17, 10.5.11, step 8 (CreateListFromArrayLike with property-key): an element that is
    /// neither a String nor a Symbol.
    KeyNotAStringOrSymbol,
    /// I18, 10.5.11, step 9: a key listed twice.
    KeyRepeated,
    /// I19, 10.5.11, step 19.a: a non-configurable own key of the target is missing.
    NonConfigurableKeyMissing,
    /// I20, 10.5.11, step 21.a: a configurable own key of a target that is not extensible is
    /// missing.
    KeyOfNonExtensibleMissing,
    /// I21, 10.5.11, step 22: a key a target that is not extensible lacks.
    KeyOfNonExtensibleExtra,
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
            Broken::PrototypeNotAnObjectOrNull => "I1: the answer is neither an object nor null",
            Broken::PrototypeOfNonExtensible => {
                "I2: the target is not extensible and the answer is not its prototype"
            }
            Broken::SetPrototypeOfNonExtensible => {
                "I3: true for a target that is not extensible and a value that is not its prototype"
            }
            Broken::ExtensibleDiffers => {
                "I4: the answer differs from whether the target is extensible"
            }
            Broken::PreventedButExtensible => "I5: true for a target that is still extensible",
            Broken::DescriptorNotAnObjectOrUndefined => {
                "I6: the answer is neither an object nor undefined"
            }
            Broken::HidesNonConfigurable => {
                "I7: undefined for a non-configurable own property of the target"
            }
            Broken::HidesOnNonExtensible => {
                "I8: undefined for an own property of a target that is not extensible"
            }
            Broken::DescriptorIncompatible => {
                "I9: the descriptor is not compatible with the target's property"
            }
            Broken::ReportsNonConfigurable => {
                "I10: non-configurable for a property the target lacks or has configurable"
            }
            Broken::ReportsNonWritable => {
                "I11: non-configurable and non-writable for a writable property of the target"
            }
            Broken::DefinesOnNonExtensible => {
                "I12: true for a property a target that is not extensible lacks"
            }
            Broken::DefinesNonConfigurableAbsent => {
                "I13: true for a non-configurable definition of a property the target lacks"
            }
            Broken::DefinesIncompatible => {
                "I14: true for a descriptor not compatible with the target's property"
            }
            Broken::DefinesNonConfigurable => {
                "I15: true for a non-configurable definition of a configurable property of the target"
            }
            Broken::DefinesNonWritable => {
                "I16: true for a non-writable definition of a non-configurable, writable property of the target"
            }
            Broken::KeyNotAStringOrSymbol => "I17: an element is neither a String nor a Symbol",
            Broken::KeyRepeated => "I18: the key is listed more than once",
            Broken::NonConfigurableKeyMissing => {
                "I19: a non-configurable own key of the target is missing"
            }
            Broken::KeyOfNonExtensibleMissing => {
                "I20: an own key of a target that is not extensible is missing"
            }
            Broken::KeyOfNonExtensibleExtra => {
                "I21: a key a target that is not extensible lacks is listed"
            }
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

/// CompletePropertyDescriptor (`#sec-completepropertydescriptor`): the fields `descriptor` lacks
/// take the values of `like` ({ [[Value]]: undefined, [[Writable]]: false, [[Getter]]:
/// undefined, [[Setter]]: undefined, [[Enumerable]]: false, [[Configurable]]: false }), a
/// generic descriptor completed as a data one; the answer is the complete descriptor as a
/// property.
pub(crate) fn complete_property_descriptor(descriptor: &Descriptor) -> Property {
    applied_property(descriptor, None)
}

/// The descriptor of every field of `property`.
pub(crate) fn descriptor_of(property: Property) -> Descriptor {
    let mut descriptor = Descriptor {
        enumerable: Some(property.enumerable),
        configurable: Some(property.configurable),
        ..Descriptor::default()
    };
    match property.slot {
        Slot::Data { value, writable } => {
            descriptor.value = Some(value);
            descriptor.writable = Some(writable);
        }
        Slot::Accessor { get, set } => {
            descriptor.get = Some(get);
            descriptor.set = Some(set);
        }
    }
    descriptor
}

/// IsCompatiblePropertyDescriptor (`#sec-iscompatiblepropertydescriptor`):
/// ValidateAndApplyPropertyDescriptor with no object, so nothing is applied; its SameValue of
/// two Strings is charged by `agent`.
pub(crate) fn is_compatible_property_descriptor<H: Host>(
    agent: &mut Agent<H>,
    extensible: bool,
    descriptor: Descriptor,
    current: Option<Property>,
) -> Completion<bool> {
    agent.validate_and_apply_property_descriptor(
        None,
        Key::Index(0),
        extensible,
        descriptor,
        current,
    )
}

/// [[GetPrototypeOf]] (`#sec-proxy-object-internal-methods-and-internal-slots-getprototypeof`),
/// step 8 (I1), before any read: the answer as a prototype.
pub(crate) fn get_prototype_of_answer(answer: Value) -> Result<Option<ObjectRef>, Broken> {
    match answer {
        Value::Object(object) => Ok(Some(object)),
        Value::Null => Ok(None),
        _ => Err(Broken::PrototypeNotAnObjectOrNull),
    }
}

/// [[GetPrototypeOf]], step 10, after IsExtensible is read (step 9): an extensible target
/// answers the trap's result; for one that is not, its prototype is read (step 11).
pub(crate) fn get_prototype_of_after_extensible(extensible: bool) -> Next {
    if extensible {
        Next::TrapResult
    } else {
        Next::ReadPrototype
    }
}

/// [[GetPrototypeOf]], step 12 (I2), after the target's prototype is read: SameValue of two
/// Objects or null is their identity.
pub(crate) fn get_prototype_of_after_prototype(
    answer: Option<ObjectRef>,
    prototype: Option<ObjectRef>,
) -> Result<(), Broken> {
    if answer == prototype {
        Ok(())
    } else {
        Err(Broken::PrototypeOfNonExtensible)
    }
}

/// [[SetPrototypeOf]] (`#sec-proxy-object-internal-methods-and-internal-slots-setprototypeof-v`),
/// step 8: false answers false with no read; after true IsExtensible is read (step 9).
pub(crate) fn set_prototype_of_after_trap(answer: bool) -> Next {
    if answer {
        Next::ReadExtensible
    } else {
        Next::Answer(false)
    }
}

/// [[SetPrototypeOf]], step 10: an extensible target answers true; for one that is not, its
/// prototype is read (step 11).
pub(crate) fn set_prototype_of_after_extensible(extensible: bool) -> Next {
    if extensible {
        Next::Answer(true)
    } else {
        Next::ReadPrototype
    }
}

/// [[SetPrototypeOf]], step 12 (I3), after the target's prototype is read: the value set must be
/// SameValue with it; the answer is true (step 13).
pub(crate) fn set_prototype_of_after_prototype(
    value: Option<ObjectRef>,
    prototype: Option<ObjectRef>,
) -> Result<bool, Broken> {
    if value == prototype {
        Ok(true)
    } else {
        Err(Broken::SetPrototypeOfNonExtensible)
    }
}

/// [[IsExtensible]] (`#sec-proxy-object-internal-methods-and-internal-slots-isextensible`), step
/// 9 (I4), after the target's IsExtensible is read (step 8): the answer must be the same.
pub(crate) fn is_extensible_after_target(answer: bool, target: bool) -> Result<bool, Broken> {
    if answer == target {
        Ok(answer)
    } else {
        Err(Broken::ExtensibleDiffers)
    }
}

/// [[PreventExtensions]] (`#sec-proxy-object-internal-methods-and-internal-slots-preventextensions`),
/// step 8: false answers false with no read (step 9); after true IsExtensible is read (8.a).
pub(crate) fn prevent_extensions_after_trap(answer: bool) -> Next {
    if answer {
        Next::ReadExtensible
    } else {
        Next::Answer(false)
    }
}

/// [[PreventExtensions]], step 8.b (I5), after IsExtensible is read: a target that is still
/// extensible breaks it; otherwise the answer true.
pub(crate) fn prevent_extensions_after_extensible(extensible: bool) -> Result<bool, Broken> {
    if extensible {
        Err(Broken::PreventedButExtensible)
    } else {
        Ok(true)
    }
}

/// [[GetOwnProperty]] (`#sec-proxy-object-internal-methods-and-internal-slots-getownproperty-p`),
/// step 8 (I6), before any read: the answer as an Object, or `None` for undefined.
pub(crate) fn get_own_property_answer(answer: Value) -> Result<Option<ObjectRef>, Broken> {
    match answer {
        Value::Object(object) => Ok(Some(object)),
        Value::Undefined => Ok(None),
        _ => Err(Broken::DescriptorNotAnObjectOrUndefined),
    }
}

/// [[GetOwnProperty]], step 10 for an undefined answer, after the target's descriptor is read
/// (step 9): none answers undefined (10.a); a non-configurable one breaks I7 (10.b) with no
/// further read; a configurable one, IsExtensible is read (10.c).
pub(crate) fn get_own_property_undefined_after_descriptor(
    descriptor: Option<Property>,
) -> Result<Next, Broken> {
    match descriptor {
        None => Ok(Next::TrapResult),
        Some(property) if !property.configurable => Err(Broken::HidesNonConfigurable),
        Some(_) => Ok(Next::ReadExtensible),
    }
}

/// [[GetOwnProperty]], step 10.d (I8), after IsExtensible is read: a target that is not
/// extensible breaks it; otherwise the answer undefined (10.e).
pub(crate) fn get_own_property_undefined_after_extensible(extensible: bool) -> Result<(), Broken> {
    if extensible {
        Ok(())
    } else {
        Err(Broken::HidesOnNonExtensible)
    }
}

/// [[GetOwnProperty]], steps 14 to 16, for an Object answer after both facts of the target were
/// read (steps 9 and 11) and the answer was made a complete descriptor (steps 12 and 13):
/// IsCompatiblePropertyDescriptor (I9, 15), then for a non-configurable answer a target property
/// that is non-configurable (I10, 16.a) and, for a non-writable one, non-writable (I11, 16.b).
pub(crate) fn get_own_property_result<H: Host>(
    agent: &mut Agent<H>,
    extensible: bool,
    result: Property,
    target: Option<Property>,
) -> Completion<Result<(), Broken>> {
    if !is_compatible_property_descriptor(agent, extensible, descriptor_of(result), target)? {
        return Ok(Err(Broken::DescriptorIncompatible));
    }
    if result.configurable {
        return Ok(Ok(()));
    }
    let Some(target) = target.filter(|property| !property.configurable) else {
        return Ok(Err(Broken::ReportsNonConfigurable));
    };
    Ok(match (result.slot, target.slot) {
        (
            Slot::Data {
                writable: false, ..
            },
            Slot::Data { writable: true, .. },
        ) => Err(Broken::ReportsNonWritable),
        _ => Ok(()),
    })
}

/// [[DefineOwnProperty]] (`#sec-proxy-object-internal-methods-and-internal-slots-defineownproperty-p-desc`),
/// step 9: false answers false with no read; after true the target's descriptor is read (step
/// 10), then IsExtensible (step 11).
pub(crate) fn define_own_property_after_trap(answer: bool) -> Next {
    if answer {
        Next::ReadDescriptor
    } else {
        Next::Answer(false)
    }
}

/// [[DefineOwnProperty]], steps 12 to 15, after both facts of the target are read: for a
/// property the target lacks, a target that is not extensible (I12, 14.a) and `configurable:
/// false` (I13, 14.b) break it; for one it has, IsCompatiblePropertyDescriptor (I14, 15.a),
/// `configurable: false` for a configurable property (I15, 15.b) and `writable: false` for a
/// non-configurable, writable data property (I16, 15.c). The answer is true (step 16).
pub(crate) fn define_own_property<H: Host>(
    agent: &mut Agent<H>,
    extensible: bool,
    descriptor: Descriptor,
    target: Option<Property>,
) -> Completion<Result<(), Broken>> {
    let setting_configurable_false = descriptor.configurable == Some(false);
    let Some(current) = target else {
        if !extensible {
            return Ok(Err(Broken::DefinesOnNonExtensible));
        }
        if setting_configurable_false {
            return Ok(Err(Broken::DefinesNonConfigurableAbsent));
        }
        return Ok(Ok(()));
    };
    if !is_compatible_property_descriptor(agent, extensible, descriptor, Some(current))? {
        return Ok(Err(Broken::DefinesIncompatible));
    }
    if setting_configurable_false && current.configurable {
        return Ok(Err(Broken::DefinesNonConfigurable));
    }
    if let Slot::Data { writable: true, .. } = current.slot
        && !current.configurable
        && descriptor.writable == Some(false)
    {
        return Ok(Err(Broken::DefinesNonWritable));
    }
    Ok(Ok(()))
}

/// [[OwnPropertyKeys]] (`#sec-proxy-object-internal-methods-and-internal-slots-ownpropertykeys`),
/// step 8 (I17): an element of the trap's answer must be a property key, a String or a Symbol
/// (CreateListFromArrayLike with property-key).
pub(crate) fn own_keys_element(element: Value) -> Result<(), Broken> {
    match element {
        Value::String(_) | Value::Symbol(_) => Ok(()),
        _ => Err(Broken::KeyNotAStringOrSymbol),
    }
}

/// [[OwnPropertyKeys]], step 9 (I18), for each key of the answer: whether it was listed before.
pub(crate) fn own_keys_listed_before(listed: bool) -> Result<(), Broken> {
    if listed {
        Err(Broken::KeyRepeated)
    } else {
        Ok(())
    }
}

/// [[OwnPropertyKeys]], step 17, after IsExtensible (step 10), the target's keys (step 11) and
/// each key's descriptor (step 16) were read: an extensible target with no non-configurable key
/// answers the trap's result; otherwise the checks go on.
pub(crate) fn own_keys_after_target(extensible: bool, non_configurable: bool) -> Next {
    if extensible && !non_configurable {
        Next::TrapResult
    } else {
        Next::Check
    }
}

/// [[OwnPropertyKeys]], steps 19.a (I19) and 21.a (I20), for one key of the target: it must be
/// among the answer's keys not checked yet.
pub(crate) fn own_keys_target_key(unchecked: bool, configurable: bool) -> Result<(), Broken> {
    match (unchecked, configurable) {
        (true, _) => Ok(()),
        (false, false) => Err(Broken::NonConfigurableKeyMissing),
        (false, true) => Err(Broken::KeyOfNonExtensibleMissing),
    }
}

/// [[OwnPropertyKeys]], step 20, after the non-configurable keys were checked: an extensible
/// target answers the trap's result; one that is not checks its configurable keys (step 21).
pub(crate) fn own_keys_after_non_configurable(extensible: bool) -> Next {
    if extensible {
        Next::TrapResult
    } else {
        Next::Check
    }
}

/// [[OwnPropertyKeys]], step 22 (I21): no key of the answer may stay unchecked; `unchecked`
/// counts them.
pub(crate) fn own_keys_rest(unchecked: usize) -> Result<(), Broken> {
    if unchecked == 0 {
        Ok(())
    } else {
        Err(Broken::KeyOfNonExtensibleExtra)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

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

    /// The checks of the seven traps of shape that compare no value: each answers, breaks the
    /// invariant its number names, or asks for the next fact only where its clause reads it.
    #[test]
    fn each_check_of_the_shape_traps_answers_as_its_step_says() {
        let configurable = Property::data(Value::Null, true, true, true);
        let fixed = Property::data(Value::Null, true, true, false);
        for value in [
            Value::Number(0.0),
            Value::Number(-0.0),
            Value::Number(f64::NAN),
        ] {
            assert_eq!(
                get_prototype_of_answer(value),
                Err(Broken::PrototypeNotAnObjectOrNull)
            );
            assert_eq!(
                get_own_property_answer(value),
                Err(Broken::DescriptorNotAnObjectOrUndefined)
            );
            assert_eq!(own_keys_element(value), Err(Broken::KeyNotAStringOrSymbol));
        }
        assert_eq!(get_prototype_of_answer(Value::Null), Ok(None));
        assert_eq!(
            get_prototype_of_answer(Value::Undefined),
            Err(Broken::PrototypeNotAnObjectOrNull)
        );
        assert_eq!(get_own_property_answer(Value::Undefined), Ok(None));
        assert_eq!(
            get_own_property_answer(Value::Null),
            Err(Broken::DescriptorNotAnObjectOrUndefined)
        );
        assert_eq!(get_prototype_of_after_extensible(true), Next::TrapResult);
        assert_eq!(
            get_prototype_of_after_extensible(false),
            Next::ReadPrototype
        );
        assert_eq!(get_prototype_of_after_prototype(None, None), Ok(()));
        assert_eq!(set_prototype_of_after_trap(false), Next::Answer(false));
        assert_eq!(set_prototype_of_after_trap(true), Next::ReadExtensible);
        assert_eq!(set_prototype_of_after_extensible(true), Next::Answer(true));
        assert_eq!(
            set_prototype_of_after_extensible(false),
            Next::ReadPrototype
        );
        assert_eq!(set_prototype_of_after_prototype(None, None), Ok(true));
        assert_eq!(is_extensible_after_target(true, true), Ok(true));
        assert_eq!(is_extensible_after_target(false, false), Ok(false));
        assert_eq!(
            is_extensible_after_target(true, false),
            Err(Broken::ExtensibleDiffers)
        );
        assert_eq!(
            is_extensible_after_target(false, true),
            Err(Broken::ExtensibleDiffers)
        );
        assert_eq!(prevent_extensions_after_trap(false), Next::Answer(false));
        assert_eq!(prevent_extensions_after_trap(true), Next::ReadExtensible);
        assert_eq!(prevent_extensions_after_extensible(false), Ok(true));
        assert_eq!(
            prevent_extensions_after_extensible(true),
            Err(Broken::PreventedButExtensible)
        );
        assert_eq!(
            get_own_property_undefined_after_descriptor(None),
            Ok(Next::TrapResult)
        );
        assert_eq!(
            get_own_property_undefined_after_descriptor(Some(fixed)),
            Err(Broken::HidesNonConfigurable)
        );
        assert_eq!(
            get_own_property_undefined_after_descriptor(Some(configurable)),
            Ok(Next::ReadExtensible)
        );
        assert_eq!(get_own_property_undefined_after_extensible(true), Ok(()));
        assert_eq!(
            get_own_property_undefined_after_extensible(false),
            Err(Broken::HidesOnNonExtensible)
        );
        assert_eq!(define_own_property_after_trap(false), Next::Answer(false));
        assert_eq!(define_own_property_after_trap(true), Next::ReadDescriptor);
        assert_eq!(own_keys_listed_before(false), Ok(()));
        assert_eq!(own_keys_listed_before(true), Err(Broken::KeyRepeated));
        assert_eq!(own_keys_after_target(true, false), Next::TrapResult);
        assert_eq!(own_keys_after_target(true, true), Next::Check);
        assert_eq!(own_keys_after_target(false, false), Next::Check);
        assert_eq!(own_keys_target_key(true, false), Ok(()));
        assert_eq!(
            own_keys_target_key(false, false),
            Err(Broken::NonConfigurableKeyMissing)
        );
        assert_eq!(
            own_keys_target_key(false, true),
            Err(Broken::KeyOfNonExtensibleMissing)
        );
        assert_eq!(own_keys_after_non_configurable(true), Next::TrapResult);
        assert_eq!(own_keys_after_non_configurable(false), Next::Check);
        assert_eq!(own_keys_rest(0), Ok(()));
        assert_eq!(own_keys_rest(1), Err(Broken::KeyOfNonExtensibleExtra));
        let generic = Descriptor {
            enumerable: Some(true),
            ..Descriptor::default()
        };
        assert_eq!(
            complete_property_descriptor(&generic),
            Property::data(Value::Undefined, false, true, false)
        );
        assert_eq!(
            descriptor_of(fixed),
            Descriptor::data(Value::Null, true, true, false)
        );
    }

    /// Each text of a check of the seven traps of shape starts with its number, `I1:` to
    /// `I21:`, in the order of the story.
    #[test]
    fn each_invariant_of_the_shape_traps_names_its_number() {
        let numbered = [
            Broken::PrototypeNotAnObjectOrNull,
            Broken::PrototypeOfNonExtensible,
            Broken::SetPrototypeOfNonExtensible,
            Broken::ExtensibleDiffers,
            Broken::PreventedButExtensible,
            Broken::DescriptorNotAnObjectOrUndefined,
            Broken::HidesNonConfigurable,
            Broken::HidesOnNonExtensible,
            Broken::DescriptorIncompatible,
            Broken::ReportsNonConfigurable,
            Broken::ReportsNonWritable,
            Broken::DefinesOnNonExtensible,
            Broken::DefinesNonConfigurableAbsent,
            Broken::DefinesIncompatible,
            Broken::DefinesNonConfigurable,
            Broken::DefinesNonWritable,
            Broken::KeyNotAStringOrSymbol,
            Broken::KeyRepeated,
            Broken::NonConfigurableKeyMissing,
            Broken::KeyOfNonExtensibleMissing,
            Broken::KeyOfNonExtensibleExtra,
        ];
        for (index, broken) in numbered.iter().enumerate() {
            let prefix = format!("I{}: ", index + 1);
            assert!(broken.text().starts_with(&prefix), "{broken:?}");
        }
    }
}
