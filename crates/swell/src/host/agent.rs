//! The agent (`docs/design/host-interface.md`, decisions 1, 2, 4, 6, 12, 13 and 18; ECMA-262 9.6,
//! `#sec-agents`): one engine instance with its heap, its host value, its limits and seeds, its
//! interrupt handle and its state; how it is created, and the reads that run no code, call no
//! hook and spend no step. Written by `story:heap-values-and-host-surface`.
//!
//! An agent is created, run and dropped on one thread and is not `Send` (decision 1): its roots
//! share an `Rc`'d release queue. Every entry point takes `&mut self`, so code running inside
//! the agent cannot call it; inside, the host reaches the engine through `Cx` or `HookCx`.
//! After a run ends in `Terminated(EngineFault)` the agent is faulted for good and answers every
//! call but `state()` with `Err(HostError::Faulted)` (decision 6).
//!
//! Root site of the collector: [`trace_roots`] reports the values handed to host code in the
//! running `with_cx`, host function and hook scopes.

use std::sync::atomic::{AtomicU32, Ordering};

use super::Host;
use super::interrupt::InterruptHandle;
use super::limits::{CreateError, Limits, Seeds};
use super::outcome::{Abrupt, CollectReport, Completion, HostError, Outcome, Termination};
use crate::heap::Heap;
use crate::heap::collect::{self, Mark, Stale};
use crate::value::{Root, Value};

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
    #[expect(
        dead_code,
        reason = "the step budget and depth bounds: story:interpreter-and-script-entry-points"
    )]
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
        })
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

    /// The bytes the agent's heap is charged now (decision 13).
    pub fn heap_bytes(&self) -> Result<u64, HostError> {
        self.alive()?;
        Ok(self.heap.bytes())
    }

    /// A full collection now (decision 18). It runs no code and calls no hook. A stale handle
    /// reported by a trace answers `Terminated(EngineFault)` and faults the agent.
    pub fn collect(&mut self) -> Result<Outcome<CollectReport>, HostError> {
        self.alive()?;
        match collect::collect(self) {
            Ok(report) => Ok(Outcome::Normal(report)),
            Err(Stale) => {
                self.fault();
                Ok(Outcome::Terminated(Termination::EngineFault))
            }
        }
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

    /// End the running entry with `termination`; `EngineFault` faults the agent.
    pub(crate) fn terminate(&mut self, termination: Termination) -> Abrupt {
        if termination == Termination::EngineFault {
            self.state = AgentState::Faulted;
        }
        self.termination = Some(termination);
        Abrupt::Terminated(termination)
    }

    /// The engine found one of its invariants broken: end the run and fault the agent.
    pub(crate) fn fault(&mut self) -> Abrupt {
        self.terminate(Termination::EngineFault)
    }

    /// A safe point (decision 4): release the dropped roots, and collect when the bytes
    /// allocated since the last collection passed the threshold, or always with the feature
    /// `collect-every-safe-point`.
    pub(crate) fn safe_point(&mut self) -> Result<(), Stale> {
        self.heap.roots.drain();
        if cfg!(feature = "collect-every-safe-point") || self.heap.collection_due() {
            collect::collect(self)?;
        }
        Ok(())
    }

    /// Record `value` as handed to host code in the running scope.
    pub(crate) fn hand_out(&mut self, value: Value) {
        self.scope.push(value);
    }
}

/// Root site of the collector: the values handed to host code in the running scopes.
pub(crate) fn trace_roots<H: Host>(agent: &Agent<H>, mark: &mut Mark) {
    mark.count("host/agent.rs trace_roots");
    for &value in &agent.scope {
        mark.value(value);
    }
}
