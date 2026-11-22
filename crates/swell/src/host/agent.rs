//! The agent (`docs/design/host-interface.md`, decisions 1, 2, 4, 6, 7, 12, 13, 14 and 18;
//! ECMA-262 9.6, `#sec-agents`): one engine instance with its heap, its host value, its limits
//! and seeds, its interrupt handle and its state; how it is created, the reads that run no code,
//! call no hook and spend no step, and the entry points of `story:object-model-and-realms`:
//! `create_realm`, `global_object`, `realm_host_defined`, `call` and `construct`, and of
//! `story:interpreter-and-script-entry-points` (decision 8): `parse_script`, `evaluate_script`,
//! `script_host_defined` and `last_entry_steps`. Written by `story:heap-values-and-host-surface`,
//! `story:object-model-and-realms` and `story:interpreter-and-script-entry-points`.
//!
//! An agent is created, run and dropped on one thread and is not `Send` (decision 1): its roots
//! share an `Rc`'d release queue. Every entry point takes `&mut self`, so code running inside
//! the agent cannot call it; inside, the host reaches the engine through `Cx` or `HookCx`.
//! After a run ends in `Terminated(EngineFault)` the agent is faulted for good and answers every
//! call but `state()` with `Err(HostError::Faulted)` (decision 6).
//!
//! The entry bookkeeping (decisions 2 and 12): a call made while no builtin runs is an outermost
//! entry, which starts its step counter at 0 and checks the interrupt flag; [`Agent::charge_steps`]
//! is the one function that charges steps and checks the counter and the flag; each call into a
//! function charges one and counts against `reentry_depth` (`object/builtin_function.rs`).
//!
//! The test-only switch of the feature `fault-injection` ([`FaultPoint`], HOST-71) makes
//! `parse_script`, `create_realm` or `collect` find a broken invariant; written by
//! `story:host-interface-hardening`.
//!
//! Root site of the collector: [`trace_roots`] reports the values handed to host code in the
//! running `with_cx`, host function and hook scopes, and the scripts handed out as `ScriptRef`s.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

use super::hooks::HostDefined;
use super::host_bytes::HostBytes;
use super::interrupt::InterruptHandle;
use super::limits::{CreateError, Limits, Seeds};
use super::outcome::{
    Abrupt, CollectReport, Completion, HostError, Outcome, Parsed, Termination, ValueKind,
};
use super::{ErrorKind, GlobalSetup, Host, Source};
use crate::heap::Heap;
use crate::heap::arena::Handle;
use crate::heap::collect::{self, Item, Mark, Stale};
use crate::interp::Interp;
use crate::object::builtin_function::Activation;
use crate::realm::WellKnown;
use crate::script::{ParseAnswer, parse_termination};
use crate::value::{Realm, Root, Script, Value};

/// How many agents this process has created; the next agent's key is one more. A key is never
/// 0, and never used twice in a process.
static AGENTS_CREATED: AtomicU32 = AtomicU32::new(0);

/// The key of a new agent, or `AgentKeysExhausted` after 2^32 - 1 agents.
fn next_key() -> Result<u32, CreateError> {
    let created = AGENTS_CREATED
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |created| {
            created.checked_add(1)
        })
        .map_err(|_| CreateError::AgentKeysExhausted)?;
    created
        .checked_add(1)
        .ok_or(CreateError::AgentKeysExhausted)
}

/// Whether an agent answers calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AgentState {
    Alive,
    /// A run ended in `Terminated(EngineFault)`; the host discards the agent.
    Faulted,
}

