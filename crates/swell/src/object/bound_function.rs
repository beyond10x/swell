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
use alloc::vec::Vec;

/// The slots of a bound function (Table 27).
pub(crate) struct BoundFunction {
    /// [[BoundTargetFunction]]. Always `Some` for a bound function BoundFunctionCreate makes;
    /// `None` only in the fixture of the collector's tests, and a call that reaches `None` is a
    /// broken invariant that faults the agent.
    pub(crate) target: Option<ObjectRef>,
    /// [[BoundThis]].
    pub(crate) this: Value,
    /// [[BoundArguments]]. Its bytes are the object's owned bytes (`heap/mod.rs`), charged
    /// before they are copied.
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
/// bound arguments are charged to the memory budget before they are copied, and the object owns
/// them.
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
    // The bound arguments are charged before they are copied; the record owns them once it is
    // stored, and the sweep that frees it credits them.
    let owned = u64::try_from(size_of_val(arguments)).unwrap_or(u64::MAX);
    agent.charge_bytes(owned)?;
    let mut kept = Vec::new();
    #[expect(
        clippy::disallowed_methods,
        reason = "charged by `charge_bytes` before it is reserved; the record owns it and its sweep credits it"
    )]
    if kept.try_reserve_exact(arguments.len()).is_err() {
        agent.heap.accounting.credit(owned);
        return Err(agent.terminate(Termination::MemoryBudget));
    }
    kept.extend_from_slice(arguments);
    #[cfg(test)]
    KEPT_COPIED.with(|copied| {
        let count = u64::try_from(arguments.len()).unwrap_or(u64::MAX);
        copied.set(copied.get().saturating_add(count));
    });
    let record = ObjectRecord::new(
        prototype,
        ObjectKind::BoundFunction(BoundFunction {
            target: Some(target),
            this,
            arguments: kept,
            constructor,
        }),
    );
    match agent.heap.new_object_charged(record, owned) {
        Ok(object) => Ok(object),
        Err(_) => {
            agent.heap.accounting.credit(owned);
            Err(agent.terminate(Termination::MemoryBudget))
        }
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

/// One link of a chain: its target and its bound `this`, read from its record without a copy.
fn link<H: Host>(agent: &mut Agent<H>, function: ObjectRef) -> Completion<(ObjectRef, Value)> {
    let found = match &agent.engine_object(function)?.kind {
        ObjectKind::BoundFunction(BoundFunction {
            target: Some(target),
            this,
            ..
        }) => Some((*target, *this)),
        _ => None,
    };
    found.ok_or_else(|| agent.fault())
}

/// The bound arguments of the link `function` copied into `gathered` so that they end at
/// position `end`: answers the link's target and the position they start at. A link whose
/// arguments do not fit before `end` is a broken invariant that faults the agent.
fn copy_link_arguments<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    gathered: &mut [Value],
    end: usize,
) -> Completion<(ObjectRef, usize)> {
    let copied = match &agent.engine_object(function)?.kind {
        ObjectKind::BoundFunction(BoundFunction {
            target: Some(target),
            arguments,
            ..
        }) => end.checked_sub(arguments.len()).and_then(|start| {
            let into = gathered.get_mut(start..end)?;
            #[cfg(test)]
            ARGUMENTS_CLONED.with(|cloned| {
                let count = u64::try_from(arguments.len()).unwrap_or(u64::MAX);
                cloned.set(cloned.get().saturating_add(count));
            });
            for (slot, &argument) in into.iter_mut().zip(arguments) {
                *slot = argument;
            }
            Some((*target, start))
        }),
        _ => None,
    };
    copied.ok_or_else(|| agent.fault())
}

/// Whether `object` is a bound function exotic object.
fn is_bound<H: Host>(agent: &mut Agent<H>, object: ObjectRef) -> Completion<bool> {
    Ok(matches!(
        agent.engine_object(object)?.kind,
        ObjectKind::BoundFunction(_)
    ))
}

/// What the innermost target of a chain is asked for: [[Call]], or [[Construct]] with the new
/// target, which each link's step "If SameValue(F, newTarget) is true, set newTarget to target"
/// may replace.
#[derive(Clone, Copy)]
enum Invocation {
    Call,
    Construct(Option<ObjectRef>),
}

