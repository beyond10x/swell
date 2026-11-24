#![forbid(unsafe_code)]
//! The host interface of swell as a [`ConformanceTarget`]: one function per command of
//! `spec/domains/host.yaml` that the engine has an entry point for, each turning the model's input
//! into one call of the interface and the interface's answer into the model's outcome, error and
//! events.
//!
//! | command | the call | outcomes the call answers |
//! |---|---|---|
//! | `CreateAgent` | `Agent::new(host, limits, seeds)` | `created` |
//! | `CreateRealm` | `Agent::create_realm(None, None)` | `created`, `faulted` |
//! | `ParseScript` | `Agent::parse_script(realm, Source::Utf8(text), None)` | `parsed`, `parse-failed`, `memory-budget`, `foreign-realm`, `faulted` |
//! | `EvaluateScript` | `Agent::evaluate_script(script)` | `completed`, `threw`, `step-budget`, `memory-budget`, `interrupted`, `engine-fault`, `faulted` |
//! | `PerformMicrotaskCheckpoint` | `Agent::perform_microtask_checkpoint()` | `drained`, `step-budget`, `memory-budget`, `interrupted`, `engine-fault`, `faulted` |
//! | `Interrupt` | `InterruptHandle::interrupt` on `Agent::interrupt_handle()` | `set` |
//! | `Collect` | `Agent::collect()` | `collected`, `engine-fault`, `faulted` |
//!
//! Two commands the engine issues itself are awaited, never invoked: the hooks
//! `host_enqueue_promise_job` (`HostEnqueuePromiseJob`, `enqueued`) and
//! `host_promise_rejection_tracker` (`HostPromiseRejectionTracker`, `tracked`). The
//! [`ConformanceHost`] of each agent records every call of the two with its realm or its
//! operation, in the agent's own host value; an await of one of them on an agent answers the
//! first call of that hook the agent recorded and no await has answered yet, so the calls are
//! answered in the order the engine made them. An await no recorded call answers is
//! `unsupported`: the engine runs only inside a command, so a hook it did not call by then is
//! never called for that await.
//!
//! The model's identities are the target's: each scenario keeps its agents by `agent_id`, and
//! the `Realm` and `Script` values the interface returned by `realm_id` and `script_id`, each with
//! the agent that made it (the model's `Agent` owns its realms, `realms` via `agent_id`). An
//! identity the scenario holds no value for is the model's `NotHeld` (`no-agent`, `no-realm`,
//! `no-script`), answered here without a call, since the interface has no identity that names
//! nothing. An identity the target makes is a UUID of version 4 form, numbered in the scenario.
//!
//! What the target answers `unsupported`, with its reason, never as a pass:
//!
//! - the nine commands without an entry point yet: the module commands (slice 11) and the hooks
//!   `HostEnqueueGenericJob` and `HostEnqueueTimeoutJob` (slice 14);
//! - an await of a hook the engine did not call (above);
//! - an answer of the interface the model has no outcome for, each an `UNMAPPED:` marker of the
//!   model: `Err(ForeignHandle)` for a script of another agent (HOST-45, HOST-48), a
//!   `create_realm` that ends in `Throw` or `Terminated` (HOST-12, HOST-41, HOST-71), a
//!   `parse_script` that ends in `Terminated(EngineFault)` (HOST-71);
//! - a parse the parser of this slice refuses as a construct of a later slice (a `SyntaxError`
//!   whose message is a `swell_syntax::MessageKind::Refusal`): the source is a Script, so the
//!   refusal is not the model's `parse-failed`;
//! - a forced outcome the scenario gives the engine no means to reach.
//!
//! A forced outcome is otherwise reached for real, never injected into the answer. The target
//! accepts the force and runs the call on the scenario's own input, and reports what the engine
//! answered, so that an engine that does not reach the outcome fails the scenario: `ParseScript`'s
//! `parse-failed` (a source that is no Script), `EvaluateScript`'s `threw`, `step-budget` and
//! `memory-budget` (a source and limits that reach them) and `interrupted` (the scenario's
//! `Interrupt` before the run; HOST-92). `PerformMicrotaskCheckpoint`'s `step-budget` and
//! `memory-budget` are reached by the jobs the scenario's scripts queued, and its `interrupted`
//! through the scenario's `Interrupt` on the agent (HOST-198); a checkpoint forced to the first
//! two on an empty queue, or to the third on an agent no `Interrupt` of the scenario was made
//! on, has no means to reach them and is unsupported. `Collect`'s `engine-fault` is reached
//! through the interface's own test switch, the feature `fault-injection`
//! (`Agent::inject_fault`, HOST-71). Unsupported stay `ParseScript`'s `memory-budget` (the
//! target makes no limits and no source of its own, and no scenario's parse passes its budget)
//! and the `engine-fault` of `EvaluateScript` and `PerformMicrotaskCheckpoint` (the fault switch
//! has no point at either).
//!
//! How the `Interrupt` command sets the flag is the target's [`Interrupter`]:
//! [`sets_the_flag`] in the target of [`shipped`](crate::shipped), a function that does nothing
//! in the planted-defect test. How `PerformMicrotaskCheckpoint` runs the queue is its
//! [`Checkpoint`]: [`runs_the_queue`] in the shipped target, one that answers `Normal` without
//! running a job in the planted-defect test.
//!
//! Every agent runs on the thread [`run`](crate::run) makes, of [`STACK_BYTES`]; an agent whose
//! `Limits::min_stack_bytes()` is more is not created (HOST-17).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use ess_conformance::scenario::{CommandRef, ErrorRef, EventRef, OutcomeRef};
use ess_conformance::target::{
    AwaitAnswer, AwaitRequest, ConformanceTarget, DeclaredErrorValue, EventObservationRequest,
    ExternalOutcomeControl, ImplementationIdentity, ObservedEvent, RedeliveryRequest,
    ScenarioContext, SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest,
    SemanticViewResult, TargetError,
};
use ess_domain::command::OutcomeName;
use ess_primitives::ids::CorrelationId;
use ess_primitives::node::Node;
use swell::{
    Agent, CheckpointReport, Cx, FaultPoint, HookCx, Host, HostError, InterruptHandle, Job,
    LoadRequest, ObjectRef, Outcome, Parsed, PropertyKey, Realm, RealmRef, RejectionOperation,
    Root, Script, Source, Termination, Value,
};
use swell_syntax::{MessageKind, message_kind};

use crate::values::{self, Input};

/// The stack, in bytes, of the thread every scenario runs on: 64 MiB. The limits of every
/// `CreateAgent` of the suite (1,000 parser levels, 256 re-entries) need 10,059,520 bytes by
/// `Limits::min_stack_bytes()`.
pub const STACK_BYTES: usize = 64 * 1024 * 1024;

/// How the target sets an agent's interrupt flag for the `Interrupt` command.
pub type Interrupter = fn(&InterruptHandle);

/// The interface's own: `InterruptHandle::interrupt`.
pub fn sets_the_flag(handle: &InterruptHandle) {
    handle.interrupt();
}

/// How the target performs an agent's microtask checkpoint for `PerformMicrotaskCheckpoint`.
pub type Checkpoint =
    fn(&mut Agent<ConformanceHost>) -> Result<Outcome<CheckpointReport>, HostError>;

/// The interface's own: `Agent::perform_microtask_checkpoint`.
pub fn runs_the_queue(
    agent: &mut Agent<ConformanceHost>,
) -> Result<Outcome<CheckpointReport>, HostError> {
    agent.perform_microtask_checkpoint()
}