/// One engine instance (9.6).
pub struct Agent<H: Host> {
    pub(crate) host: H,
    pub(crate) limits: Limits,
    #[expect(dead_code, reason = "Math.random: slice 4")]
    pub(crate) random_seed: [u8; 32],
    pub(crate) state: AgentState,
    pub(crate) heap: Heap,
    pub(crate) interrupt: InterruptHandle,
    /// The values handed to host code in the running scopes, outermost first.
    pub(crate) scope: Vec<Value>,
    /// The termination of the running entry: once set, every `Cx` call answers it (decision 10,
    /// "Termination is sticky for the run").
    pub(crate) termination: Option<Termination>,
    /// The activations of the running builtins, outermost first (`object/builtin_function.rs`).
    pub(crate) activations: Vec<Activation>,
    /// The values running builtins keep across calls into code.
    pub(crate) kept: Vec<Value>,
    /// The steps the running outermost entry has spent.
    pub(crate) steps: u64,
    /// The bytes of property maps freed by the last sweep, to credit (`object/mod.rs`).
    pub(crate) credit: Rc<Cell<u64>>,
    /// The bytes of the host's charges whose `HostBytes` were dropped, to credit after the
    /// next sweep or before the next charge of host bytes (`host_bytes.rs`).
    pub(crate) host_credit: Rc<Cell<u64>>,
    /// The realm the running entry names (decision 7), the current realm while no builtin runs
    /// (`realm/mod.rs`); `None` between entries.
    pub(crate) entry_realm: Option<Handle>,
    /// The well-known symbols, held weakly (`realm/mod.rs`).
    pub(crate) symbols: Option<WellKnown>,
    /// The frame stack and the running loop (`interp/mod.rs`).
    pub(crate) interp: Interp,
    /// The script records `Cx::parse_script` handed out as `ScriptRef`s, held until the
    /// outermost entry ends.
    pub(crate) scoped_scripts: Vec<Handle>,
    /// A run ended in `Terminated(MemoryBudget)`: the next safe point collects, so that what
    /// that run left unreachable does not hold the budget the next entry needs (HOST-70).
    pub(crate) collect_pending: bool,
    /// The refusals of a later slice's construct the agent raised ([`Agent::refusals`]).
    pub(crate) refusals: u64,
    /// The call the test-only switch makes find a broken invariant next ([`FaultPoint`]).
    #[cfg(feature = "fault-injection")]
    pub(crate) injected: Option<FaultPoint>,
}

/// The calls a test build with the feature `fault-injection` can make find a broken engine
/// invariant (HOST-71; `story:ess-conformance-target` uses it for the `engine-fault` outcome of
/// `Collect`). The feature is off by default; no build of the engine for a host has it.
#[cfg(feature = "fault-injection")]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaultPoint {
    ParseScript,
    CreateRealm,
    Collect,
}

#[cfg(feature = "fault-injection")]
impl<H: Host> Agent<H> {
    /// Test only: the next call of `point` finds a broken engine invariant after it did its
    /// work, and answers `Terminated(EngineFault)` as such a finding does (decision 10). The
    /// switch fires once; another call of `inject_fault` replaces it.
    pub fn inject_fault(&mut self, point: FaultPoint) {
        self.injected = Some(point);
    }

    /// Whether the switch is set at `point`; it is cleared when it fires.
    fn injected(&mut self, point: FaultPoint) -> bool {
        if self.injected == Some(point) {
            self.injected = None;
            return true;
        }
        false
    }
}

impl<H: Host> Agent<H> {
    /// A new agent with its host value, limits and seeds. It holds no realm and its heap is
    /// empty. This is the only function of the interface that takes `Seeds` (HOST-146).
    pub fn new(host: H, limits: Limits, seeds: Seeds) -> Result<Agent<H>, CreateError> {
        let key = next_key()?;
        Ok(Agent {
            host,
            heap: Heap::new(key, limits.memory_bytes, &seeds.hash_key),
            limits,
            random_seed: seeds.random_seed,
            state: AgentState::Alive,
            interrupt: InterruptHandle::default(),
            scope: Vec::new(),
            termination: None,
            activations: Vec::new(),
            kept: Vec::new(),
            steps: 0,
            credit: Rc::new(Cell::new(0)),
            host_credit: Rc::new(Cell::new(0)),
            entry_realm: None,
            symbols: None,
            interp: Interp::default(),
            scoped_scripts: Vec::new(),
            collect_pending: false,
            refusals: 0,
            #[cfg(feature = "fault-injection")]
            injected: None,
        })
    }