/// The walk of a chain from the bound function `function` and the call of its innermost
/// target: the innermost bound `this`, the arguments "the list-concatenation of boundArgs and
/// argList" of every link, innermost first, and, for [[Construct]], the new target after each
/// link's step. A step per link and per bound argument, the count read from the link's record
/// and charged before anything is copied; no safe point is passed, so the values need no root
/// until the call.
///
/// The chain is walked twice and each link's arguments are read in place: the first walk, here,
/// charges the steps and counts the arguments, the gathered list's bytes are charged to the
/// memory budget, and [`call_gathered`] reserves it, copies each link's arguments into it in a
/// second walk, from its end, so that the innermost link's come first, and calls the target.
/// No copy of a link's arguments is made beside it. The list's bytes stay charged while the
/// target runs and are credited when [`call_gathered`] returns, on every way out; a refused
/// charge credits nothing.
fn walk<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    args: &[Value],
    mut invocation: Invocation,
) -> Completion<Value> {
    let mut count = args.len();
    let mut current = function;
    let (target, this) = loop {
        let bound = bound_count(agent, current)?;
        agent.charge_steps(bound.saturating_add(1))?;
        let (target, this) = link(agent, current)?;
        count = count.saturating_add(usize::try_from(bound).unwrap_or(usize::MAX));
        if let Invocation::Construct(new_target) = &mut invocation
            && *new_target == Some(current)
        {
            *new_target = Some(target);
        }
        if !is_bound(agent, target)? {
            break (target, this);
        }
        current = target;
    };
    // The gathered list lives until the call returns: its bytes are charged before it is made
    // and credited here, once, whatever the completion of the rest.
    let bytes = u64::try_from(count.saturating_mul(size_of::<Value>())).unwrap_or(u64::MAX);
    agent.charge_bytes(bytes)?;
    let answer = call_gathered(agent, function, args, (target, this), count, invocation);
    agent.heap.accounting.credit(bytes);
    answer
}

/// The rest of [`walk`]: the gathered list of `count` values reserved, `args` and each link's
/// arguments copied into it, and the call of the innermost `target` with it.
fn call_gathered<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    args: &[Value],
    (target, this): (ObjectRef, Value),
    count: usize,
    invocation: Invocation,
) -> Completion<Value> {
    let mut gathered = Vec::new();
    #[expect(
        clippy::disallowed_methods,
        reason = "charged by `walk` before it is reserved, a `Value` per gathered argument"
    )]
    if gathered.try_reserve_exact(count).is_err() {
        return Err(agent.terminate(Termination::MemoryBudget));
    }
    gathered.resize(count, Value::Undefined);
    let mut end = count.saturating_sub(args.len());
    let Some(tail) = gathered.get_mut(end..) else {
        return Err(agent.fault());
    };
    for (slot, &argument) in tail.iter_mut().zip(args) {
        *slot = argument;
    }
    let mut current = function;
    loop {
        let (next, start) = copy_link_arguments(agent, current, &mut gathered, end)?;
        end = start;
        if next == target {
            break;
        }
        current = next;
    }
    if end != 0 {
        return Err(agent.fault());
    }
    match invocation {
        Invocation::Call => {
            #[cfg(test)]
            GATHERED_READING.with(|reading| reading.set(agent.heap.accounting.used()));
            agent.internal_call(target, this, &gathered)
        }
        Invocation::Construct(Some(new_target)) => {
            agent.internal_construct(target, &gathered, new_target)
        }
        Invocation::Construct(None) => Err(agent.fault()),
    }
}

/// [[Call]] (`#sec-bound-function-exotic-objects-call-thisargument-argumentslist`): "Return ?
/// Call(target, boundThis, args)", for the whole chain at once.
pub(crate) fn call<H: Host>(
    agent: &mut Agent<H>,
    function: ObjectRef,
    _this: Value,
    args: &[Value],
) -> Completion<Value> {
    walk(agent, function, args, Invocation::Call)
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
    walk(
        agent,
        function,
        args,
        Invocation::Construct(Some(new_target)),
    )
}

#[cfg(test)]
std::thread_local! {
    /// The bound arguments the walks copied from their links, in the counting build of the
    /// tests; a test resets it before it reads it.
    static ARGUMENTS_CLONED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// The values `bound_function_create` copied into a bound function's [[BoundArguments]], in
    /// the counting build of the tests; a test resets it before it reads it.
    static KEPT_COPIED: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
    /// `accounting.used()` read just before [`call_gathered`] calls `internal_call`, in the
    /// counting build of the tests; a test resets it before it reads it.
    static GATHERED_READING: core::cell::Cell<u64> = const { core::cell::Cell::new(0) };
}