/// The host of every agent the target creates. It records each call of the two hooks the model
/// states as commands the engine issues, in the order the engine made them; a promise job stays
/// in the agent's queue, as the hook's default leaves it. A generic, timeout or cleanup job and
/// a module load the engine hands it are dropped (slices 14, 16 and 11), and the clock reads the
/// epoch.
#[derive(Default)]
pub struct ConformanceHost {
    /// Every call of a recorded hook no await has answered yet, oldest first.
    issued: Vec<Issued>,
}

/// One call of a hook the model states as a command the engine issues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Issued {
    /// `host_enqueue_promise_job` with the job's realm or none (`HostEnqueuePromiseJob`).
    PromiseJob(Option<RealmRef>),
    /// `host_promise_rejection_tracker` with its operation (`HostPromiseRejectionTracker`).
    RejectionTracker(RejectionOperation),
}

/// Which recorded hook an await asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Hook {
    PromiseJob,
    RejectionTracker,
}

impl Issued {
    fn hook(self) -> Hook {
        match self {
            Self::PromiseJob(_) => Hook::PromiseJob,
            Self::RejectionTracker(_) => Hook::RejectionTracker,
        }
    }
}

impl ConformanceHost {
    /// The oldest call of `hook` no await has answered yet, now answered.
    fn take(&mut self, hook: Hook) -> Option<Issued> {
        let index = self.issued.iter().position(|call| call.hook() == hook)?;
        Some(self.issued.remove(index))
    }
}

impl Host for ConformanceHost {
    /// HostEnqueuePromiseJob: recorded with the job's realm; the job stays in the agent's queue.
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, realm: Option<RealmRef>) {
        cx.host_mut().issued.push(Issued::PromiseJob(realm));
    }

    /// HostPromiseRejectionTracker: recorded with its operation. The promise is an engine value
    /// the model does not hold (`spec/domains/host.yaml`, the `UNMAPPED:` marker of the command).
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        operation: RejectionOperation,
    ) {
        cx.host_mut()
            .issued
            .push(Issued::RejectionTracker(operation));
    }

    fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}

    fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}

    fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}

    fn host_system_utc_epoch_nanoseconds(_cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        0
    }
}

/// A forced outcome the target arranges through a control of the interface before the next
/// invocation of its command; every other one it accepts is reached by the call itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Forced {
    /// `Collect`'s `engine-fault`: the fault switch is set at `collect` before the call.
    CollectFault,
    /// `PerformMicrotaskCheckpoint`'s `step-budget` or `memory-budget`: reached by the queued
    /// jobs, so unsupported on an empty queue.
    CheckpointBudget,
    /// `PerformMicrotaskCheckpoint`'s `interrupted`: reached through the scenario's `Interrupt`,
    /// so unsupported on an agent no `Interrupt` of the scenario was made on.
    CheckpointInterrupt,
}

/// A value of the interface and the agent that made it.
struct Held<T> {
    agent: String,
    value: T,
}

/// One scenario's agents and the values the interface returned, by the model's identities.
#[derive(Default)]
struct Scenario {
    agents: BTreeMap<String, Agent<ConformanceHost>>,
    realms: BTreeMap<String, Held<Realm>>,
    scripts: BTreeMap<String, Held<Script>>,
    /// Identities made so far in this scenario.
    made: u64,
    forced: Option<Forced>,
    /// The agents the scenario's `Interrupt` was made on.
    interrupted: BTreeSet<String>,
    /// Every event the scenario's commands published, in order.
    published: Vec<ObservedEvent>,
}

impl Scenario {
    /// A new identity for an `AgentId` (kind 1), `RealmId` (2), `ScriptId` (3) or `JobId` (4).
    fn identity(&mut self, kind: u32) -> String {
        self.made += 1;
        format!("{kind:08x}-0000-4000-8000-{:012x}", self.made)
    }

    /// The agent `agent_id` names.
    fn agent(&mut self, agent_id: &str) -> Option<&mut Agent<ConformanceHost>> {
        self.agents.get_mut(agent_id)
    }

    /// The forced outcome, taken when it is `forced`; another stays for its own command.
    fn take(&mut self, forced: Forced) -> bool {
        if self.forced == Some(forced) {
            self.forced = None;
            return true;
        }
        false
    }

    /// The forced outcome of `PerformMicrotaskCheckpoint`, taken; another stays for its own
    /// command.
    fn take_checkpoint(&mut self) -> Option<Forced> {
        let forced = self.forced?;
        if matches!(
            forced,
            Forced::CheckpointBudget | Forced::CheckpointInterrupt
        ) {
            self.forced = None;
            return Some(forced);
        }
        None
    }
}

/// A command's answer: its branch, the declared error it carries and the events it published,
/// each with its payload.
struct Answer {
    outcome: &'static str,
    error: Option<DeclaredErrorValue>,
    events: Vec<(&'static str, Vec<(&'static str, Node)>)>,
}