    /// How many refusals of a later slice's construct the agent raised (`host-interface.md`,
    /// section 13, "A count of refusals"): every `SyntaxError` whose message
    /// `swell_syntax::message_kind` reads as a refusal, raised by a parse through
    /// `parse_script`, `eval` or `Function` or by an instruction (a regular expression literal
    /// before slice 5), caught by the script or not. The nesting limit and every other
    /// `SyntaxError` are not counted. It starts at 0, is never reset, saturates, and is read in
    /// every state of the agent, `Faulted` included. A read: it runs no code, calls no hook and
    /// spends no step.
    pub fn refusals(&self) -> u64 {
        self.refusals
    }

    /// Count the `SyntaxError` of `message` if it is a refusal.
    pub(crate) fn count_refusal(&mut self, message: &str) {
        if matches!(
            swell_syntax::message_kind(message),
            swell_syntax::MessageKind::Refusal { .. }
        ) {
            self.refusals = self.refusals.saturating_add(1);
        }
    }

    /// The host value.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// The host value, to change.
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    /// Whether the agent answers calls. The one read a faulted agent answers.
    pub fn state(&self) -> AgentState {
        self.state
    }

    /// The bytes the agent's heap is charged now, the reading `memory_bytes` bounds (decisions 2,
    /// "Two readings of the heap", and 13): the arenas' capacity with the collector's workspace
    /// and the atom table's, which stay charged while the agent lives, what the records own, and
    /// what a running step holds charged.
    pub fn heap_bytes(&self) -> Result<u64, HostError> {
        self.alive()?;
        Ok(self.heap.bytes())
    }

    /// What the records the heap holds own, each one's slot included, and what the atom table's
    /// entries own; free slots, the collector's workspace and empty buckets are left out
    /// (decision 2, "Two readings of the heap"). After a collection these are the live records.
    /// It is at most `heap_bytes()`, falls by everything a sweep frees, and sets the collection
    /// threshold.
    pub fn live_bytes(&self) -> Result<u64, HostError> {
        self.alive()?;
        Ok(self.heap.live_bytes())
    }

    /// A full collection now (decision 18). It runs no code and calls no hook. A stale handle
    /// reported by a trace answers `Terminated(EngineFault)` and faults the agent.
    pub fn collect(&mut self) -> Result<Outcome<CollectReport>, HostError> {
        self.alive()?;
        let found = self.collect_now();
        #[cfg(feature = "fault-injection")]
        let found = if self.injected(FaultPoint::Collect) {
            Err(Stale)
        } else {
            found
        };
        match found {
            Ok(report) => Ok(Outcome::Normal(report)),
            Err(Stale) => {
                self.fault();
                Ok(Outcome::Terminated(Termination::EngineFault))
            }
        }
    }

    /// Give back the bytes of `charge` at once and answer them (section 14, "The host's own work
    /// and bytes"): the handle is emptied before it drops, so its drop adds nothing and the
    /// bytes are credited once. It needs no realm and no scope, runs no code, calls no hook and
    /// spends no step. A handle of another agent answers `Err(ForeignHandle)` and credits
    /// nothing here; dropped in the call, it gives its bytes back to its own agent.
    pub fn credit_host_bytes(&mut self, charge: HostBytes) -> Result<u64, HostError> {
        self.alive()?;
        if charge.key != self.heap.key {
            return Err(HostError::ForeignHandle);
        }
        let mut charge = charge;
        let bytes = std::mem::take(&mut charge.bytes);
        self.heap.accounting.credit(bytes);
        Ok(bytes)
    }

    /// The agent's interrupt handle; every call answers a clone sharing one flag (decision 12).
    pub fn interrupt_handle(&self) -> InterruptHandle {
        self.interrupt.clone()
    }

