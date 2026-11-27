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
//! running `with_cx`, host function and hook scopes, the scripts handed out as `ScriptRef`s,
//! and, through `host/jobs.rs`, what the microtask queue holds.
//!
//! The calls of the microtask queue (decision 11), `queue_microtask`, `pending_microtasks`,
//! `clear_microtasks` and `perform_microtask_checkpoint`, are written by
//! `story:job-queue-and-microtask-checkpoint`; the queue and its loop are `host/jobs.rs`. The
//! reads of a promise, `promise_state`, `promise_is_handled` and `mark_promise_as_handled`, are
//! written by `story:promise-rejection-tracker-and-promise-reads`.

use alloc::collections::VecDeque;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::Cell;
use core::sync::atomic::{AtomicU32, Ordering};

use swell_compiler::bytecode::Instruction;

use super::hooks::HostDefined;
use super::host_bytes::HostBytes;
use super::host_object::HostObject;
use super::interrupt::InterruptHandle;
use super::jobs::{CheckpointReport, Entry, Microtask};
use super::limits::{CreateError, Limits, Seeds};
use super::outcome::{
    Abrupt, CollectReport, Completion, ErrorSite, ErrorSummary, HostError, Outcome, Parsed,
    Termination, ValueKind,
};
use super::{ErrorKind, GlobalSetup, Host, PromiseState, Source};
use crate::heap::Heap;
use crate::heap::arena::Handle;
use crate::heap::collect::{self, Item, Mark, Stale};
use crate::interp::Interp;
use crate::object::ObjectKind;
use crate::object::builtin_function::Activation;
use crate::object::promise::{Promise, PromiseState as Settlement};
use crate::realm::WellKnown;
use crate::script::{Compiled, ParseAnswer, parse_termination};
use crate::value::{ObjectRef, Realm, Root, Script, Value};

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

/// Where an error arose, as the agent keeps it past the end of the entry and a collection
/// (decision 13): what the read turns into an [`ErrorSite`].
pub(crate) enum SiteRecord {
    /// The first error of a parse, with the [[HostDefined]] given to it.
    Parse {
        line: u32,
        column: u32,
        host_defined: Option<HostDefined>,
    },
    /// An instruction: the code that holds it, its template (`None` for the script's own code)
    /// and its index, with a clone of its script record's [[HostDefined]].
    Code {
        compiled: Rc<Compiled>,
        template: Option<u32>,
        index: usize,
        host_defined: Option<HostDefined>,
    },
}

impl SiteRecord {
    /// The site: the line and column of an instruction are read from its code's positions.
    fn site(&self) -> Option<ErrorSite> {
        match self {
            SiteRecord::Parse {
                line,
                column,
                host_defined,
            } => Some(ErrorSite {
                line: *line,
                column: *column,
                dynamic: false,
                host_defined: host_defined.clone(),
            }),
            SiteRecord::Code {
                compiled,
                template,
                index,
                host_defined,
            } => {
                let positions = match template {
                    None => &compiled.code.positions,
                    Some(template) => {
                        let template = usize::try_from(*template).ok()?;
                        &compiled.code.functions.get(template)?.positions
                    }
                };
                let span = positions.get(*index)?;
                Some(ErrorSite {
                    line: span.line,
                    column: span.column,
                    dynamic: compiled.dynamic,
                    host_defined: host_defined.clone(),
                })
            }
        }
    }
}

/// The answer of a read of a promise given a value that is no promise (HOST-46).
const NOT_A_PROMISE: HostError = HostError::WrongKind {
    expected: ValueKind::Promise,
};

/// The object `value` is, or the answer of a read of a promise given a primitive.
fn not_a_promise_unless_object(value: Value) -> Result<ObjectRef, HostError> {
    match value {
        Value::Object(object) => Ok(object),
        _ => Err(NOT_A_PROMISE),
    }
}