impl Answer {
    /// A branch that publishes `events`.
    fn took(outcome: &'static str, events: Vec<(&'static str, Vec<(&'static str, Node)>)>) -> Self {
        Self {
            outcome,
            error: None,
            events,
        }
    }

    /// A branch that carries the declared error `error` with `fields`, and publishes nothing.
    fn refused(
        outcome: &'static str,
        error: &str,
        fields: Vec<(&'static str, Node)>,
    ) -> Result<Self, TargetError> {
        let mut value = DeclaredErrorValue::new(named::<ErrorRef>(error)?);
        for (field, node) in fields {
            value = value.with(field, node);
        }
        Ok(Self {
            outcome,
            error: Some(value),
            events: Vec::new(),
        })
    }

    /// The model's `NotHeld`: no value of the scenario has this identity.
    fn not_held(outcome: &'static str) -> Result<Self, TargetError> {
        Self::refused(outcome, "swell.host.NotHeld", Vec::new())
    }

    /// The model's `Faulted`: `Err(HostError::Faulted)` from the agent `agent_id`.
    fn faulted(agent_id: &str) -> Result<Self, TargetError> {
        Self::refused(
            "faulted",
            "swell.host.Faulted",
            vec![("agent_id", values::text(agent_id))],
        )
    }

    /// `RunTerminated` of the agent `agent_id` for `termination`, as the branch the model
    /// names for it.
    fn terminated(agent_id: &str, termination: Termination) -> Self {
        let (outcome, reason) = match termination {
            Termination::StepBudget => ("step-budget", "StepBudget"),
            Termination::MemoryBudget => ("memory-budget", "MemoryBudget"),
            Termination::Interrupted => ("interrupted", "Interrupted"),
            Termination::EngineFault => ("engine-fault", "EngineFault"),
        };
        Self::took(
            outcome,
            vec![(
                "swell.host.RunTerminated",
                vec![
                    ("agent_id", values::text(agent_id)),
                    ("reason", values::text(reason)),
                ],
            )],
        )
    }
}

/// `name` as the reference type `T`, or an adapter failure.
fn named<T: FromStr>(name: &str) -> Result<T, TargetError>
where
    T::Err: std::fmt::Display,
{
    name.parse().map_err(|error: T::Err| {
        TargetError::unavailable(format!("naming `{name}`"), error.to_string())
    })
}

/// A call on the agent `agent_id` that the interface answered with a `HostError` the model has
/// no branch for; `value` names the realm or script of the call and the agent that made it.
fn host_error<T>(
    call: &str,
    agent_id: &str,
    value: (&str, &Held<T>),
    error: HostError,
) -> TargetError {
    let (id, held) = value;
    match error {
        HostError::ForeignHandle => TargetError::unsupported(
            format!(
                "`{call}` on the agent `{agent_id}` with `{id}`, made by the agent `{}`",
                held.agent
            ),
            "the interface answers Err(ForeignHandle) for a script of another agent (HOST-45, \
             HOST-48), and the model states no outcome for it (`UNMAPPED:` at \
             `swell.host.EvaluateScript`: a script of another agent than the one called)",
        ),
        other => unexpected(call, other),
    }
}

/// A call the interface answered with a `HostError` that no value the scenario holds explains.
fn unexpected(call: &str, error: HostError) -> TargetError {
    TargetError::unavailable(
        format!("`{call}`"),
        format!(
            "the interface answered Err({error:?}) for values the scenario holds, and the model \
             has no outcome for it"
        ),
    )
}

/// The slice that brings an entry point for a command of the model the engine does not have yet.
fn later_slice(command: &str) -> Option<&'static str> {
    Some(match command.strip_prefix("swell.host.")? {
        "ParseModule"
        | "LoadRequestedModules"
        | "FinishLoadingImportedModule"
        | "LinkModule"
        | "EvaluateModule"
        | "HostLoadImportedModule"
        | "AsyncModuleExecutionSettled" => "slice 11 (modules)",
        "HostEnqueueGenericJob" | "HostEnqueueTimeoutJob" => "slice 14 (generic and timeout jobs)",
        _ => return None,
    })
}

/// The hook a command the engine issues names, for the two hooks the target records.
fn recorded_hook(command: &str) -> Option<Hook> {
    match command {
        "swell.host.HostEnqueuePromiseJob" => Some(Hook::PromiseJob),
        "swell.host.HostPromiseRejectionTracker" => Some(Hook::RejectionTracker),
        _ => None,
    }
}

/// The swell host interface as a conformance target: a fresh set of agents per scenario.
pub struct SwellTarget {
    identity: ImplementationIdentity,
    interrupter: Interrupter,
    checkpoint: Checkpoint,
    scenario: RefCell<Option<Scenario>>,
}

impl SwellTarget {
    /// A target named `identity` that sets an interrupt flag with `interrupter` and performs a
    /// checkpoint with [`runs_the_queue`]. It runs agents on the calling thread, which must have
    /// [`STACK_BYTES`]; [`run`](crate::run) makes it.
    pub fn new(identity: ImplementationIdentity, interrupter: Interrupter) -> Self {
        Self {
            identity,
            interrupter,
            checkpoint: runs_the_queue,
            scenario: RefCell::new(None),
        }
    }

    /// The same target, performing a checkpoint with `checkpoint`.
    #[must_use]
    pub fn with_checkpoint(self, checkpoint: Checkpoint) -> Self {
        Self { checkpoint, ..self }
    }

    /// `f` on the open scenario.
    fn with<T>(
        &self,
        f: impl FnOnce(&mut Scenario) -> Result<T, TargetError>,
    ) -> Result<T, TargetError> {
        let mut open = self.scenario.borrow_mut();
        let scenario = open.as_mut().ok_or_else(|| {
            TargetError::unavailable("reaching the scenario", "no scenario is open")
        })?;
        f(scenario)
    }

    /// One command of the model, as one call of the interface.
    fn issue(
        &self,
        scenario: &mut Scenario,
        command: &str,
        input: &BTreeMap<String, Node>,
    ) -> Result<Answer, TargetError> {
        let mut input = Input::new(command, input);
        let answer = match command {
            "swell.host.CreateAgent" => create_agent(scenario, &mut input),
            "swell.host.CreateRealm" => create_realm(scenario, &mut input),
            "swell.host.ParseScript" => parse_script(scenario, &mut input),
            "swell.host.EvaluateScript" => self.evaluate_script(scenario, &mut input),
            "swell.host.PerformMicrotaskCheckpoint" => {
                self.perform_microtask_checkpoint(scenario, &mut input)
            }
            "swell.host.Interrupt" => self.interrupt(scenario, &mut input),
            "swell.host.Collect" => collect(scenario, &mut input),
            other => {
                return Err(TargetError::unsupported(
                    format!("invoking `{other}`"),
                    if let Some(slice) = later_slice(other) {
                        format!("the engine has no entry point for `{other}`; {slice} brings it")
                    } else if recorded_hook(other).is_some() {
                        format!("the engine issues `{other}` itself; a scenario awaits it")
                    } else {
                        format!("the model has no command `{other}`")
                    },
                ));
            }
        }?;
        input.finish()?;
        Ok(answer)
    }

    /// `PerformMicrotaskCheckpoint`: the target's [`Checkpoint`] on the agent, its `Outcome` as
    /// the branch the model names for it: `Normal` is `drained`, `Terminated(reason)` the branch
    /// of the reason with `RunTerminated`, `Err(Faulted)` is `faulted` (HOST-79, HOST-77). A
    /// forced `step-budget` or `memory-budget` on an empty queue, or a forced `interrupted` on an
    /// agent no `Interrupt` of the scenario was made on, gives the engine no means to reach it,
    /// and is unsupported before the call.
    fn perform_microtask_checkpoint(
        &self,
        scenario: &mut Scenario,
        input: &mut Input<'_>,
    ) -> Result<Answer, TargetError> {
        let agent_id = input.text("agent_id")?;
        let forced = scenario.take_checkpoint();
        let interrupted = scenario.interrupted.contains(&agent_id);
        let Some(agent) = scenario.agent(&agent_id) else {
            return Answer::not_held("no-agent");
        };
        match forced {
            Some(Forced::CheckpointBudget) if matches!(agent.pending_microtasks(), Ok(0)) => {
                return Err(TargetError::unsupported(
                    "`swell.host.PerformMicrotaskCheckpoint` forced to a budget on an empty queue",
                    "a checkpoint that finds the queue empty takes nothing off, spends no step and \
                     allocates nothing (HOST-193), and the scenario's scripts queued no job",
                ));
            }
            Some(Forced::CheckpointInterrupt) if !interrupted => {
                return Err(TargetError::unsupported(
                    "`swell.host.PerformMicrotaskCheckpoint` forced to `interrupted`",
                    "the target sets the interrupt handle only for the scenario's `Interrupt`, \
                     and none was made on this agent (HOST-198)",
                ));
            }
            _ => {}
        }
        match (self.checkpoint)(agent) {
            Err(HostError::Faulted) => Answer::faulted(&agent_id),
            Err(error) => Err(unexpected("swell.host.PerformMicrotaskCheckpoint", error)),
            Ok(Outcome::Normal(_)) => Ok(Answer::took(
                "drained",
                vec![(
                    "swell.host.MicrotaskCheckpointPerformed",
                    vec![("agent_id", values::text(&agent_id))],
                )],
            )),
            Ok(Outcome::Terminated(termination)) => Ok(Answer::terminated(&agent_id, termination)),
            Ok(Outcome::Throw(_)) => Err(TargetError::unavailable(
                "`swell.host.PerformMicrotaskCheckpoint`",
                "the checkpoint answered Throw, which it never answers: an entry's throw stays \
                 inside the entry",
            )),
        }
    }