    /// A second root of what `root` holds.
    pub fn duplicate(&mut self, root: &Root) -> Result<Root, HostError> {
        self.alive()?;
        let value = self.heap.roots.get(root)?;
        Ok(self.heap.roots.add(value))
    }

    /// A new realm (decision 7): InitializeHostDefinedRealm (`#sec-initializehostdefinedrealm`)
    /// with the intrinsics, then `setup`, which may build the global object and the global
    /// `this` value, then the properties of clause 19. `host_defined` is the realm's
    /// [[HostDefined]]. Over the memory budget it answers `Terminated(MemoryBudget)`; a throw of
    /// `setup` or of SetDefaultGlobalBindings answers `Throw`; a stale or foreign handle in what
    /// `setup` returns faults the agent (decision 6). It is a safe point.
    pub fn create_realm(
        &mut self,
        setup: Option<GlobalSetup<'_, H>>,
        host_defined: Option<HostDefined>,
    ) -> Result<Outcome<Realm>, HostError> {
        self.alive()?;
        self.termination = None;
        self.steps = 0;
        if self.safe_point().is_err() {
            self.fault();
            return Ok(Outcome::Terminated(Termination::EngineFault));
        }
        let start = self.scope.len();
        let outcome = self.initialize_host_defined_realm(setup, host_defined);
        #[cfg(feature = "fault-injection")]
        let outcome = if self.injected(FaultPoint::CreateRealm) {
            self.fault();
            Outcome::Terminated(Termination::EngineFault)
        } else {
            outcome
        };
        self.scope.truncate(start);
        self.end_entry();
        Ok(outcome)
    }

    /// A root of the global object of `realm`.
    pub fn global_object(&mut self, realm: &Realm) -> Result<Root, HostError> {
        self.alive()?;
        let handle = self.live_realm(realm)?;
        let global = self
            .heap
            .realms
            .get(handle)
            .map_err(|_| HostError::StaleHandle)?
            .global
            .ok_or(HostError::StaleHandle)?;
        Ok(self.heap.roots.add(Value::Object(global)))
    }

    /// The [[HostDefined]] of `realm`: the `Rc` given to `create_realm`.
    pub fn realm_host_defined(&self, realm: &Realm) -> Result<Option<HostDefined>, HostError> {
        self.alive()?;
        let handle = self.live_realm(realm)?;
        Ok(self
            .heap
            .realms
            .get(handle)
            .map_err(|_| HostError::StaleHandle)?
            .host_defined())
    }

    /// Call (`#sec-call`) of the function `function` holds, with the `this` value and arguments
    /// the roots hold, as an outermost entry (decision 14). A value that is not callable answers
    /// `Throw` of a `TypeError` (HOST-47). `realm` is the current realm of the call (decision
    /// 7) and is checked first: a stale or released one answers `Err(StaleHandle)`, one of
    /// another agent `Err(ForeignHandle)`. A safe point.
    pub fn call(
        &mut self,
        realm: &Realm,
        function: &Root,
        this: &Root,
        args: &[&Root],
    ) -> Result<Outcome<Root>, HostError> {
        self.alive()?;
        let realm = self.live_realm(realm)?;
        let function = self.heap.roots.get(function)?;
        let this = self.heap.roots.get(this)?;
        let args = self.root_values(args)?;
        Ok(self.outermost_entry(realm, |agent| agent.call_value(function, this, &args)))
    }

    /// Construct (`#sec-construct`) of the constructor `function` holds, `new_target` being
    /// `function` when `None`, as an outermost entry. A value that is not a constructor answers
    /// `Throw` of a `TypeError` (HOST-47); a `new_target` that is not a constructor answers
    /// `Err(WrongKind)`. `realm` is the current realm of the call and is checked first, as for
    /// [`Agent::call`]. A safe point.
    pub fn construct(
        &mut self,
        realm: &Realm,
        function: &Root,
        args: &[&Root],
        new_target: Option<&Root>,
    ) -> Result<Outcome<Root>, HostError> {
        self.alive()?;
        let realm = self.live_realm(realm)?;
        let function = self.heap.roots.get(function)?;
        let args = self.root_values(args)?;
        let new_target = match new_target {
            Some(root) => {
                let value = self.heap.roots.get(root)?;
                Some(self.constructor_or_wrong_kind(value)?)
            }
            None => None,
        };
        Ok(self.outermost_entry(realm, |agent| {
            agent.construct_checked(function, &args, new_target)
        }))
    }

