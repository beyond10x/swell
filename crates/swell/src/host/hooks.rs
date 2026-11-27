//! The hook trait (`docs/design/host-interface.md`, decisions 16 and 17; ECMA-262 Annex D,
//! `#sec-host-hooks-summary`). The agent owns the host value; a hook is an associated function
//! that receives a context, not `&mut self` (decision 1). A hook the draft gives a default has
//! that default as a provided method; job scheduling, module loading and the clock have none and
//! are required (decision 16's table). Written by `story:object-model-and-realms`.
//!
//! `Job` and `LoadRequest` are the handle types of decisions 11 and 9. No engine path makes one
//! before slices 10 and 11; their agent key and roots come with the requirements that read them
//! (`HOST-153`, `HOST-154`).

use alloc::rc::Rc;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;

use super::host_object::HostData;
use super::outcome::Completion;
use super::{Cx, HookCx};
use crate::value::{ModuleRef, ObjectRef, PropertyKey, RealmRef, ScriptRef, StringRef, Value};

/// The `[[HostDefined]]` value of a realm, script, module or job callback (decision 7).
pub type HostDefined = Rc<dyn HostData>;

/// What a generic, timeout or cleanup job is (decision 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum JobKind {
    Generic,
    Timeout,
    FinalizationRegistryCleanup,
}

/// A Job Abstract Closure handed to the host (decision 11, `#sec-jobs`). Neither `Clone` nor
/// `Copy`. The root of the closure and the agent key come with slices 10 and 14.
#[derive(Debug)]
pub struct Job {
    kind: JobKind,
    realm: Option<RealmRef>,
}

impl Job {
    /// What the job is.
    pub fn kind(&self) -> JobKind {
        self.kind
    }

    /// The job's realm, `None` when it has none.
    pub fn realm(&self) -> Option<RealmRef> {
        self.realm
    }
}

/// The referrer of a module load (`#sec-HostLoadImportedModule`): a script, a module, or a realm
/// for an `import()` with no active script.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Referrer {
    Script(ScriptRef),
    Module(ModuleRef),
    Realm(RealmRef),
}

/// One import attribute of a module request (`#sec-modulerequest-record`), as written.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ImportAttribute {
    pub key: Vec<u16>,
    pub value: Vec<u16>,
}

/// One call of HostLoadImportedModule (decision 9). Neither `Clone` nor `Copy`. The roots of
/// the referrer and of the payload come with slice 11.
pub struct LoadRequest {
    referrer: Referrer,
    specifier: Vec<u16>,
    attributes: Vec<ImportAttribute>,
    host_defined: Option<HostDefined>,
    dynamic: bool,
}

impl LoadRequest {
    /// The script, module or realm that asked for the module.
    pub fn referrer(&self) -> Referrer {
        self.referrer
    }

    /// The module specifier, as written.
    pub fn specifier(&self) -> &[u16] {
        &self.specifier
    }

    /// The import attributes, as written.
    pub fn attributes(&self) -> &[ImportAttribute] {
        &self.attributes
    }

    /// The hostDefined of LoadRequestedModules; `None` for `import()`.
    pub fn host_defined(&self) -> Option<&HostDefined> {
        self.host_defined.as_ref()
    }

    /// Whether the payload is a PromiseCapability Record (`import()`).
    pub fn is_dynamic(&self) -> bool {
        self.dynamic
    }
}

impl core::fmt::Debug for LoadRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LoadRequest")
            .field("referrer", &self.referrer)
            .field("specifier", &self.specifier)
            .field("attributes", &self.attributes)
            .field("dynamic", &self.dynamic)
            .finish_non_exhaustive()
    }
}

/// The operation of HostPromiseRejectionTracker (`#sec-host-promise-rejection-tracker`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RejectionOperation {
    Reject,
    Handle,
}

/// The answer of HostResizeArrayBuffer and HostGrowSharedArrayBuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Handled {
    Handled,
    Unhandled,
}

/// The compilationType of HostEnsureCanCompileStrings.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompilationType {
    DirectEval,
    IndirectEval,
    Function,
}

/// The arguments of HostEnsureCanCompileStrings (`#sec-hostensurecancompilestrings`).
#[derive(Clone, Copy, Debug)]
pub struct CompileStrings<'r> {
    pub callee_realm: RealmRef,
    pub param_strings: &'r [StringRef],
    pub body_string: StringRef,
    pub code_string: StringRef,
    pub compilation_type: CompilationType,
    pub param_args: &'r [Value],
    pub body_arg: Value,
}

/// An ISO date-time record (decision 17; `#sec-temporal-iso-date-time-records`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IsoDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub millisecond: u16,
    pub microsecond: u16,
    pub nanosecond: u16,
}

/// A Time Zone Identifier Record (`#sec-time-zone-identifier-record`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TimeZoneIdentifier {
    pub identifier: String,
    pub primary: String,
}