    /// `EvaluateScript`: `Agent::evaluate_script`, its `Outcome` as the branch of the same name.
    /// A flag the scenario's `Interrupt` set ends the run at its start with
    /// `Terminated(Interrupted)`, and the engine clears it (HOST-92, HOST-91).
    fn evaluate_script(
        &self,
        scenario: &mut Scenario,
        input: &mut Input<'_>,
    ) -> Result<Answer, TargetError> {
        let agent_id = input.text("agent_id")?;
        let script_id = input.text("script_id")?;
        let Some(script) = scenario.scripts.remove(&script_id) else {
            return Answer::not_held("no-script");
        };
        let answered = match scenario.agent(&agent_id) {
            None => Answer::not_held("no-agent"),
            Some(agent) => match agent.evaluate_script(&script.value) {
                Err(HostError::Faulted) => Answer::faulted(&agent_id),
                Err(error) => Err(host_error(
                    "swell.host.EvaluateScript",
                    &agent_id,
                    (&script_id, &script),
                    error,
                )),
                Ok(Outcome::Normal(_)) => Ok(Answer::took(
                    "completed",
                    vec![(
                        "swell.host.ScriptCompleted",
                        vec![("script_id", values::text(&script_id))],
                    )],
                )),
                Ok(Outcome::Throw(_)) => Ok(Answer::took(
                    "threw",
                    vec![(
                        "swell.host.ScriptThrew",
                        vec![("script_id", values::text(&script_id))],
                    )],
                )),
                Ok(Outcome::Terminated(termination)) => {
                    Ok(Answer::terminated(&agent_id, termination))
                }
            },
        };
        scenario.scripts.insert(script_id, script);
        answered
    }

    /// `Interrupt`: the agent's interrupt handle, set with the target's [`Interrupter`]. A store
    /// that answers nothing, for an alive and a faulted agent alike.
    fn interrupt(
        &self,
        scenario: &mut Scenario,
        input: &mut Input<'_>,
    ) -> Result<Answer, TargetError> {
        let agent_id = input.text("agent_id")?;
        let Some(agent) = scenario.agent(&agent_id) else {
            return Answer::not_held("no-agent");
        };
        (self.interrupter)(&agent.interrupt_handle());
        scenario.interrupted.insert(agent_id);
        Ok(Answer::took("set", Vec::new()))
    }