    /// The values of `roots`.
    fn root_values(&self, roots: &[&Root]) -> Result<Vec<Value>, HostError> {
        roots.iter().map(|root| self.heap.roots.get(root)).collect()
    }

    /// The realm record handle `realm` holds, if the record is alive.
    pub(crate) fn live_realm(&self, realm: &Realm) -> Result<Handle, HostError> {
        let handle = self.heap.roots.get_realm(&realm.0)?;
        self.heap
            .realms
            .get(handle)
            .map_err(|_| HostError::StaleHandle)?;
        Ok(handle)
    }

    /// `value` as a constructor, or `Err(WrongKind)`: the new target of a construct, which
    /// Construct requires to be one (`#sec-construct`, "Assert: IsConstructor(newTarget) is
    /// true").
    pub(crate) fn constructor_or_wrong_kind(
        &mut self,
        value: Value,
    ) -> Result<crate::value::ObjectRef, HostError> {
        let wrong = HostError::WrongKind {
            expected: ValueKind::Constructor,
        };
        match value {
            Value::Object(object) => match self.has_construct(object) {
                Ok(true) => Ok(object),
                _ => Err(wrong),
            },
            _ => Err(wrong),
        }
    }

    /// Construct of `function` after the check that it is a constructor, which throws a
    /// `TypeError` otherwise.
    pub(crate) fn construct_checked(
        &mut self,
        function: Value,
        args: &[Value],
        new_target: Option<crate::value::ObjectRef>,
    ) -> Completion<Value> {
        if let Value::Object(object) = function
            && self.has_construct(object)?
        {
            return self.construct_value(object, args, new_target);
        }
        Err(self.throw_error(ErrorKind::TypeError, "the value is not a constructor"))
    }

    /// An outermost entry from `Agent` (decision 2) whose current realm is `realm` (decision 7):
    /// a safe point, a fresh run, and its completion as roots.
    fn outermost_entry(
        &mut self,
        realm: Handle,
        f: impl FnOnce(&mut Self) -> Completion<Value>,
    ) -> Outcome<Root> {
        self.termination = None;
        if self.safe_point().is_err() {
            self.fault();
            return Outcome::Terminated(Termination::EngineFault);
        }
        let start = self.scope.len();
        let completion = self.in_entry_realm(realm, |agent| agent.begin().and_then(|()| f(agent)));
        self.scope.truncate(start);
        let outcome = match completion {
            Ok(value) => Outcome::Normal(self.heap.roots.add(value)),
            Err(Abrupt::Throw(value)) => Outcome::Throw(self.heap.roots.add(value)),
            Err(Abrupt::Terminated(termination)) => Outcome::Terminated(termination),
        };
        self.end_entry();
        outcome
    }

    /// After an outermost entry: the next one runs afresh unless the agent is faulted, and the
    /// scripts handed out as `ScriptRef`s in it are no longer held.
    pub(crate) fn end_entry(&mut self) {
        self.scoped_scripts.clear();
        if self.state == AgentState::Alive {
            self.termination = None;
        }
    }

