//! Bound function exotic objects (ECMA-262 10.4.1, `#sec-bound-function-exotic-objects`):
//! [[BoundTargetFunction]], [[BoundThis]] and [[BoundArguments]] (Table 27), [[Call]]
//! (10.4.1.1), [[Construct]] (10.4.1.2) and BoundFunctionCreate (10.4.1.3,
//! `#sec-boundfunctioncreate`). `story:object-model-and-realms` wrote the dispatch arms of
//! `object/mod.rs`; `story:harness-builtins` wrote this file.
//!
//! A bound function's target may be a bound function, to any depth a script builds. [[Call]]
//! and [[Construct]] therefore walk the chain in a loop, a step per link, gathering the bound
//! arguments innermost first, and call the innermost target once: the observable result is the
//! spec's nested `Call(target, boundThis, args)` (each link adds no step of its own that code can
//! see), and the Rust stack does not grow with the chain.
//!
//! Kind site of the collector: [`BoundFunction::trace`] reports the target, the bound `this`
//! and every bound argument.

use super::{ObjectKind, ObjectRecord};
use crate::heap::collect::Mark;
use crate::host::Host;
use crate::host::Termination;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::value::{ObjectRef, Value};

/// The slots of a bound function (Table 27).
pub(crate) struct BoundFunction {
    /// [[BoundTargetFunction]]. Always `Some` for a bound function BoundFunctionCreate makes;
    /// `None` only in the fixture of the collector's tests, and a call that reaches `None` is a
    /// broken invariant that faults the agent.
    pub(crate) target: Option<ObjectRef>,
    /// [[BoundThis]].
    pub(crate) this: Value,
    /// [[BoundArguments]]. Its bytes are charged with the object (`heap/mod.rs`, owned bytes).
    pub(crate) arguments: Vec<Value>,
    /// Whether BoundFunctionCreate gave it a [[Construct]]: IsConstructor of the target.
    constructor: bool,
}

impl BoundFunction {
    /// Kind site of the collector: the target, the bound `this` and the bound arguments.
    pub(crate) fn trace(&self, mark: &mut Mark) {
        mark.count("object/bound_function.rs BoundFunction::trace");
        if let Some(target) = self.target {
            mark.object(target);
        }
        mark.value(self.this);
        for &argument in &self.arguments {
            mark.value(argument);
        }
    }

    /// A bound function exotic object has a [[Call]].
    pub(crate) fn has_call(&self) -> bool {
        true
    }

    /// Whether it has a [[Construct]] (BoundFunctionCreate, step 6).
    pub(crate) fn has_construct(&self) -> bool {
        self.constructor
    }

    /// A bound function with no target, for the tests of the collector.
    #[cfg(test)]
    pub(crate) fn for_test() -> BoundFunction {
        BoundFunction {
            target: None,
            this: Value::Undefined,
            arguments: Vec::new(),
            constructor: false,
        }
    }
}

/// BoundFunctionCreate (`#sec-boundfunctioncreate`): a bound function exotic object whose
/// [[Prototype]] is the target's, with a [[Construct]] when the target is a constructor. The
/// bound arguments are charged to the memory budget with the object.
pub(crate) fn bound_function_create<H: Host>(
    agent: &mut Agent<H>,
    target: ObjectRef,
    this: Value,
    arguments: &[Value],
) -> Completion<ObjectRef> {
    // 1. Let proto be ? targetFunc.[[GetPrototypeOf]]().
    let prototype = agent.get_prototype_of(target)?;
    // 6. If IsConstructor(targetFunc) is true, set obj.[[Construct]].
    let constructor = agent.has_construct(target)?;
    let owned = u64::try_from(size_of_val(arguments)).unwrap_or(u64::MAX);
    let mut kept = Vec::new();
    if kept.try_reserve_exact(arguments.len()).is_err() {
        return Err(agent.terminate(Termination::MemoryBudget));
    }
    kept.extend_from_slice(arguments);
    let record = ObjectRecord::new(
        prototype,
        ObjectKind::BoundFunction(BoundFunction {
            target: Some(target),
            this,
            arguments: kept,
            constructor,
        }),
    );
    match agent.heap.new_object(record, owned) {
        Ok(object) => Ok(object),
        Err(_) => Err(agent.terminate(Termination::MemoryBudget)),
    }
}