    /// The answer to an await of `hook` on the agent `agent_id`: the oldest call of the hook the
    /// agent's host recorded and no await answered, as the branch, the events and the input the
    /// model states for it. `unsupported` when the scenario holds no such agent or the engine
    /// made no such call (module documentation).
    fn awaited(
        scenario: &mut Scenario,
        command: &str,
        hook: Hook,
        agent_id: &str,
    ) -> Result<(Answer, BTreeMap<String, Node>), TargetError> {
        let awaiting = || format!("awaiting `{command}` on the agent `{agent_id}`");
        let Some(agent) = scenario.agents.get_mut(agent_id) else {
            return Err(TargetError::unsupported(
                awaiting(),
                "the scenario holds no agent of this identity, so no engine issues the command \
                 on it",
            ));
        };
        let Some(call) = agent.host_mut().take(hook) else {
            return Err(TargetError::unsupported(
                awaiting(),
                format!(
                    "the engine made no call of the hook `{command}` names on this agent that an \
                     earlier await did not answer: it calls the hook only inside a command, for a \
                     promise a script made, and the scenario's commands made none"
                ),
            ));
        };
        let mut input = BTreeMap::new();
        input.insert("agent_id".to_owned(), values::text(agent_id));
        let answer = match call {
            Issued::RejectionTracker(operation) => {
                let operation = values::text(match operation {
                    RejectionOperation::Reject => "reject",
                    RejectionOperation::Handle => "handle",
                });
                input.insert("operation".to_owned(), operation.clone());
                Answer::took(
                    "tracked",
                    vec![(
                        "swell.host.PromiseRejectionTracked",
                        vec![
                            ("agent_id", values::text(agent_id)),
                            ("operation", operation),
                        ],
                    )],
                )
            }
            Issued::PromiseJob(realm) => {
                let realm_id = match realm {
                    None => None,
                    Some(realm) => Some(realm_id_of(scenario, agent_id, realm)?),
                };
                let job_id = scenario.identity(4);
                let mut payload = vec![
                    ("job_id", values::text(&job_id)),
                    ("agent_id", values::text(agent_id)),
                ];
                if let Some(realm_id) = realm_id {
                    input.insert("realm_id".to_owned(), values::text(&realm_id));
                    payload.push(("realm_id", values::text(&realm_id)));
                }
                Answer::took("enqueued", vec![("swell.host.PromiseJobEnqueued", payload)])
            }
        };
        Ok((answer, input))
    }
}

/// The identity of the realm of the agent `agent_id` that `realm`, a realm a hook was handed,
/// is: each realm the scenario holds of that agent is read as a `RealmRef` through
/// `Agent::with_cx` and compared. A realm the scenario does not hold is an adapter failure: the
/// engine hands a hook only realms of the agent.
fn realm_id_of(
    scenario: &mut Scenario,
    agent_id: &str,
    realm: RealmRef,
) -> Result<String, TargetError> {
    let reading = "reading the realm of `swell.host.HostEnqueuePromiseJob`";
    let Some(agent) = scenario.agents.get_mut(agent_id) else {
        return Err(TargetError::unavailable(reading, "the agent is not held"));
    };
    for (realm_id, held) in &scenario.realms {
        if held.agent != agent_id {
            continue;
        }
        let read = agent
            .with_cx(&held.value, |cx| cx.realm_ref(&held.value))
            .and_then(|read| read)
            .map_err(|error| unexpected(reading, error))?;
        if read == realm {
            return Ok(realm_id.clone());
        }
    }
    Err(TargetError::unavailable(
        reading,
        "the hook was handed a realm the scenario holds no identity for",
    ))
}

/// `CreateAgent`: `Agent::new` with the limits and the seeds of the input.
fn create_agent(scenario: &mut Scenario, input: &mut Input<'_>) -> Result<Answer, TargetError> {
    let limits = input.limits("limits")?;
    let hash_key = input.bytes("hash_key")?;
    let random_seed = input.bytes("random_seed")?;
    let seeds = values::seeds(&hash_key, &random_seed)?;
    let needed = limits.min_stack_bytes();
    if needed > STACK_BYTES {
        return Err(TargetError::unsupported(
            "`swell.host.CreateAgent` with these limits",
            format!(
                "the limits need a thread stack of {needed} bytes (Limits::min_stack_bytes, \
                 HOST-17) and the target runs on one of {STACK_BYTES}"
            ),
        ));
    }
    let agent = Agent::new(ConformanceHost::default(), limits, seeds).map_err(|error| {
        TargetError::unavailable("`swell.host.CreateAgent`", format!("{error:?}"))
    })?;
    let agent_id = scenario.identity(1);
    scenario.agents.insert(agent_id.clone(), agent);
    Ok(Answer::took(
        "created",
        vec![(
            "swell.host.AgentCreated",
            vec![("agent_id", values::text(&agent_id))],
        )],
    ))
}

/// `CreateRealm`: `Agent::create_realm` with no global setup and no [[HostDefined]].
fn create_realm(scenario: &mut Scenario, input: &mut Input<'_>) -> Result<Answer, TargetError> {
    let agent_id = input.text("agent_id")?;
    let Some(agent) = scenario.agent(&agent_id) else {
        return Answer::not_held("no-agent");
    };
    let realm = match agent.create_realm(None, None) {
        Err(HostError::Faulted) => return Answer::faulted(&agent_id),
        Err(error) => return Err(unexpected("swell.host.CreateRealm", error)),
        Ok(Outcome::Normal(realm)) => realm,
        Ok(Outcome::Throw(_)) => {
            return Err(TargetError::unsupported(
                "`swell.host.CreateRealm` that throws",
                "create_realm answered Throw, and the model has no outcome for it (HOST-41)",
            ));
        }
        Ok(Outcome::Terminated(termination)) => {
            return Err(TargetError::unsupported(
                format!("`swell.host.CreateRealm` that ends in Terminated({termination:?})"),
                "the model has no outcome for a create_realm that ends this way (HOST-12, \
                 HOST-71)",
            ));
        }
    };
    let realm_id = scenario.identity(2);
    scenario.realms.insert(
        realm_id.clone(),
        Held {
            agent: agent_id.clone(),
            value: realm,
        },
    );
    Ok(Answer::took(
        "created",
        vec![(
            "swell.host.RealmCreated",
            vec![
                ("realm_id", values::text(&realm_id)),
                ("agent_id", values::text(&agent_id)),
            ],
        )],
    ))
}

/// `ParseScript`: `Agent::parse_script` of the source text, as UTF-8, in the realm the model
/// names, on the agent the model names.
fn parse_script(scenario: &mut Scenario, input: &mut Input<'_>) -> Result<Answer, TargetError> {
    let agent_id = input.text("agent_id")?;
    let realm_id = input.text("realm_id")?;
    let source_text = input.text("source_text")?;
    if !scenario.agents.contains_key(&agent_id) {
        return Answer::not_held("no-agent");
    }
    let Some(realm) = scenario.realms.remove(&realm_id) else {
        return Answer::not_held("no-realm");
    };
    let parsed = match scenario.agent(&agent_id) {
        Some(agent) => parse_in(agent, &agent_id, (&realm_id, &realm), &source_text),
        None => Answer::not_held("no-agent").map(Err),
    };
    scenario.realms.insert(realm_id.clone(), realm);
    let script = match parsed? {
        Ok(script) => script,
        Err(answer) => return Ok(answer),
    };
    let script_id = scenario.identity(3);
    scenario.scripts.insert(
        script_id.clone(),
        Held {
            agent: agent_id,
            value: script,
        },
    );
    Ok(Answer::took(
        "parsed",
        vec![(
            "swell.host.ScriptParsed",
            vec![
                ("script_id", values::text(&script_id)),
                ("realm_id", values::text(&realm_id)),
            ],
        )],
    ))
}

/// The parse of `source` on `agent` in `realm`: the `Script`, or the refusing branch it takes.
///
/// A realm of another agent is the model's `foreign-realm`, `Err(ForeignHandle)` (HOST-45),
/// after `faulted` as the engine checks the agent first (HOST-48).
///
/// A list of `SyntaxError`s is the model's `parse-failed` when each is an error of ECMA-262 or
/// the nesting limit (`ParseScript`'s `external:` names both). The parser of this slice also
/// refuses a construct a later slice brings, with a `SyntaxError` whose message says so
/// (`swell_syntax::MessageKind::Refusal`); the source is then a Script all the same, so the
/// refusal is `unsupported`, never `parse-failed`, as the test262 runner never counts it as the
/// `SyntaxError` a case asks for. The kind is read from each error's `message` in `realm`.
fn parse_in(
    agent: &mut Agent<ConformanceHost>,
    agent_id: &str,
    realm: (&str, &Held<Realm>),
    source: &str,
) -> Result<Result<Script, Answer>, TargetError> {
    let (_, held) = realm;
    let errors = match agent.parse_script(&held.value, Source::Utf8(source), None) {
        Err(HostError::Faulted) => return Answer::faulted(agent_id).map(Err),
        Err(HostError::ForeignHandle) => {
            return Answer::refused("foreign-realm", "swell.host.ForeignHandle", Vec::new())
                .map(Err);
        }
        Err(error) => {
            return Err(host_error("swell.host.ParseScript", agent_id, realm, error));
        }
        Ok(Parsed::Parsed(script)) => return Ok(Ok(script)),
        Ok(Parsed::Terminated(Termination::MemoryBudget)) => {
            return Answer::refused(
                "memory-budget",
                "swell.host.Terminated",
                vec![("reason", values::text("MemoryBudget"))],
            )
            .map(Err);
        }
        Ok(Parsed::Terminated(termination)) => {
            return Err(TargetError::unsupported(
                format!("`swell.host.ParseScript` that ends in Terminated({termination:?})"),
                "the model has no outcome for it: an engine fault while parsing is `UNMAPPED:` \
                 (HOST-71), and a parse ended by the interrupt flag at its start (HOST-166) has \
                 no outcome of ParseScript",
            ));
        }
        Ok(Parsed::Errors(errors)) => errors,
    };
    let messages = messages(agent, &held.value, &errors)?;
    let refusal = messages.iter().find(|message| {
        matches!(
            message_kind(without_position(message)),
            MessageKind::Refusal { .. }
        )
    });
    if let Some(refusal) = refusal {
        return Err(TargetError::unsupported(
            format!("`swell.host.ParseScript` of {source:?}"),
            format!(
                "the parser of slice 1 refuses a construct of a later slice: \"{refusal}\"; the \
                 source is a Script of ECMA-262, so this is not the model's parse-failed"
            ),
        ));
    }
    Answer::refused("parse-failed", "swell.host.ParseFailed", Vec::new()).map(Err)
}

/// The `message` of each error object in `errors`, read in `realm`.
fn messages(
    agent: &mut Agent<ConformanceHost>,
    realm: &Realm,
    errors: &[Root],
) -> Result<Vec<String>, TargetError> {
    let reading = "reading the SyntaxErrors of `swell.host.ParseScript`";
    agent
        .with_cx(realm, |cx| {
            errors
                .iter()
                .map(|error| message(cx, error))
                .collect::<Option<Vec<String>>>()
        })
        .map_err(|error| unexpected(reading, error))?
        .ok_or_else(|| TargetError::unavailable(reading, "an error has no String `message`"))
}

/// The `message` of the error object `error` holds, when it is a String.
fn message(cx: &mut Cx<'_, ConformanceHost>, error: &Root) -> Option<String> {
    let Value::Object(object) = cx.get(error).ok()? else {
        return None;
    };
    let key = PropertyKey::String(cx.new_string("message").ok()?);
    let Value::String(text) = cx.get_property(object, key).ok()?.ok()? else {
        return None;
    };
    cx.to_rust_string_lossy(text).ok()
}

/// `text` without the ` (<line>:<column>)` the engine adds to a parse error's message.
fn without_position(text: &str) -> &str {
    let Some((message, position)) = text.rsplit_once(" (") else {
        return text;
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    let is_position = position
        .strip_suffix(')')
        .and_then(|p| p.split_once(':'))
        .is_some_and(|(line, column)| digits(line) && digits(column));
    if is_position { message } else { text }
}

/// `Collect`: `Agent::collect`. A forced `engine-fault` sets the fault switch at `collect`
/// first (`Agent::inject_fault`, HOST-71).
fn collect(scenario: &mut Scenario, input: &mut Input<'_>) -> Result<Answer, TargetError> {
    let agent_id = input.text("agent_id")?;
    let fault = scenario.take(Forced::CollectFault);
    let Some(agent) = scenario.agent(&agent_id) else {
        return Answer::not_held("no-agent");
    };
    if fault {
        agent.inject_fault(FaultPoint::Collect);
    }
    match agent.collect() {
        Err(HostError::Faulted) => Answer::faulted(&agent_id),
        Err(error) => Err(unexpected("swell.host.Collect", error)),
        Ok(Outcome::Normal(_)) => Ok(Answer::took("collected", Vec::new())),
        Ok(Outcome::Terminated(Termination::EngineFault)) => {
            Ok(Answer::terminated(&agent_id, Termination::EngineFault))
        }
        Ok(other) => Err(TargetError::unavailable(
            "`swell.host.Collect`",
            format!("collect answered {other:?}, which decision 18 rules out"),
        )),
    }
}

/// The event `name` with `payload`, as observed in the scenario `correlation` at `sequence`.
fn observed(
    name: &str,
    payload: Vec<(&'static str, Node)>,
    correlation: &CorrelationId,
    sequence: u64,
) -> Result<ObservedEvent, TargetError> {
    let mut event = ObservedEvent::new(named::<EventRef>(name)?);
    for (field, node) in payload {
        event = event.with(field, node);
    }
    Ok(event.in_activity(correlation.clone()).at(sequence))
}

/// `answer` of `command` as the runner's result, its events published in the scenario
/// `correlation`, after every event the scenario published before.
fn published(
    scenario: &mut Scenario,
    command: &str,
    answer: Answer,
    correlation: &CorrelationId,
) -> Result<SemanticCommandResult, TargetError> {
    let outcome = OutcomeName::new(answer.outcome).map_err(|error| {
        TargetError::unavailable(format!("naming `{}`", answer.outcome), error.to_string())
    })?;
    let mut result =
        SemanticCommandResult::took(OutcomeRef::new(named::<CommandRef>(command)?, outcome));
    result.error = answer.error;
    for (name, payload) in answer.events {
        let sequence = u64::try_from(scenario.published.len()).unwrap_or(u64::MAX);
        let event = observed(name, payload, correlation, sequence)?;
        scenario.published.push(event.clone());
        result = result.emitting(event);
    }
    Ok(result)
}

/// The agent an await names: its `agent_id` input where the runner states one, else the
/// awaited instance.
fn awaited_agent(request: &AwaitRequest) -> Option<String> {
    match request
        .compared
        .get("agent_id")
        .unwrap_or(&request.instance)
    {
        Node::Text(agent_id) => Some(agent_id.clone()),
        _ => None,
    }
}

impl ConformanceTarget for SwellTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(self.identity.clone())
    }

    fn begin_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = Some(Scenario::default());
        Ok(())
    }