    /// ParseScript (`#sec-parse-script`, decision 8) of `source` in `realm`, whose
    /// [[HostDefined]] is `host_defined`: a `Script`, or the parse errors as `SyntaxError`
    /// objects of `realm` (`script.rs`). It runs no code, calls no hook and spends no step; it is
    /// an outermost entry that spends none, so `last_entry_steps()` then reads 0. It reads the
    /// interrupt flag at its start, after the safe point, as every outermost entry does
    /// (decision 12): a set flag is cleared and the call answers `Terminated(Interrupted)` before
    /// the source is copied. Over the memory budget it answers `Terminated(MemoryBudget)`.
    /// `realm` is checked first. A safe point.
    pub fn parse_script(
        &mut self,
        realm: &Realm,
        source: Source<'_>,
        host_defined: Option<HostDefined>,
    ) -> Result<Parsed<Script>, HostError> {
        self.alive()?;
        let realm = self.live_realm(realm)?;
        self.termination = None;
        self.steps = 0;
        if self.safe_point().is_err() {
            self.fault();
            return Ok(Parsed::Terminated(Termination::EngineFault));
        }
        if self.check_interrupt().is_err() {
            self.end_entry();
            return Ok(Parsed::Terminated(Termination::Interrupted));
        }
        let answer = self.in_entry_realm(realm, |agent| {
            agent.parse_script_in(realm, source, host_defined)
        });
        #[cfg(feature = "fault-injection")]
        let answer = if self.injected(FaultPoint::ParseScript) {
            Err(self.fault())
        } else {
            answer
        };
        let parsed = match answer {
            Ok(ParseAnswer::Script(script)) => {
                Parsed::Parsed(Script(self.heap.roots.add_script(script)))
            }
            Ok(ParseAnswer::Errors(errors)) => Parsed::Errors(
                errors
                    .into_iter()
                    .map(|error| self.heap.roots.add(error))
                    .collect(),
            ),
            Err(abrupt) => {
                let termination = parse_termination(abrupt);
                if termination == Termination::EngineFault {
                    self.fault();
                }
                Parsed::Terminated(termination)
            }
        };
        self.end_entry();
        Ok(parsed)
    }

    /// ScriptEvaluation (`#sec-runtime-semantics-scriptevaluation`, decision 8) of `script`, as
    /// an outermost entry whose current realm is the script's [[Realm]]: its completion value,
    /// or what it threw, as a root. A script may be evaluated again. A safe point.
    pub fn evaluate_script(&mut self, script: &Script) -> Result<Outcome<Root>, HostError> {
        self.alive()?;
        let handle = self.heap.roots.get_script(&script.0)?;
        let realm = self
            .heap
            .scripts
            .get(handle)
            .map_err(|_| HostError::StaleHandle)?
            .realm
            .ok_or(HostError::StaleHandle)?;
        Ok(self.outermost_entry(realm, |agent| agent.script_evaluation(handle)))
    }

    /// The [[HostDefined]] of `script`: the `Rc` given to `parse_script`.
    pub fn script_host_defined(&self, script: &Script) -> Result<Option<HostDefined>, HostError> {
        self.alive()?;
        let handle = self.heap.roots.get_script(&script.0)?;
        self.heap
            .scripts
            .get(handle)
            .map_err(|_| HostError::StaleHandle)?;
        Ok(self.script_record_host_defined(handle))
    }

    /// The steps the last outermost entry spent, or the running one so far (decision 13). Never
    /// more than `steps_per_entry`: the step that would pass it is not spent.
    pub fn last_entry_steps(&self) -> Result<u64, HostError> {
        self.alive()?;
        Ok(self.steps)
    }

    /// `Err(Faulted)` for a faulted agent.
    pub(crate) fn alive(&self) -> Result<(), HostError> {
        match self.state {
            AgentState::Alive => Ok(()),
            AgentState::Faulted => Err(HostError::Faulted),
        }
    }

    /// Whether the running entry may go on: not after its run was terminated.
    pub(crate) fn running(&self) -> Completion<()> {
        match (self.state, self.termination) {
            (AgentState::Faulted, _) => Err(Abrupt::Terminated(Termination::EngineFault)),
            (AgentState::Alive, Some(termination)) => Err(Abrupt::Terminated(termination)),
            (AgentState::Alive, None) => Ok(()),
        }
    }