/// The host of an agent: the value that answers the hooks (decision 16's table).
pub trait Host: Sized + 'static {
    /// HostGetSupportedImportAttributes (`#sec-hostgetsupportedimportattributes`): "it must
    /// return the same List with the same contents in the same order", so a constant.
    const SUPPORTED_IMPORT_ATTRIBUTES: &'static [&'static str] = &[];

    /// HostEnsureCanCompileStrings (`#sec-hostensurecancompilestrings`): the default "Return
    /// NormalCompletion(unused)".
    fn host_ensure_can_compile_strings(
        _cx: &mut Cx<'_, Self>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        Ok(())
    }

    /// HostGetCodeForEval (`#sec-hostgetcodeforeval`): the default "Return no-code".
    fn host_get_code_for_eval(_cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        None
    }

    /// HostEnsureCanAddPrivateElement (`#sec-hostensurecanaddprivateelement`): the default
    /// "Return NormalCompletion(unused)". The engine calls it only for a host object with an
    /// override table (`HostObjectSpec::overrides`), before a private field (HOST-119), a private
    /// method or a private accessor (HOST-223) is added to it, once per element (a getter and a
    /// setter of one name are one element); for any other object it calls nothing. A `Throw` is
    /// the add's throw, and the element is not added; a `Throw` value that holds a stale handle
    /// or a handle of another agent ends the run in `Terminated(EngineFault)` (decision 6). A set
    /// interrupt flag ends the run before the element is added. The `HookCx` runs no code and
    /// is no safe point.
    fn host_ensure_can_add_private_element(
        _cx: &mut HookCx<'_, Self>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        Ok(())
    }

    /// HostHasSourceTextAvailable (`#sec-hosthassourcetextavailable`): the default "Return
    /// true".
    fn host_has_source_text_available(_cx: &mut HookCx<'_, Self>, _func: ObjectRef) -> bool {
        true
    }

    /// HostMakeJobCallback (`#sec-hostmakejobcallback`): the default's [[HostDefined]] is
    /// empty.
    fn host_make_job_callback(
        _cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<HostDefined> {
        None
    }

    /// HostCallJobCallback (`#sec-hostcalljobcallback`): the default "Return ?
    /// Call(jobCallback.[[Callback]], V, argumentsList)". The engine gave these handles, so an
    /// `Err` from the call is a broken engine invariant and ends the run in
    /// `Terminated(EngineFault)` (decision 16's table).
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        _host_defined: Option<&HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        match cx.call(Value::Object(callback), this, args) {
            Ok(completion) => completion,
            Err(_) => Err(cx.agent.fault()),
        }
    }

    /// HostEnqueuePromiseJob (`#sec-hostenqueuepromisejob`): the job stays in the agent's
    /// queue (decision 11); the default does nothing more.
    fn host_enqueue_promise_job(_cx: &mut HookCx<'_, Self>, _realm: Option<RealmRef>) {}

    /// HTML's steps before a promise job (`html:hostenqueuepromisejob`): nothing by default.
    fn host_prepare_to_run_job(_cx: &mut HookCx<'_, Self>, _realm: RealmRef) {}

    /// HTML's steps after a promise job (`html:hostenqueuepromisejob`): nothing by default.
    fn host_clean_up_after_job(
        _cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
    }

    /// HostEnqueueGenericJob (`#sec-hostenqueuegenericjob`). Required: a dropped job is never
    /// performed.
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job);

    /// HostEnqueueTimeoutJob (`#sec-hostenqueuetimeoutjob`). Required.
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, milliseconds: f64);

    /// HostEnqueueFinalizationRegistryCleanupJob (`#sec-host-cleanup-finalization-registry`).
    /// Required.
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job);

    /// HostPromiseRejectionTracker (`#sec-host-promise-rejection-tracker`): the default does
    /// nothing.
    fn host_promise_rejection_tracker(
        _cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        _operation: RejectionOperation,
    ) {
    }

    /// HostLoadImportedModule (`#sec-HostLoadImportedModule`). Required: an unfinished load
    /// never finishes.
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest);

    /// HostGetImportMetaProperties (`#sec-hostgetimportmetaproperties`): the default "Return a
    /// new empty List".
    fn host_get_import_meta_properties(
        _cx: &mut HookCx<'_, Self>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        Ok(Vec::new())
    }

    /// HostFinalizeImportMeta (`#sec-hostfinalizeimportmeta`): the default "Return unused".
    fn host_finalize_import_meta(
        _cx: &mut HookCx<'_, Self>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
    }

    /// HostResizeArrayBuffer (`#sec-hostresizearraybuffer`): the default "Return
    /// NormalCompletion(unhandled)".
    fn host_resize_array_buffer(
        _cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        Ok(Handled::Unhandled)
    }

    /// HostGrowSharedArrayBuffer (`#sec-hostgrowsharedarraybuffer`): the default "Return
    /// NormalCompletion(unhandled)".
    fn host_grow_shared_array_buffer(
        _cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        Ok(Handled::Unhandled)
    }

    /// HTML's HostSystemUTCEpochNanoseconds (`html:hostsystemutcepochnanoseconds`), given the
    /// current realm's global object. Required: an engine with no ambient authority has no time
    /// to give.
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, global: ObjectRef) -> i128;

    /// SystemTimeZoneIdentifier (`#sec-systemtimezoneidentifier`): "If the implementation only
    /// supports the UTC time zone, return "UTC"".
    fn system_time_zone_identifier(_cx: &mut HookCx<'_, Self>) -> String {
        "UTC".to_string()
    }

    /// GetNamedTimeZoneOffsetNanoseconds (`#sec-getnamedtimezoneoffsetnanoseconds`): the
    /// default "Return 0".
    fn get_named_time_zone_offset_nanoseconds(
        _cx: &mut HookCx<'_, Self>,
        _id: &str,
        _epoch_ns: i128,
    ) -> i128 {
        0
    }

    /// GetNamedTimeZoneEpochNanoseconds (`#sec-getnamedtimezoneepochnanoseconds`): the default
    /// « GetUTCEpochNanoseconds(isoDateTime) », empty where `swell-num` finds the clause's
    /// assertion would fail.
    fn get_named_time_zone_epoch_nanoseconds(
        _cx: &mut HookCx<'_, Self>,
        _id: &str,
        local: IsoDateTime,
    ) -> Vec<i128> {
        let date = swell_num::date::IsoDateTime {
            year: i64::from(local.year),
            month: local.month,
            day: local.day,
            hour: local.hour,
            minute: local.minute,
            second: local.second,
            millisecond: local.millisecond,
            microsecond: local.microsecond,
            nanosecond: local.nanosecond,
        };
        swell_num::date::get_utc_epoch_nanoseconds(&date)
            .into_iter()
            .collect()
    }

    /// AvailableNamedTimeZoneIdentifiers (`#sec-availablenamedtimezoneidentifiers`): "Return
    /// « the Time Zone Identifier Record { [[Identifier]]: "UTC", [[PrimaryIdentifier]]: "UTC" }
    /// »".
    fn available_named_time_zone_identifiers(
        _cx: &mut HookCx<'_, Self>,
    ) -> Vec<TimeZoneIdentifier> {
        vec![TimeZoneIdentifier {
            identifier: "UTC".to_string(),
            primary: "UTC".to_string(),
        }]
    }
}

