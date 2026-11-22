#![forbid(unsafe_code)]
//! The host interface of swell as a [`ConformanceTarget`]: one function per command of
//! `spec/domains/host.yaml` that slice 1 has an entry point for, each turning the model's input
//! into one call of the interface and the interface's answer into the model's outcome, error and
//! events.
//!
//! | command | the call | outcomes the call answers |
//! |---|---|---|
//! | `CreateAgent` | `Agent::new(host, limits, seeds)` | `created` |
//! | `CreateRealm` | `Agent::create_realm(None, None)` | `created`, `faulted` |
//! | `ParseScript` | `Agent::parse_script(realm, Source::Utf8(text), None)` | `parsed`, `parse-failed`, `memory-budget`, `foreign-realm`, `faulted` |
//! | `EvaluateScript` | `Agent::evaluate_script(script)` | `completed`, `threw`, `step-budget`, `memory-budget`, `interrupted`, `engine-fault`, `faulted` |
//! | `Interrupt` | `InterruptHandle::interrupt` on `Agent::interrupt_handle()` | `set` |
//! | `Collect` | `Agent::collect()` | `collected`, `engine-fault`, `faulted` |
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
//! - the twelve commands without an entry point in slice 1: the module commands (slice 11),
//!   `PerformMicrotaskCheckpoint` and the hooks the engine issues (slices 10 and 11);
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
//! `Interrupt` before the run; HOST-92). `Collect`'s `engine-fault` is reached through the
//! interface's own test switch, the feature `fault-injection` (`Agent::inject_fault`, HOST-71).
//! Unsupported stay `ParseScript`'s `memory-budget` (the target makes no limits and no source
//! of its own, and no scenario's parse passes its budget) and `EvaluateScript`'s
//! `engine-fault` (the fault switch has no point at `evaluate_script`).
//!
//! How the `Interrupt` command sets the flag is the target's [`Interrupter`]:
//! [`sets_the_flag`] in the target of [`shipped`](crate::shipped), a function that does nothing
//! in the planted-defect test.
//!
//! Every agent runs on the thread [`run`](crate::run) makes, of [`STACK_BYTES`]; an agent whose
//! `Limits::min_stack_bytes()` is more is not created (HOST-17).

use std::cell::RefCell;
use std::collections::BTreeMap;
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
    Agent, Cx, FaultPoint, HookCx, Host, HostError, InterruptHandle, Job, LoadRequest, ObjectRef,
    Outcome, Parsed, PropertyKey, Realm, Root, Script, Source, Termination, Value,
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

/// The host of every agent the target creates. The scripts of slice 1 schedule no job and load
/// no module; a job or a load the engine hands it is dropped, and the clock reads the epoch.
pub struct ConformanceHost;

impl Host for ConformanceHost {
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
    /// Every event the scenario's commands published, in order.
    published: Vec<ObservedEvent>,
}

impl Scenario {
    /// A new identity for an `AgentId` (kind 1), `RealmId` (2) or `ScriptId` (3).
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

/// The slice that brings an entry point for a command of the model slice 1 does not have.
fn later_slice(command: &str) -> Option<&'static str> {
    Some(match command.strip_prefix("swell.host.")? {
        "ParseModule"
        | "LoadRequestedModules"
        | "FinishLoadingImportedModule"
        | "LinkModule"
        | "EvaluateModule"
        | "HostLoadImportedModule"
        | "AsyncModuleExecutionSettled" => "slice 11 (modules)",
        "PerformMicrotaskCheckpoint"
        | "HostEnqueuePromiseJob"
        | "HostEnqueueGenericJob"
        | "HostEnqueueTimeoutJob"
        | "HostPromiseRejectionTracker" => "slice 10 (promises and jobs)",
        _ => return None,
    })
}

/// The swell host interface as a conformance target: a fresh set of agents per scenario.
pub struct SwellTarget {
    identity: ImplementationIdentity,
    interrupter: Interrupter,
    scenario: RefCell<Option<Scenario>>,
}

impl SwellTarget {
    /// A target named `identity` that sets an interrupt flag with `interrupter`. It runs agents
    /// on the calling thread, which must have [`STACK_BYTES`]; [`run`](crate::run) makes it.
    pub fn new(identity: ImplementationIdentity, interrupter: Interrupter) -> Self {
        Self {
            identity,
            interrupter,
            scenario: RefCell::new(None),
        }
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
            "swell.host.Interrupt" => self.interrupt(scenario, &mut input),
            "swell.host.Collect" => collect(scenario, &mut input),
            other => {
                return Err(TargetError::unsupported(
                    format!("invoking `{other}`"),
                    match later_slice(other) {
                        Some(slice) => {
                            format!("slice 1 has no entry point for `{other}`; {slice} brings it")
                        }
                        None => format!("the model of slice 1 has no command `{other}`"),
                    },
                ));
            }
        }?;
        input.finish()?;
        Ok(answer)
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
        Ok(Answer::took("set", Vec::new()))
    }
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
    let agent = Agent::new(ConformanceHost, limits, seeds).map_err(|error| {
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
            let outcome = OutcomeName::new(answer.outcome).map_err(|error| {
                TargetError::unavailable(format!("naming `{}`", answer.outcome), error.to_string())
            })?;
            let mut result = SemanticCommandResult::took(OutcomeRef::new(
                named::<CommandRef>(&command)?,
                outcome,
            ));
            result.error = answer.error;
            for (name, payload) in answer.events {
                let sequence = u64::try_from(scenario.published.len()).unwrap_or(u64::MAX);
                let event = observed(name, payload, &request.correlation, sequence)?;
                scenario.published.push(event.clone());
                result = result.emitting(event);
            }
            Ok(result)
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

    /// Accepts a forced outcome the call reaches for real and arranges `Collect`'s
    /// `engine-fault`; every other one is unsupported (module documentation).
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
            _ => {
                let why = if let Some(slice) = later_slice(&command) {
                    format!("slice 1 has no entry point for `{command}`; {slice} brings it")
                } else if command == "swell.host.EvaluateScript" && outcome == "engine-fault" {
                    "the fault switch of the interface has no point at evaluate_script \
                     (FaultPoint: ParseScript, CreateRealm, Collect; HOST-71)"
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

    fn await_outcome(&self, request: AwaitRequest) -> Result<AwaitAnswer, TargetError> {
        let command = request.command.to_string();
        Err(TargetError::unsupported(
            format!("awaiting `{command}` on an instance"),
            match later_slice(&command) {
                Some(slice) => format!("the engine of slice 1 never issues `{command}`; {slice}"),
                None => format!("the model of slice 1 has no command `{command}`"),
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ess_primitives::facts::Number;
    use ess_primitives::node::Node;

    use ess_conformance::target::{ImplementationIdentity, TargetError};
    use swell::InterruptHandle;

    use super::{
        Answer, Forced, Interrupter, STACK_BYTES, Scenario, SwellTarget, create_agent,
        create_realm, parse_script, sets_the_flag, values,
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
    }
}