/// One link of a chain: its target, its bound `this`, and its bound arguments.
fn link<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
) -> Completion<(ObjectRef, Value, Vec<Value>)> {
    let found = match &agent.engine_object(function)?.kind {
        ObjectKind::BoundFunction(BoundFunction {
            target: Some(target),
            this,
            arguments,
            ..
        }) => Some((*target, *this, arguments.clone())),
        _ => None,
    };
    found.ok_or_else(|| agent.fault())
}

/// Whether `object` is a bound function exotic object.
fn is_bound<H: Host>(agent: &mut Agent<H>, object: ObjectRef) -> Completion<bool> {
    Ok(matches!(
        agent.engine_object(object)?.kind,
        ObjectKind::BoundFunction(_)
    ))
}

/// The walk of a chain from the bound function `function`: the innermost target, the
/// innermost bound `this`, the arguments "the list-concatenation of boundArgs and argList" of
/// every link, innermost first, and, for [[Construct]], the new target after each link's step
/// "If SameValue(F, newTarget) is true, set newTarget to target". A step per link and per bound
/// argument; no safe point is passed, so the values need no root until the call.
fn walk<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    args: &[Value],
    mut new_target: Option<ObjectRef>,
) -> Completion<(ObjectRef, Value, Vec<Value>, Option<ObjectRef>)> {
    let mut segments: Vec<Vec<Value>> = Vec::new();
    let mut current = function;
    let (target, this) = loop {
        let (target, this, arguments) = link(agent, current)?;
        let count = u64::try_from(arguments.len()).unwrap_or(u64::MAX);
        agent.charge_steps(count.saturating_add(1))?;
        segments.push(arguments);
        if new_target == Some(current) {
            new_target = Some(target);
        }
        if !is_bound(agent, target)? {
            break (target, this);
        }
        current = target;
    };
    // The gathered list lives until the call returns: the memory budget is asked before it is
    // made.
    let count = segments.iter().fold(args.len(), |count, segment| {
        count.saturating_add(segment.len())
    });
    let bytes = u64::try_from(count.saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
    let mut gathered = Vec::new();
    if agent.heap.accounting.check(bytes).is_err() || gathered.try_reserve_exact(count).is_err() {
        return Err(agent.terminate(Termination::MemoryBudget));
    }
    for segment in segments.iter().rev() {
        gathered.extend_from_slice(segment);
    }
    gathered.extend_from_slice(args);
    Ok((target, this, gathered, new_target))
}

/// [[Call]] (`#sec-bound-function-exotic-objects-call-thisargument-argumentslist`): "Return ?
/// Call(target, boundThis, args)", for the whole chain at once.
pub(crate) fn call<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    _this: Value,
    args: &[Value],
) -> Completion<Value> {
    let (target, this, args, _) = walk(agent, function, args, None)?;
    agent.internal_call(target, this, &args)
}

/// [[Construct]] (`#sec-bound-function-exotic-objects-construct-argumentslist-newtarget`):
/// "Return ? Construct(target, args, newTarget)", for the whole chain at once. Every link of a
/// chain that has a [[Construct]] has a target that is a constructor (BoundFunctionCreate,
/// step 6), so the innermost target is one.
pub(crate) fn construct<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    args: &[Value],
    new_target: ObjectRef,
) -> Completion<Value> {
    let (target, _, args, new_target) = walk(agent, function, args, Some(new_target))?;
    let Some(new_target) = new_target else {
        return Err(agent.fault());
    };
    agent.internal_construct(target, &args, new_target)
}