#[cfg(test)]
mod tests {
    use super::{
        ARGUMENTS_CLONED, GATHERED_READING, Invocation, KEPT_COPIED, bound_function_create, call,
        walk,
    };
    use crate::host::agent::Agent;
    use crate::host::hooks::test_host::{NoHooks, agent_with_realm_and, enter, limits};
    use crate::host::outcome::Abrupt;
    use crate::object::property::Key;
    use crate::realm::intrinsics::Intrinsic;
    use crate::value::{ObjectRef, Root, Value};
    use crate::{Outcome, Realm, Termination};
    use alloc::vec::Vec;

    /// The bytes of `n` values in a list.
    fn list_bytes(n: u32) -> u64 {
        u64::from(n) * size_of::<Value>() as u64
    }

    /// The bound function of `root`.
    fn object(agent: &Agent<NoHooks>, root: &Root) -> ObjectRef {
        match agent.heap.roots.get(root) {
            Ok(Value::Object(object)) => object,
            other => panic!("the bound function: {other:?}"),
        }
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated),
    /// and BoundFunctionCreate (`#sec-boundfunctioncreate`), step 9 (obj.[[BoundArguments]]):
    /// called directly to bind `Function.prototype` with 100,000 Numbers under a budget of the
    /// reading after a collection plus half of their bytes, it ends in
    /// `Terminated(MemoryBudget)` before a value is copied into the bound function's list. With
    /// the budget raised it answers a bound function that holds each of them, and after a
    /// collection the reading is the one before. A bound function made and dropped before the
    /// reading leaves a free slot in the arena for the one made after it.
    #[test]
    fn bound_arguments_are_charged_before_they_are_kept() {
        const N: u32 = 100_000;
        let mut limits = limits();
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        let roomy_limit = agent.heap.bytes() + (1 << 28);
        agent.heap.accounting.set_limit(roomy_limit);
        enter(&mut agent, &realm);
        let realm_handle = agent.heap.roots.get_realm(&realm.0).unwrap();
        let target = agent
            .intrinsic(realm_handle, Intrinsic::FunctionPrototype)
            .unwrap();
        assert!(bound_function_create(&mut agent, target, Value::Undefined, &[]).is_ok());
        assert!(agent.collect().is_ok());
        let before = agent.heap_bytes().unwrap();
        let arguments: Vec<Value> = (0..N).map(|i| Value::Number(f64::from(i))).collect();
        agent.heap.accounting.set_limit(before + list_bytes(N) / 2);
        KEPT_COPIED.with(|copied| copied.set(0));
        let answer = bound_function_create(&mut agent, target, Value::Undefined, &arguments);
        assert!(
            matches!(answer, Err(Abrupt::Terminated(Termination::MemoryBudget))),
            "{answer:?}"
        );
        assert_eq!(
            KEPT_COPIED.with(core::cell::Cell::get),
            0,
            "bound arguments copied before the memory budget ended the run"
        );
        agent.termination = None;
        agent.heap.accounting.set_limit(roomy_limit);
        let answer = bound_function_create(&mut agent, target, Value::Undefined, &arguments);
        assert!(answer.is_ok(), "{answer:?}");
        assert_eq!(KEPT_COPIED.with(core::cell::Cell::get), u64::from(N));
        assert!(agent.collect().is_ok());
        assert_eq!(
            agent.heap_bytes().unwrap(),
            before,
            "after the bound function"
        );
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated and
    /// credited where its holder drops it), and [[Call]] of a bound function: the function of
    /// [`bound_with`] with 100,000 bound arguments, called under a roomy budget after a
    /// collection. While the target runs, the charged reading holds the gathered list's bytes on
    /// top of the reading before the call; after the call it is the reading before.
    #[test]
    fn the_gathered_list_is_charged_while_the_call_runs() {
        const N: u32 = 100_000;
        let (mut agent, realm, bound, _) = bound_with(N);
        let bound = object(&agent, &bound);
        assert!(agent.collect().is_ok());
        enter(&mut agent, &realm);
        agent.steps = 0;
        let before = agent.heap.accounting.used();
        GATHERED_READING.with(|reading| reading.set(0));
        let answer = call(&mut agent, bound, Value::Undefined, &[]);
        assert!(matches!(answer, Ok(Value::Undefined)), "{answer:?}");
        let reading = GATHERED_READING.with(core::cell::Cell::get);
        assert!(
            reading >= before + list_bytes(N),
            "{reading} while the target runs, {before} before the call"
        );
        assert_eq!(agent.heap.accounting.used(), before, "after the call");
    }

    /// `docs/design/engine.md`, "Memory budget" (a charged structure is credited on every way
    /// out), and [[Construct]] of a bound function: [`walk`] entered for a construction with no
    /// new target, a broken invariant that only this test reaches, gathers the 100,000 bound
    /// arguments of the function of [`bound_with`] and ends in `Terminated(EngineFault)`; the
    /// charged reading is the one before.
    #[test]
    fn a_construction_that_faults_credits_the_gathered_list() {
        const N: u32 = 100_000;
        let (mut agent, realm, bound, _) = bound_with(N);
        let bound = object(&agent, &bound);
        assert!(agent.collect().is_ok());
        enter(&mut agent, &realm);
        agent.steps = 0;
        let before = agent.heap.accounting.used();
        let answer = walk(&mut agent, bound, &[], Invocation::Construct(None));
        assert!(
            matches!(answer, Err(Abrupt::Terminated(Termination::EngineFault))),
            "{answer:?}"
        );
        assert_eq!(agent.heap.accounting.used(), before, "after the fault");
    }

    /// An agent with a budget of what its realm holds plus 256 MiB, `Function.prototype` bound
    /// with `n` arguments through `Function.prototype.bind.apply`, and a root of `undefined`.
    fn bound_with(n: u32) -> (Agent<NoHooks>, Realm, Root, Root) {
        let mut limits = limits();
        limits.memory_bytes = 1 << 30;
        let (mut agent, realm) = agent_with_realm_and(limits);
        let held = agent.heap.bytes();
        agent.heap.accounting.set_limit(held + (1 << 28));
        enter(&mut agent, &realm);
        // The arguments of apply: the bound `this`, then n bound arguments.
        let list = agent.array_create(0.0, None).unwrap();
        for i in 0..=n {
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
        (agent, realm, bound, undefined)
    }

    /// `docs/design/engine.md`, "Memory budget" (work is charged before it is done), and
    /// [[Call]] of a bound function (`#sec-bound-function-exotic-objects-call-thisargument-argumentslist`):
    /// a function bound with 100,000 arguments through `Function.prototype.bind.apply`, called
    /// with 10 steps for its entry. The call ends in `Terminated(StepBudget)` before a bound
    /// argument is copied, since the link's arguments are charged first.
    #[test]
    fn bound_arguments_are_charged_before_they_are_copied() {
        let (mut agent, realm, bound, undefined) = bound_with(100_000);
        agent.limits.steps_per_entry = 10;
        ARGUMENTS_CLONED.with(|cloned| cloned.set(0));
        let answer = agent.call(&realm, &bound, &undefined, &[]);
        assert!(
            matches!(answer, Ok(Outcome::Terminated(Termination::StepBudget))),
            "{answer:?}"
        );
        assert_eq!(
            ARGUMENTS_CLONED.with(core::cell::Cell::get),
            0,
            "bound arguments copied before the step budget ended the run"
        );
    }

    /// `docs/design/engine.md`, "Memory budget" (storage is charged before it is allocated),
    /// and [[Call]] of a bound function: the function of [`bound_with`] with 100,000 bound
    /// arguments, called under a budget of the reading after the setup plus half of the
    /// arguments' bytes. The call ends in `Terminated(MemoryBudget)` before a bound argument is
    /// copied; the next call answers normally and copies each bound argument once, and after a
    /// collection the reading is the one before.
    #[test]
    fn bound_arguments_are_not_copied_past_the_memory_budget() {
        const N: u32 = 100_000;
        let (mut agent, realm, bound, undefined) = bound_with(N);
        let arguments = u64::from(N) * size_of::<Value>() as u64;
        let roomy_limit = agent.heap.bytes() + (1 << 28);
        assert!(agent.collect().is_ok());
        let before = agent.heap_bytes().unwrap();
        agent.heap.accounting.set_limit(before + arguments / 2);
        ARGUMENTS_CLONED.with(|cloned| cloned.set(0));
        let answer = agent.call(&realm, &bound, &undefined, &[]);
        assert!(
            matches!(answer, Ok(Outcome::Terminated(Termination::MemoryBudget))),
            "{answer:?}"
        );
        assert_eq!(
            ARGUMENTS_CLONED.with(core::cell::Cell::get),
            0,
            "bound arguments copied before the memory budget ended the run"
        );
        agent.heap.accounting.set_limit(roomy_limit);
        let answer = agent.call(&realm, &bound, &undefined, &[]);
        assert!(matches!(answer, Ok(Outcome::Normal(_))), "{answer:?}");
        drop(answer);
        assert_eq!(ARGUMENTS_CLONED.with(core::cell::Cell::get), u64::from(N));
        assert!(agent.collect().is_ok());
        assert_eq!(agent.heap_bytes().unwrap(), before, "after the next call");
    }
}