    /// The start of a `Cx` operation that may run code or spend steps: the sticky termination;
    /// when no builtin runs, it is an outermost entry, whose step counter starts at 0 and which
    /// a set interrupt flag ends before it spends a step (decision 12).
    pub(crate) fn begin(&mut self) -> Completion<()> {
        self.running()?;
        if self.activations.is_empty() {
            self.steps = 0;
            if self.interrupt.is_set() {
                self.interrupt.clear();
                return Err(self.terminate(Termination::Interrupted));
            }
        }
        Ok(())
    }

    /// The one function that charges builtins' steps (decision 2), beside the interpreter's own
    /// step per instruction (`interp/budget.rs`): `steps` more to the running outermost entry. A
    /// set interrupt flag ends the run in `Terminated(Interrupted)` and is cleared; steps that
    /// would pass `steps_per_entry` are not spent, and the run ends in
    /// `Terminated(StepBudget)`, so the counter never reads more than the budget.
    pub(crate) fn charge_steps(&mut self, steps: u64) -> Completion<()> {
        if self.interrupt.is_set() {
            self.interrupt.clear();
            return Err(self.terminate(Termination::Interrupted));
        }
        let spent = self.steps.saturating_add(steps);
        if spent > self.limits.steps_per_entry {
            return Err(self.terminate(Termination::StepBudget));
        }
        self.steps = spent;
        Ok(())
    }

    /// End the running entry with `termination`; `EngineFault` faults the agent.
    pub(crate) fn terminate(&mut self, termination: Termination) -> Abrupt {
        if termination == Termination::EngineFault {
            self.state = AgentState::Faulted;
        }
        if termination == Termination::MemoryBudget {
            self.collect_pending = true;
        }
        self.termination = Some(termination);
        Abrupt::Terminated(termination)
    }

    /// The engine found one of its invariants broken: end the run and fault the agent.
    pub(crate) fn fault(&mut self) -> Abrupt {
        self.terminate(Termination::EngineFault)
    }

    /// A collection, with the bytes of the property maps it freed credited and the host's
    /// charges whose `HostBytes` were dropped (in the sweep, with the host data that held them,
    /// or before by the host), and then the collection threshold set from the live reading.
    /// Both credits read a `Cell`: the collection allocates nothing for them.
    fn collect_now(&mut self) -> Result<CollectReport, Stale> {
        let mut report = collect::collect(self)?;
        let freed = self.credit.replace(0);
        self.heap.accounting.credit(freed);
        self.credit_dropped_host_bytes();
        self.heap.collected();
        report.heap_bytes_after = self.heap.bytes();
        Ok(report)
    }

    /// A safe point (decision 4): release the dropped roots, and collect when the bytes
    /// allocated since the last collection passed the threshold, when the last run ended in
    /// `Terminated(MemoryBudget)`, or always with the feature `collect-every-safe-point`.
    /// Each trigger counts bytes or follows an outcome, never time.
    pub(crate) fn safe_point(&mut self) -> Result<(), Stale> {
        self.heap.roots.drain();
        if cfg!(feature = "collect-every-safe-point")
            || self.collect_pending
            || self.heap.collection_due()
        {
            self.collect_pending = false;
            self.collect_now()?;
        }
        Ok(())
    }

    /// Record `value` as handed to host code in the running scope.
    pub(crate) fn hand_out(&mut self, value: Value) {
        self.scope.push(value);
    }

    /// Hand out the value of a completion, normal or thrown, and answer it.
    pub(crate) fn hand_out_completion(
        &mut self,
        completion: Completion<Value>,
    ) -> Completion<Value> {
        if let Ok(value) | Err(Abrupt::Throw(value)) = completion {
            self.hand_out(value);
        }
        completion
    }
}

/// Root site of the collector: the values handed to host code in the running scopes, and the
/// scripts handed out as `ScriptRef`s in the running outermost entry.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("host/agent.rs trace_roots");
    for &value in &agent.scope {
        mark.value(value);
    }
    for &script in &agent.scoped_scripts {
        mark.push(Item::Script(script));
    }
}