    fn end_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = None;
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        self.with(|scenario| {
            let command = request.command.to_string();
            let answer = self.issue(scenario, &command, &request.input)?;
            published(scenario, &command, answer, &request.correlation)
        })
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Err(TargetError::unsupported(
            format!("reading `{}`", request.view),
            "the model of the host interface declares no view",
        ))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        self.with(|scenario| {
            Ok(scenario
                .published
                .iter()
                .filter(|event| event.event == request.event)
                .cloned()
                .collect())
        })
    }

    /// Accepts a forced outcome the call reaches for real, arranges `Collect`'s `engine-fault`
    /// and keeps a forced outcome of `PerformMicrotaskCheckpoint` for the check of its call;
    /// every other one is unsupported (module documentation).
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        let command = request.force.command.to_string();
        let outcome = request.force.outcome.to_string();
        let forced = match (command.as_str(), outcome.as_str()) {
            (
                "swell.host.EvaluateScript",
                "interrupted" | "step-budget" | "memory-budget" | "threw",
            )
            | ("swell.host.ParseScript", "parse-failed") => return Ok(()),
            ("swell.host.Collect", "engine-fault") => Forced::CollectFault,
            ("swell.host.PerformMicrotaskCheckpoint", "step-budget" | "memory-budget") => {
                Forced::CheckpointBudget
            }
            ("swell.host.PerformMicrotaskCheckpoint", "interrupted") => Forced::CheckpointInterrupt,
            _ => {
                let why = if let Some(slice) = later_slice(&command) {
                    format!("the engine has no entry point for `{command}`; {slice} brings it")
                } else if outcome == "engine-fault"
                    && matches!(
                        command.as_str(),
                        "swell.host.EvaluateScript" | "swell.host.PerformMicrotaskCheckpoint"
                    )
                {
                    "the fault switch of the interface has no point at evaluate_script or \
                     perform_microtask_checkpoint (FaultPoint: ParseScript, CreateRealm, \
                     Collect; HOST-71)"
                        .to_owned()
                } else if command == "swell.host.ParseScript" && outcome == "memory-budget" {
                    "a parse passes the memory budget only with limits and a source that reach \
                     it, and the target runs the scenario's own; the model states this answer \
                     only as forced, with no requirement of its own"
                        .to_owned()
                } else {
                    format!("the target has no means to reach `{outcome}` of `{command}`")
                };
                return Err(TargetError::unsupported(
                    format!("forcing `{}`", request.force),
                    why,
                ));
            }
        };
        self.with(|scenario| {
            scenario.forced = Some(forced);
            Ok(())
        })
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("delivering `{}` again", request.event),
            "the model of the host interface declares no binding",
        ))
    }

    /// Answers an await of `HostEnqueuePromiseJob` or `HostPromiseRejectionTracker` from the
    /// calls the agent's host recorded, in their order; every other await is unsupported
    /// (module documentation).
    fn await_outcome(&self, request: AwaitRequest) -> Result<AwaitAnswer, TargetError> {
        let command = request.command.to_string();
        let Some(hook) = recorded_hook(&command) else {
            return Err(TargetError::unsupported(
                format!("awaiting `{command}` on an instance"),
                match later_slice(&command) {
                    Some(slice) => format!("the engine never issues `{command}` yet; {slice}"),
                    None => format!("the model has no command `{command}` the engine issues"),
                },
            ));
        };
        let Some(agent_id) = awaited_agent(&request) else {
            return Err(TargetError::unavailable(
                format!("awaiting `{command}`"),
                "the await names no agent by its identity",
            ));
        };
        self.with(|scenario| {
            let (answer, input) = Self::awaited(scenario, &command, hook, &agent_id)?;
            let result = published(scenario, &command, answer, &request.correlation)?;
            Ok(AwaitAnswer::Reached {
                result,
                input,
                elapsed_ms: None,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ess_primitives::facts::Number;
    use ess_primitives::node::Node;

    use ess_conformance::target::{ImplementationIdentity, TargetError};
    use swell::{Agent, CheckpointReport, HostError, InterruptHandle, Outcome};

    use super::{
        Answer, ConformanceHost, Forced, Hook, Interrupter, STACK_BYTES, Scenario, SwellTarget,
        create_agent, create_realm, parse_script, sets_the_flag, values,
    };
    use crate::values::Input;

    fn limits() -> Node {
        let mut fields = BTreeMap::new();
        for (name, value) in [
            ("steps_per_entry", 1_000_000_i64),
            ("memory_bytes", 67_108_864),
            ("frame_depth", 10_000),
            ("reentry_depth", 256),
            ("parser_nesting", 1_000),
            ("string_length", 1_073_741_824),
            ("bigint_bits", 1_048_576),
        ] {
            fields.insert(name.to_owned(), Node::Number(Number::from(value)));
        }
        Node::Map(fields)
    }

    fn create_agent_input(limits: Node) -> BTreeMap<String, Node> {
        let mut input = BTreeMap::new();
        input.insert("limits".to_owned(), limits);
        input.insert("hash_key".to_owned(), values::text("Zw=="));
        input.insert("random_seed".to_owned(), values::text("Zw=="));
        input
    }

    fn created(answer: &Answer) -> bool {
        answer.outcome == "created" && answer.error.is_none() && answer.events.len() == 1
    }

    /// The limits a `CreateAgent` input of the suite gives are read as exactly these limits, and
    /// the stack they need by `Limits::min_stack_bytes()` fits [`STACK_BYTES`]. The need is the
    /// engine's own answer, not a number held here, so a measured change of its constants does
    /// not break this test; a need beyond the thread does.
    #[test]
    fn the_suites_limits_fit_the_targets_stack() {
        let fields = create_agent_input(limits());
        let mut input = Input::new("swell.host.CreateAgent", &fields);
        let read = input.limits("limits").map_err(|error| error.to_string());
        let suite = swell::Limits {
            steps_per_entry: 1_000_000,
            memory_bytes: 67_108_864,
            frame_depth: 10_000,
            reentry_depth: 256,
            parser_nesting: 1_000,
            string_length: 1_073_741_824,
            bigint_bits: 1_048_576,
        };
        assert_eq!(read, Ok(suite));
        let needed = suite.min_stack_bytes();
        assert!(needed <= STACK_BYTES, "{needed} > {STACK_BYTES}");
    }

    #[test]
    fn an_agent_is_created_and_named_by_a_new_identity() {
        let mut scenario = Scenario::default();
        let fields = create_agent_input(limits());
        let mut input = Input::new("swell.host.CreateAgent", &fields);
        let answer = create_agent(&mut scenario, &mut input);
        assert!(answer.is_ok_and(|answer| created(&answer)));
        assert!(input.finish().is_ok());
        assert_eq!(
            scenario.agents.keys().collect::<Vec<_>>(),
            ["00000001-0000-4000-8000-000000000001"]
        );
    }

    const AGENT: &str = "00000001-0000-4000-8000-000000000001";
    const REALM: &str = "00000002-0000-4000-8000-000000000002";

    fn fields(pairs: &[(&str, &str)]) -> BTreeMap<String, Node> {
        pairs
            .iter()
            .map(|(name, value)| ((*name).to_owned(), values::text(value)))
            .collect()
    }

    /// A scenario with the agent `AGENT` and its realm `REALM`.
    fn agent_and_realm() -> Scenario {
        let mut scenario = Scenario::default();
        let input = create_agent_input(limits());
        let answer = create_agent(&mut scenario, &mut Input::new("CreateAgent", &input));
        assert!(answer.is_ok_and(|answer| created(&answer)));
        let input = fields(&[("agent_id", AGENT)]);
        let answer = create_realm(&mut scenario, &mut Input::new("CreateRealm", &input));
        assert!(answer.is_ok_and(|answer| created(&answer)));
        scenario
    }

    /// The branch and the error `ParseScript` of `source` answers on `AGENT` in `REALM`.
    fn parse(
        scenario: &mut Scenario,
        source: &str,
    ) -> Result<(&'static str, Option<String>), TargetError> {
        let input = fields(&[
            ("agent_id", AGENT),
            ("realm_id", REALM),
            ("source_text", source),
        ]);
        parse_script(scenario, &mut Input::new("ParseScript", &input)).map(|answer| {
            (
                answer.outcome,
                answer.error.map(|error| error.error.to_string()),
            )
        })
    }

    #[test]
    fn a_construct_the_parser_refuses_until_a_later_slice_is_unsupported_not_parse_failed() {
        let mut scenario = agent_and_realm();
        let answer = parse(&mut scenario, "<!-- an HTML-like comment, Annex B");
        assert!(
            answer.as_ref().is_err_and(|error| error.is_unsupported()
                && error
                    .to_string()
                    .contains("HTML-like comment is not supported before slice 18")),
            "{answer:?}"
        );
    }

    #[test]
    fn a_grammar_error_is_parse_failed() {
        let mut scenario = agent_and_realm();
        let answer = parse(&mut scenario, ")").map_err(|error| error.to_string());
        assert_eq!(
            answer,
            Ok(("parse-failed", Some("swell.host.ParseFailed".to_owned())))
        );
    }

    #[test]
    fn a_realm_of_another_agent_is_foreign_realm() {
        let mut scenario = agent_and_realm();
        let input = create_agent_input(limits());
        let answer = create_agent(&mut scenario, &mut Input::new("CreateAgent", &input));
        assert!(answer.is_ok_and(|answer| created(&answer)));
        let other = "00000001-0000-4000-8000-000000000003";
        let input = fields(&[
            ("agent_id", other),
            ("realm_id", REALM),
            ("source_text", "1 + 1;"),
        ]);
        let answer = parse_script(&mut scenario, &mut Input::new("ParseScript", &input))
            .map(|answer| {
                (
                    answer.outcome,
                    answer.error.map(|error| error.error.to_string()),
                )
            })
            .map_err(|error| error.to_string());
        assert_eq!(
            answer,
            Ok(("foreign-realm", Some("swell.host.ForeignHandle".to_owned())))
        );
    }

    /// The branches of `Interrupt` on `AGENT` and then of two runs of `1 + 1;`, over a target
    /// that sets the interrupt flag with `interrupter`.
    fn interrupt_then_run_twice(interrupter: Interrupter) -> Result<[&'static str; 3], String> {
        let mut scenario = agent_and_realm();
        assert_eq!(
            parse(&mut scenario, "1 + 1;").map_err(|error| error.to_string()),
            Ok(("parsed", None))
        );
        let target = SwellTarget::new(ImplementationIdentity::new("swell", "test"), interrupter);
        let agent = fields(&[("agent_id", AGENT)]);
        let run = fields(&[
            ("agent_id", AGENT),
            ("script_id", "00000003-0000-4000-8000-000000000003"),
        ]);
        let evaluate = |scenario: &mut Scenario| {
            target
                .evaluate_script(scenario, &mut Input::new("EvaluateScript", &run))
                .map(|answer| answer.outcome)
                .map_err(|error| error.to_string())
        };
        let set = target
            .interrupt(&mut scenario, &mut Input::new("Interrupt", &agent))
            .map(|answer| answer.outcome)
            .map_err(|error| error.to_string())?;
        let first = evaluate(&mut scenario)?;
        let second = evaluate(&mut scenario)?;
        Ok([set, first, second])
    }

    #[test]
    fn an_interrupt_ends_the_next_run_and_is_cleared_by_it() {
        assert_eq!(
            interrupt_then_run_twice(sets_the_flag),
            Ok(["set", "interrupted", "completed"])
        );
        fn leaves_the_flag_unset(_handle: &InterruptHandle) {}
        assert_eq!(
            interrupt_then_run_twice(leaves_the_flag_unset),
            Ok(["set", "completed", "completed"])
        );
    }

    #[test]
    fn a_limit_outside_its_rust_type_is_unsupported() {
        let Node::Map(mut fields) = limits() else {
            unreachable!()
        };
        fields.insert(
            "frame_depth".to_owned(),
            Node::Number(Number::from(1_i64 << 40)),
        );
        let input_fields = create_agent_input(Node::Map(fields));
        let mut input = Input::new("swell.host.CreateAgent", &input_fields);
        let answer = create_agent(&mut Scenario::default(), &mut input);
        assert!(answer.is_err_and(|error| error.is_unsupported()));
    }

    #[test]
    fn a_forced_outcome_is_taken_by_its_own_command_once() {
        let mut scenario = Scenario {
            forced: Some(Forced::CollectFault),
            ..Scenario::default()
        };
        assert!(scenario.take(Forced::CollectFault));
        assert!(!scenario.take(Forced::CollectFault));
        assert_eq!(scenario.take_checkpoint(), None);
        scenario.forced = Some(Forced::CheckpointInterrupt);
        assert!(!scenario.take(Forced::CollectFault));
        assert_eq!(
            scenario.take_checkpoint(),
            Some(Forced::CheckpointInterrupt)
        );
        assert_eq!(scenario.take_checkpoint(), None);
    }

    fn target() -> SwellTarget {
        SwellTarget::new(ImplementationIdentity::new("swell", "test"), sets_the_flag)
    }

    /// The branch `EvaluateScript` of `source`, parsed on `AGENT` in `REALM`, answers.
    fn run(scenario: &mut Scenario, source: &str) -> Result<&'static str, String> {
        let parsed = parse(scenario, source).map_err(|error| error.to_string())?;
        assert_eq!(parsed, ("parsed", None));
        let script_id = format!("00000003-0000-4000-8000-{:012x}", scenario.made);
        let input = fields(&[("agent_id", AGENT), ("script_id", &script_id)]);
        target()
            .evaluate_script(scenario, &mut Input::new("EvaluateScript", &input))
            .map(|answer| answer.outcome)
            .map_err(|error| error.to_string())
    }

    /// The branch `PerformMicrotaskCheckpoint` on `AGENT` answers over `target`.
    fn checkpoint(target: &SwellTarget, scenario: &mut Scenario) -> Result<&'static str, String> {
        let input = fields(&[("agent_id", AGENT)]);
        target
            .perform_microtask_checkpoint(scenario, &mut Input::new("Checkpoint", &input))
            .map(|answer| answer.outcome)
            .map_err(|error| error.to_string())
    }

    fn pending(scenario: &mut Scenario) -> Option<usize> {
        scenario.agent(AGENT)?.pending_microtasks().ok()
    }

    /// HOST-79 through the target: the job a script queued stays queued until the checkpoint,
    /// which runs it and answers `drained` with the queue empty; a checkpoint that runs no job
    /// answers `drained` too, with the job still queued, which only a later read can tell.
    #[test]
    fn the_checkpoint_runs_the_queued_job_and_answers_drained() {
        let mut scenario = agent_and_realm();
        let source = "var ran = false; Promise.resolve().then(function () { ran = true; });";
        assert_eq!(run(&mut scenario, source), Ok("completed"));
        assert_eq!(pending(&mut scenario), Some(1));
        assert_eq!(checkpoint(&target(), &mut scenario), Ok("drained"));
        assert_eq!(pending(&mut scenario), Some(0));
        assert_eq!(
            run(&mut scenario, "if (!ran) { throw 0; }"),
            Ok("completed")
        );

        fn runs_no_job(
            _agent: &mut Agent<ConformanceHost>,
        ) -> Result<Outcome<CheckpointReport>, HostError> {
            Ok(Outcome::Normal(CheckpointReport {
                promise_jobs: 0,
                host_microtasks: 0,
            }))
        }
        let mut scenario = agent_and_realm();
        assert_eq!(run(&mut scenario, source), Ok("completed"));
        let planted = target().with_checkpoint(runs_no_job);
        assert_eq!(checkpoint(&planted, &mut scenario), Ok("drained"));
        assert_eq!(pending(&mut scenario), Some(1));
        assert_eq!(run(&mut scenario, "if (!ran) { throw 0; }"), Ok("threw"));
    }

    /// A forced budget on an empty queue and a forced `interrupted` with no `Interrupt` have no
    /// means to be reached; with the `Interrupt` the checkpoint reaches `interrupted` for real.
    #[test]
    fn a_forced_checkpoint_outcome_needs_the_means_to_reach_it() {
        let mut scenario = agent_and_realm();
        scenario.forced = Some(Forced::CheckpointBudget);
        let answer = checkpoint(&target(), &mut scenario);
        assert!(
            answer
                .as_ref()
                .is_err_and(|error| error.contains("empty queue")),
            "{answer:?}"
        );
        scenario.forced = Some(Forced::CheckpointInterrupt);
        let answer = checkpoint(&target(), &mut scenario);
        assert!(
            answer
                .as_ref()
                .is_err_and(|error| error.contains("Interrupt")),
            "{answer:?}"
        );
        let agent = fields(&[("agent_id", AGENT)]);
        let set = target()
            .interrupt(&mut scenario, &mut Input::new("Interrupt", &agent))
            .map(|answer| answer.outcome)
            .map_err(|error| error.to_string());
        assert_eq!(set, Ok("set"));
        scenario.forced = Some(Forced::CheckpointInterrupt);
        assert_eq!(checkpoint(&target(), &mut scenario), Ok("interrupted"));
        assert_eq!(checkpoint(&target(), &mut scenario), Ok("drained"));
    }

    /// The branch, the input and the event payload an await of `hook` on `AGENT` answers.
    type Awaited = (
        &'static str,
        BTreeMap<String, Node>,
        Vec<(&'static str, Node)>,
    );

    fn awaited(scenario: &mut Scenario, command: &str, hook: Hook) -> Result<Awaited, String> {
        SwellTarget::awaited(scenario, command, hook, AGENT)
            .map(|(mut answer, input)| {
                let payload = answer
                    .events
                    .pop()
                    .map(|(_, payload)| payload)
                    .unwrap_or_default();
                (answer.outcome, input, payload)
            })
            .map_err(|error| error.to_string())
    }

    /// HOST-121 and HOST-83 through the target: each hook call is answered once, in the order the
    /// engine made it, with its operation or the identity of its realm; one more await of a hook
    /// the engine did not call again is unsupported.
    #[test]
    fn an_await_answers_the_hook_calls_in_their_order() {
        let mut scenario = agent_and_realm();
        assert_eq!(
            run(&mut scenario, "var p = Promise.reject(1);"),
            Ok("completed")
        );
        assert_eq!(
            run(&mut scenario, "p.catch(function () {});"),
            Ok("completed")
        );
        let tracker = "swell.host.HostPromiseRejectionTracker";
        let mut operations = Vec::new();
        for _ in 0..2 {
            let (outcome, input, payload) =
                awaited(&mut scenario, tracker, Hook::RejectionTracker).unwrap_or_default();
            assert_eq!(outcome, "tracked");
            assert_eq!(input.get("agent_id"), Some(&values::text(AGENT)));
            assert_eq!(payload.len(), 2);
            operations.push(input.get("operation").cloned());
        }
        assert_eq!(
            operations,
            [Some(values::text("reject")), Some(values::text("handle"))]
        );
        let again = awaited(&mut scenario, tracker, Hook::RejectionTracker);
        assert!(
            again
                .as_ref()
                .is_err_and(|error| error.contains("made no call")),
            "{again:?}"
        );

        let job = "swell.host.HostEnqueuePromiseJob";
        let (outcome, input, payload) =
            awaited(&mut scenario, job, Hook::PromiseJob).unwrap_or_default();
        assert_eq!(outcome, "enqueued");
        assert_eq!(input.get("realm_id"), Some(&values::text(REALM)));
        assert_eq!(
            payload.iter().map(|(field, _)| *field).collect::<Vec<_>>(),
            ["job_id", "agent_id", "realm_id"]
        );
        assert!(awaited(&mut scenario, job, Hook::PromiseJob).is_err());
    }
}
