//! The box of a class constructor (`docs/design/engine.md`, "Classes", item 3; decision 7 of
//! slice 8): the slots of an ECMAScript function object that only a class constructor has
//! (`#sec-ecmascript-function-objects`, Table 28): [[ConstructorKind]], and the initializer of
//! the instance elements, which stands for [[Fields]] and [[PrivateMethods]] together. A class
//! constructor is an `EcmascriptFunction` whose `class` holds one; [[IsClassConstructor]] is
//! that it does. Written by `story:class-definition-evaluation`, with every slot slice 8 needs:
//! `story:derived-classes-and-super-call` makes a box of kind [`ConstructorKind::Derived`], and
//! the stories of fields and private methods store an initializer.
//!
//! The box is charged with its function's record as owned bytes when F is made
//! (`Agent::instantiate_function`) and credited by the sweep that frees the record. The kind
//! site of the function, `EcmascriptFunction::trace`, traces the initializer.

use crate::value::ObjectRef;

/// [[ConstructorKind]] (`#sec-ecmascript-function-objects`): `base` for a class without a
/// heritage, `derived` for one with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructorKind {
    Base,
    Derived,
}

/// What a class constructor holds beside the slots of every ECMAScript function.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClassConstructor {
    /// [[ConstructorKind]].
    pub(crate) kind: ConstructorKind,
    /// The initializer of the instance elements ("Classes", item 4): a method whose home object
    /// is the prototype, which InitializeInstanceElements calls with `this` the new object;
    /// `None` when the class has no instance elements.
    pub(crate) instance_elements: Option<ObjectRef>,
}

/// The bytes a class constructor's record owns beside its slot: its [`ClassConstructor`] box.
pub(crate) fn class_constructor_bytes() -> u64 {
    u64::try_from(size_of::<ClassConstructor>()).unwrap_or(u64::MAX)
}