/// The host and agents of the library's own tests.
#[cfg(test)]
pub(crate) mod test_host {
    use super::{Host, Job, LoadRequest};
    use crate::host::HookCx;
    use crate::host::agent::Agent;
    use crate::host::limits::{Limits, Seeds};
    use crate::host::outcome::Outcome;
    use crate::value::{ObjectRef, Realm};

    /// A host whose required hooks do nothing.
    pub(crate) struct NoHooks;

    impl Host for NoHooks {
        fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}
        fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {}
        fn host_enqueue_finalization_registry_cleanup_job(_cx: &mut HookCx<'_, Self>, _job: Job) {}
        fn host_load_imported_module(_cx: &mut HookCx<'_, Self>, _request: LoadRequest) {}
        fn host_system_utc_epoch_nanoseconds(
            _cx: &mut HookCx<'_, Self>,
            _global: ObjectRef,
        ) -> i128 {
            0
        }
    }

    /// The limits of the library's tests.
    pub(crate) fn limits() -> Limits {
        Limits {
            steps_per_entry: 1_000_000,
            memory_bytes: 1 << 24,
            frame_depth: 100,
            reentry_depth: 10,
            parser_nesting: 100,
            string_length: 1 << 20,
            bigint_bits: 1 << 10,
        }
    }

    /// An agent with `limits`.
    pub(crate) fn agent_with(limits: Limits) -> Agent<NoHooks> {
        match Agent::new(
            NoHooks,
            limits,
            Seeds {
                hash_key: [1; 16],
                random_seed: [2; 32],
            },
        ) {
            Ok(agent) => agent,
            Err(error) => panic!("Agent::new answered {error:?}"),
        }
    }

    /// An agent with `limits` and a realm made with no setup closure.
    pub(crate) fn agent_with_realm_and(limits: Limits) -> (Agent<NoHooks>, Realm) {
        let mut agent = agent_with(limits);
        let realm = new_realm(&mut agent);
        (agent, realm)
    }

    /// Make `realm` the realm of the entry, as `with_cx` of it does, for a unit test that runs
    /// engine operations on the agent directly (decision 7: every operation runs in an entry
    /// that names its realm).
    pub(crate) fn enter(agent: &mut Agent<NoHooks>, realm: &Realm) {
        match agent.heap.roots.get_realm(&realm.0) {
            Ok(handle) => agent.entry_realm = Some(handle),
            Err(error) => panic!("the realm answered {error:?}"),
        }
    }

    /// A realm of `agent` made with no setup closure: what `with_cx`, `Agent::call` and
    /// `Agent::construct` name as the current realm (decision 7).
    pub(crate) fn new_realm(agent: &mut Agent<NoHooks>) -> Realm {
        match agent.create_realm(None, None) {
            Ok(Outcome::Normal(realm)) => realm,
            other => panic!("create_realm answered {other:?}"),
        }
    }

    /// An agent with [`limits`] and a realm made with no setup closure.
    pub(crate) fn agent_with_realm() -> (Agent<NoHooks>, Realm) {
        agent_with_realm_and(limits())
    }
}
