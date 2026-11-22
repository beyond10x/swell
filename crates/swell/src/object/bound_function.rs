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

/// The number of bound arguments of the link `function`, read from its record without a copy.
fn bound_count<H: Host>(agent: &mut Agent<H>, function: ObjectRef) -> Completion<u64> {
    let count = match &agent.engine_object(function)?.kind {
        ObjectKind::BoundFunction(BoundFunction {
            target: Some(_),
            arguments,
            ..
        }) => Some(arguments.len()),
        _ => None,
    };
    match count {
        Some(count) => Ok(u64::try_from(count).unwrap_or(u64::MAX)),
        None => Err(agent.fault()),
    }
}

/// One link of a chain: its target, its bound `this`, and a copy of its bound arguments.
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
        }) => {
            #[cfg(test)]
            ARGUMENTS_CLONED.with(|cloned| {
                let count = u64::try_from(arguments.len()).unwrap_or(u64::MAX);
                cloned.set(cloned.get().saturating_add(count));
            });
            Some((*target, *this, arguments.clone()))
        }
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
/// argument, the count read from the link's record and charged before its arguments are
/// copied; no safe point is passed, so the values need no root until the call.
fn walk<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    args: &[Value],
    mut new_target: Option<ObjectRef>,
) -> Completion<(ObjectRef, Value, Vec<Value>, Option<ObjectRef>)> {
    let mut segments: Vec<Vec<Value>> = Vec::new();
    let mut current = function;
    let (target, this) = loop {
        let count = bound_count(agent, current)?;
        agent.charge_steps(count.saturating_add(1))?;
        let (target, this, arguments) = link(agent, current)?;
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

#[cfg(test)]
thread_local! {
    /// The bound arguments the walks copied from their links, in the counting build of the
    /// tests; a test resets it before it reads it.
    static ARGUMENTS_CLONED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::ARGUMENTS_CLONED;
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::object::property::Key;
    use crate::value::{ObjectRef, Value};
    use crate::{Outcome, Termination};

    /// `docs/design/engine.md`, "Memory budget" (work is charged before it is done), and
    /// [[Call]] of a bound function (`#sec-bound-function-exotic-objects-call-thisargument-argumentslist`):
    /// a function bound with 100,000 arguments through `Function.prototype.bind.apply`, called
    /// with 10 steps for its entry. The call ends in `Terminated(StepBudget)` before a bound
    /// argument is copied, since the link's arguments are charged first.
    #[test]
    fn bound_arguments_are_charged_before_they_are_copied() {
        const N: u32 = 100_000;
        let mut limits = limits();
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        let held = agent.heap.bytes();
        agent.heap.accounting.set_limit(held + (1 << 28));
        enter(&mut agent, &realm);
        // The arguments of apply: the bound `this`, then N bound arguments.
        let list = agent.array_create(0.0, None).unwrap();
        for i in 0..=N {
            assert_eq!(
                agent.create_data_property(list, Key::Index(i), Value::Number(f64::from(i))),
                Ok(true)
            );
        }
        let global = agent.global_object(&realm).unwrap();
        let Ok(Value::Object(global)) = agent.heap.roots.get(&global) else {
            panic!("the global object");
        };
        fn property(agent: &mut Agent<NoHooks>, object: ObjectRef, name: &str) -> ObjectRef {
            let key = agent.name_key(name).unwrap();
            match agent.get(object, key) {
                Ok(Value::Object(found)) => found,
                other => panic!("{name}: {other:?}"),
            }
        }
        let function = property(&mut agent, global, "Function");
        let prototype = property(&mut agent, function, "prototype");
        let bind = property(&mut agent, prototype, "bind");
        let apply = property(&mut agent, prototype, "apply");
        let [prototype, bind, apply, list, undefined] = [
            Value::Object(prototype),
            Value::Object(bind),
            Value::Object(apply),
            Value::Object(list),
            Value::Undefined,
        ]
        .map(|value| agent.heap.roots.add(value));
        // Function.prototype.bind.apply(Function.prototype, list)
        let bound = match agent.call(&realm, &apply, &bind, &[&prototype, &list]) {
            Ok(Outcome::Normal(bound)) => bound,
            other => panic!("bind.apply answered {other:?}"),
        };
        agent.limits.steps_per_entry = 10;
        ARGUMENTS_CLONED.with(|cloned| cloned.set(0));
        let answer = agent.call(&realm, &bound, &undefined, &[]);
        assert!(
            matches!(answer, Ok(Outcome::Terminated(Termination::StepBudget))),
            "{answer:?}"
        );
        assert_eq!(
            ARGUMENTS_CLONED.with(std::cell::Cell::get),
            0,
            "bound arguments copied before the step budget ended the run"
        );
    }
}