/// Whether two thrown values are the same value by identity: the same record, or the same bits.
fn same_thrown(a: Value, b: Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.to_bits() == y.to_bits(),
        _ => a == b,
    }
}

/// One engine instance (9.6).
pub struct Agent<H: Host> {
    pub(crate) host: H,
    pub(crate) limits: Limits,
    /// The seed of every realm's `Math.random` generator (`realm/mod.rs`).
    pub(crate) random_seed: [u8; 32],
    /// The realms `create_realm` started, the creation index of the next: every call that gets
    /// past `alive()` takes one, a creation that ends in a termination, a throw or a fault
    /// included, so that no index is used twice (`docs/design/engine.md`, decision 6,
    /// "`Math.random`"; HOST-213).
    pub(crate) realm_count: u64,
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
    /// The site of the throw that unwinds in the running entry, with the thrown value, which is
    /// compared by identity and never read (decision 13, [`Agent::record_throw_site`]).
    pub(crate) throw_site: Option<(Value, SiteRecord)>,
    /// How many throw sites were recorded, wrapping: a builtin compares it across its run.
    pub(crate) throw_sites: u64,
    /// The line and column of the first error `syntax_errors` turned into an object, read by
    /// `parse_script`.
    pub(crate) syntax_error_at: Option<(u32, u32)>,
    /// The site the last outermost entry answered ([`Agent::last_entry_error_site`]).
    pub(crate) error_site: Option<SiteRecord>,
    /// The microtask queue, one FIFO order for every entry (decision 11, `host/jobs.rs`).
    pub(crate) microtasks: VecDeque<Entry<H>>,
    /// The charge of the microtask queue's capacity, capacity × `size_of::<Entry<H>>()` bytes
    /// (`host/jobs.rs`).
    pub(crate) queue_charge: crate::heap::accounting::Charge,
    /// The counting build: what the queue adds to the tallies of `copies`.
    #[cfg(test)]
    pub(crate) queue_tally: crate::object::string::copies::Tally,
    /// A checkpoint runs: every `Cx` call is a nested entry of it, and a checkpoint asked for
    /// inside it answers at once ("performing a microtask checkpoint",
    /// `html:perform-a-microtask-checkpoint`).
    pub(crate) checkpointing: bool,
    /// The [[HostDefined]] of the JobCallbacks of the promise jobs that run, innermost last, as
    /// host data the collector traces (`host/jobs.rs`): a job taken off the queue is no longer
    /// reached through it.
    pub(crate) running_host_defined: Vec<HostObject>,
    /// Test only: how many site records were written.
    #[cfg(test)]
    pub(crate) site_records: u64,
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
            realm_count: 0,
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
            throw_site: None,
            throw_sites: 0,
            syntax_error_at: None,
            error_site: None,
            microtasks: VecDeque::new(),
            queue_charge: crate::heap::accounting::Charge::default(),
            #[cfg(test)]
            queue_tally: crate::object::string::copies::Tally::default(),
            checkpointing: false,
            running_host_defined: Vec::new(),
            #[cfg(test)]
            site_records: 0,
            #[cfg(feature = "fault-injection")]
            injected: None,
        })
    }

    /// How many refusals of a later slice's construct the agent raised (`host-interface.md`,
    /// section 13, "A count of refusals"): every `SyntaxError` whose message
    /// `swell_syntax::message_kind` reads as a refusal, raised by a parse through
    /// `parse_script`, `eval` or `Function` or by a builtin (RegExpInitialize, of the
    /// constructor or of a regular expression literal), caught by the script or not. The nesting limit and every other
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
        let bytes = core::mem::take(&mut charge.bytes);
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
    /// `setup` returns faults the agent (decision 6). It is a safe point. The realm's creation
    /// index is taken first, before any step that can fail, and its `Math.random` state is
    /// derived from it.
    pub fn create_realm(
        &mut self,
        setup: Option<GlobalSetup<'_, H>>,
        host_defined: Option<HostDefined>,
    ) -> Result<Outcome<Realm>, HostError> {
        self.alive()?;
        let index = self.realm_count;
        // 2^64 creations cannot be made; reaching the end of the counter is a broken invariant.
        let Some(next) = index.checked_add(1) else {
            self.fault();
            return Ok(Outcome::Terminated(Termination::EngineFault));
        };
        self.realm_count = next;
        self.termination = None;
        self.steps = 0;
        self.clear_error_sites();
        if self.safe_point().is_err() {
            self.fault();
            return Ok(Outcome::Terminated(Termination::EngineFault));
        }
        let start = self.scope.len();
        let outcome = self.initialize_host_defined_realm(setup, host_defined, index);
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
        self.clear_error_sites();
        if self.safe_point().is_err() {
            self.fault();
            return Outcome::Terminated(Termination::EngineFault);
        }
        let start = self.scope.len();
        let completion = self.in_entry_realm(realm, |agent| agent.begin().and_then(|()| f(agent)));
        self.scope.truncate(start);
        let (outcome, site) = match completion {
            Ok(value) => (Outcome::Normal(self.heap.roots.add(value)), None),
            Err(Abrupt::Throw(value)) => (
                Outcome::Throw(self.heap.roots.add(value)),
                self.answered_site(value),
            ),
            Err(Abrupt::Terminated(termination)) => (Outcome::Terminated(termination), None),
        };
        self.end_entry();
        self.error_site = site;
        outcome
    }

    /// After an outermost entry: the next one runs afresh unless the agent is faulted, and the
    /// scripts handed out as `ScriptRef`s in it are no longer held. No error site is kept: the
    /// entries that answer one set it after this (decision 13).
    pub(crate) fn end_entry(&mut self) {
        self.clear_error_sites();
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
        self.clear_error_sites();
        if self.safe_point().is_err() {
            self.fault();
            return Ok(Parsed::Terminated(Termination::EngineFault));
        }
        if self.check_interrupt().is_err() {
            self.end_entry();
            return Ok(Parsed::Terminated(Termination::Interrupted));
        }
        let site_host_defined = host_defined.clone();
        let answer = self.in_entry_realm(realm, |agent| {
            agent.parse_script_in(realm, source, host_defined)
        });
        #[cfg(feature = "fault-injection")]
        let answer = if self.injected(FaultPoint::ParseScript) {
            Err(self.fault())
        } else {
            answer
        };
        let mut site = None;
        let parsed = match answer {
            Ok(ParseAnswer::Script(script)) => {
                Parsed::Parsed(Script(self.heap.roots.add_script(script)))
            }
            Ok(ParseAnswer::Errors(errors)) => {
                site = self.parse_error_site(site_host_defined);
                Parsed::Errors(
                    errors
                        .into_iter()
                        .map(|error| self.heap.roots.add(error))
                        .collect(),
                )
            }
            Err(abrupt) => {
                let termination = parse_termination(abrupt);
                if termination == Termination::EngineFault {
                    self.fault();
                }
                Parsed::Terminated(termination)
            }
        };
        self.end_entry();
        self.error_site = site;
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

    /// Queue the host microtask `task` behind every queued entry (decision 11, "A host
    /// microtask's realm"): `realm` is its current realm while it runs, and is checked first: a
    /// stale or released one answers `Err(StaleHandle)`, one of another agent
    /// `Err(ForeignHandle)`, and nothing is queued (HOST-192). It runs no code, spends no step and
    /// calls no hook; the queue holds the realm until the microtask is taken off. A host
    /// microtask's closure is the host's storage and is not charged to the memory budget; its
    /// slot of the queue's capacity is, and when the budget refuses the queue's growth nothing
    /// is queued and it answers `Err(MemoryBudget)` (HOST-215).
    pub fn queue_microtask(&mut self, realm: &Realm, task: Microtask<H>) -> Result<(), HostError> {
        self.alive()?;
        let realm = self.live_realm(realm)?;
        self.enqueue_microtask(realm, task)
    }

    /// How many entries the microtask queue holds (section 13). A read: it changes nothing,
    /// runs no code, calls no hook and spends no step.
    pub fn pending_microtasks(&self) -> Result<usize, HostError> {
        self.alive()?;
        Ok(self.microtasks.len())
    }

    /// Drop every queued entry unrun and answer how many (decision 11, "Steps, the interrupt and
    /// what a checkpoint leaves"; `engine.md`, decision 3, "Emptying the queue is the host's
    /// call"): a host microtask is dropped as a Rust value, its closure never called, and a
    /// promise job is dropped with the bytes of its entry credited ("Queued jobs"). It runs no
    /// script, calls no hook and spends no step, so `last_entry_steps()` reads what it read. It
    /// takes `&mut Agent`, which no code inside an entry holds: a job cannot drop the entries
    /// behind it.
    pub fn clear_microtasks(&mut self) -> Result<usize, HostError> {
        self.alive()?;
        Ok(self.drop_microtasks())
    }

    /// HTML's microtask checkpoint (`html:perform-a-microtask-checkpoint`; decision 11, "The
    /// checkpoint is one entry"), as one outermost entry with one step budget: a safe point, the
    /// step counter from 0 and the interrupt read before a step is spent (`begin`); then, while
    /// the queue is not empty, one step for the first entry, charged while it is still first,
    /// then that entry taken off and run, each in a scope of its own (`host/jobs.rs`). It
    /// answers `Normal` with the count of each kind once the queue is empty (HOST-79); a
    /// termination of any entry, `StepBudget`, `MemoryBudget` or `Interrupted`, ends the whole
    /// checkpoint with that reason: the entry that was running is not run again, and every entry
    /// not started stays queued in its order for the next checkpoint (section 10, HOST-77). The
    /// interrupt ends it at its start, before each entry is taken off and while an entry runs,
    /// and is cleared. `last_entry_steps()` then reads the steps of the whole checkpoint, at most
    /// `steps_per_entry`, 0 when it found the queue empty (HOST-193, HOST-197).
    /// `Terminated(EngineFault)` faults the agent, and its queue never runs again.
    pub fn perform_microtask_checkpoint(&mut self) -> Result<Outcome<CheckpointReport>, HostError> {
        self.alive()?;
        self.termination = None;
        self.clear_error_sites();
        if self.safe_point().is_err() {
            self.fault();
            return Ok(Outcome::Terminated(Termination::EngineFault));
        }
        let start = self.scope.len();
        let completion = self.begin().and_then(|()| self.checkpoint());
        self.scope.truncate(start);
        let outcome = match completion {
            Ok(report) => Outcome::Normal(report),
            Err(Abrupt::Terminated(termination)) => Outcome::Terminated(termination),
            // The loop answers no throw: an entry's throw stays inside the entry.
            Err(Abrupt::Throw(_)) => {
                self.fault();
                Outcome::Terminated(Termination::EngineFault)
            }
        };
        self.end_entry();
        Ok(outcome)
    }

    /// Whether the value `value` holds is an object with [[ErrorData]], with the kind the
    /// constructor that made it recorded (or the host's `HostObjectSpec::error_kind`) and its
    /// own "message" when that is a data property holding a String, its code units exact
    /// (decision 13). `None` for any other value. A read: no code, no hook, no step, no
    /// allocation in the heap, so `heap_bytes()` and `live_bytes()` read the same after it.
    pub fn error_summary(&self, value: &Root) -> Result<Option<ErrorSummary>, HostError> {
        self.alive()?;
        let value = self.heap.roots.get(value)?;
        self.scoped_error_summary(value)
    }

    /// [[PromiseState]] of the promise `promise` holds, with its [[PromiseResult]] as a new root
    /// once it is settled (section 13; HTML reads both, `html:notify-about-rejected-promises`).
    /// A read: it runs no code, calls no hook and spends no step. Its one allocation is the root
    /// of a settled promise's result, an entry of the root table, which is the host's storage
    /// for its own calls and charged to neither budget (`engine.md`, decision 4, "Memory
    /// budget"), so `heap_bytes()` and `live_bytes()` read the same after it. A root that holds
    /// no promise answers `Err(WrongKind { expected: Promise })` (HOST-46).
    pub fn promise_state(&mut self, promise: &Root) -> Result<PromiseState, HostError> {
        self.alive()?;
        let value = self.heap.roots.get(promise)?;
        Ok(match self.host_promise(value)?.state {
            Settlement::Pending => PromiseState::Pending,
            Settlement::Fulfilled(result) => PromiseState::Fulfilled(self.heap.roots.add(result)),
            Settlement::Rejected(result) => PromiseState::Rejected(self.heap.roots.add(result)),
        })
    }

    /// [[PromiseIsHandled]] of the promise `promise` holds: whether a handler was added to it
    /// or the host marked it (section 13; `html:notify-about-rejected-promises`). A read: no
    /// code, no hook, no step, no allocation. A root that holds no promise answers
    /// `Err(WrongKind { expected: Promise })`.
    pub fn promise_is_handled(&self, promise: &Root) -> Result<bool, HostError> {
        self.alive()?;
        let value = self.heap.roots.get(promise)?;
        Ok(self.host_promise(value)?.is_handled)
    }

    /// Set [[PromiseIsHandled]] of the promise `promise` holds to true (Web IDL, "mark as
    /// handled", `webidl:mark-a-promise-as-handled`): a handler added to it later causes no call
    /// of `host_promise_rejection_tracker` with `Handle`, and a rejection of it while it is
    /// pending none with `Reject` (HOST-101). It runs no code, calls no hook, spends no step and
    /// allocates nothing. A root that holds no promise answers
    /// `Err(WrongKind { expected: Promise })`.
    pub fn mark_promise_as_handled(&mut self, promise: &Root) -> Result<(), HostError> {
        self.alive()?;
        let value = self.heap.roots.get(promise)?;
        let record = self
            .heap
            .objects
            .get_mut(not_a_promise_unless_object(value)?.handle)
            .map_err(super::host_error)?;
        match &mut record.kind {
            ObjectKind::Promise(slots) => {
                slots.is_handled = true;
                Ok(())
            }
            _ => Err(NOT_A_PROMISE),
        }
    }

    /// The slots of the promise `value` is, for the host's reads: `Err(WrongKind { expected:
    /// Promise })` for any other value.
    fn host_promise(&self, value: Value) -> Result<&Promise, HostError> {
        let record = self
            .heap
            .objects
            .get(not_a_promise_unless_object(value)?.handle)
            .map_err(super::host_error)?;
        match &record.kind {
            ObjectKind::Promise(slots) => Ok(slots),
            _ => Err(NOT_A_PROMISE),
        }
    }

    /// The steps the last outermost entry spent, or the running one so far (decision 13). Never
    /// more than `steps_per_entry`: the step that would pass it is not spent.
    pub fn last_entry_steps(&self) -> Result<u64, HostError> {
        self.alive()?;
        Ok(self.steps)
    }

    /// Where the error the last outermost entry answered arose (decision 13): after
    /// `parse_script` answered `Parsed::Errors`, the line and column of its first error; after
    /// `evaluate_script`, `call` or `construct` answered `Throw`, those of the instruction where
    /// the throw arose in the innermost ECMAScript frame, whether that code is eval or
    /// `Function` code, and the [[HostDefined]] of its script. After any other entry, and for
    /// a throw raised outside any ECMAScript frame, `None`. A read: it runs no code, calls no
    /// hook, spends no step and allocates nothing in the heap; it answers the same after a
    /// collection.
    pub fn last_entry_error_site(&self) -> Result<Option<ErrorSite>, HostError> {
        self.alive()?;
        Ok(self.error_site.as_ref().and_then(SiteRecord::site))
    }

    /// The start of an outermost entry: no site is recorded (decision 13).
    fn clear_error_sites(&mut self) {
        self.throw_site = None;
        self.syntax_error_at = None;
        self.error_site = None;
    }

    /// Record where the throw of `thrown` arose: the running frame's last instruction, unless
    /// the same value is unwinding already (through callers, builtins and `finally` blocks); a
    /// `throw` statement always starts a new record. Called by `unwind`, kept out of the run
    /// loop's frame.
    #[inline(never)]
    pub(crate) fn record_throw_site(&mut self, thrown: Value) {
        let Some(frame) = self.interp.frames.last() else {
            return;
        };
        let index = frame.pc.saturating_sub(1);
        let statement = matches!(frame.instructions().get(index), Some(Instruction::Throw));
        if !statement
            && let Some((pending, _)) = &self.throw_site
            && same_thrown(*pending, thrown)
        {
            return;
        }
        let compiled = Rc::clone(&frame.compiled);
        let template = frame.template;
        let script = frame.script;
        let host_defined = self.script_record_host_defined(script);
        self.throw_site = Some((
            thrown,
            SiteRecord::Code {
                compiled,
                template,
                index,
                host_defined,
            },
        ));
        self.throw_sites = self.throw_sites.wrapping_add(1);
        #[cfg(test)]
        {
            self.site_records = self.site_records.saturating_add(1);
        }
    }

    /// The throw that unwinds was caught: a `catch` block was entered, or a builtin or a host
    /// function answered normally after it.
    pub(crate) fn end_throw_site(&mut self) {
        self.throw_site = None;
    }

    /// The site of the first error of the parse that just answered errors, with the
    /// [[HostDefined]] given to it; `None` for an error with no position (a bound of the
    /// bytecode format, `CompileError::TooMany`).
    fn parse_error_site(&mut self, host_defined: Option<HostDefined>) -> Option<SiteRecord> {
        let (line, column) = self.syntax_error_at.take()?;
        #[cfg(test)]
        {
            self.site_records = self.site_records.saturating_add(1);
        }
        Some(SiteRecord::Parse {
            line,
            column,
            host_defined,
        })
    }

    /// The site of the throw of `thrown` an outermost entry answers: the record, when it is of
    /// that value.
    fn answered_site(&mut self, thrown: Value) -> Option<SiteRecord> {
        let (pending, site) = self.throw_site.take()?;
        same_thrown(pending, thrown).then_some(site)
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
    /// when no builtin runs and no checkpoint runs, it is an outermost entry, whose step counter
    /// starts at 0 and which a set interrupt flag ends before it spends a step (decision 12).
    /// Inside a checkpoint every call of an entry is nested and spends from the checkpoint
    /// (`engine.md`, decision 3, "The microtask checkpoint is one entry"). Inside the override
    /// of a host object (`object/host_overrides.rs`), which runs as an internal method of an
    /// object with no activation of its own, every call is nested too: it spends from the entry
    /// that called the internal method.
    pub(crate) fn begin(&mut self) -> Completion<()> {
        self.running()?;
        if self.activations.is_empty() && !self.checkpointing && self.interp.proxy_nesting == 0 {
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

/// Root site of the collector: the values handed to host code in the running scopes, the
/// scripts handed out as `ScriptRef`s in the running outermost entry, and what the microtask
/// queue holds with the [[HostDefined]] of the promise jobs that run (`host/jobs.rs`).
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("host/agent.rs trace_roots");
    for &value in &agent.scope {
        mark.value(value);
    }
    for &script in &agent.scoped_scripts {
        mark.push(Item::Script(script));
    }
    super::jobs::trace_roots(agent, mark);
}
