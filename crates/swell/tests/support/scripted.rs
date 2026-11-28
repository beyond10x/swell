//! The scripted host: one type of `Host` for every test whose host answers hooks of its own.
//!
//! The engine is generic over its host (`impl<H: Host> Agent<H>`), so each type that
//! implements `Host` compiles the engine once more inside the test binary `swell::main`. A test
//! that needs hooks of its own gives them to [`Scripted`] as values instead of writing a type:
//! a table of function pointers ([`Hooks`]) and its own state, which a hook and the test reach
//! through [`Scripted::state`] and [`Scripted::state_mut`] (`host_types.rs` holds the rule).
#![cfg(test)]

use std::any::{Any, type_name};

use swell::{
    CompileStrings, Completion, Cx, Handled, HookCx, Host, HostDefined, IsoDateTime, Job,
    LoadRequest, ModuleRef, ObjectRef, PropertyKey, RealmRef, RejectionOperation, StringRef,
    TimeZoneIdentifier, Value,
};

/// The entry of HostCallJobCallback in [`Hooks`].
pub type CallJobCallback = fn(
    &mut Cx<'_, Scripted>,
    ObjectRef,
    Option<&HostDefined>,
    Value,
    &[Value],
) -> Completion<Value>;

/// The entry of HostGetImportMetaProperties in [`Hooks`].
pub type GetImportMetaProperties =
    fn(&mut HookCx<'_, Scripted>, ModuleRef) -> Completion<Vec<(PropertyKey, Value)>>;

/// The hooks of a [`Scripted`] host, one entry per hook of the trait `Host`
/// (`crates/swell/src/host/hooks.rs`), each over the context the trait gives that hook. A test
/// writes the entries it answers and takes the rest from [`Hooks::PROVIDED`]:
///
/// ```ignore
/// const HOOKS: Hooks = Hooks {
///     host_enqueue_generic_job,
///     host_enqueue_timeout_job,
///     host_enqueue_finalization_registry_cleanup_job,
///     host_load_imported_module,
///     host_system_utc_epoch_nanoseconds,
///     ..Hooks::PROVIDED
/// };
/// ```
#[derive(Clone, Copy)]
pub struct Hooks {
    pub host_ensure_can_compile_strings:
        fn(&mut Cx<'_, Scripted>, &CompileStrings<'_>) -> Completion<()>,
    pub host_get_code_for_eval: fn(&mut HookCx<'_, Scripted>, ObjectRef) -> Option<StringRef>,
    pub host_ensure_can_add_private_element:
        fn(&mut HookCx<'_, Scripted>, ObjectRef) -> Completion<()>,
    pub host_has_source_text_available: fn(&mut HookCx<'_, Scripted>, ObjectRef) -> bool,
    pub host_make_job_callback: fn(&mut HookCx<'_, Scripted>, ObjectRef) -> Option<HostDefined>,
    pub host_call_job_callback: CallJobCallback,
    pub host_enqueue_promise_job: fn(&mut HookCx<'_, Scripted>, Option<RealmRef>),
    pub host_prepare_to_run_job: fn(&mut HookCx<'_, Scripted>, RealmRef),
    pub host_clean_up_after_job: fn(&mut HookCx<'_, Scripted>, Option<RealmRef>, Result<(), Value>),
    pub host_enqueue_generic_job: fn(&mut HookCx<'_, Scripted>, Job),
    pub host_enqueue_timeout_job: fn(&mut HookCx<'_, Scripted>, Job, f64),
    pub host_enqueue_finalization_registry_cleanup_job: fn(&mut HookCx<'_, Scripted>, Job),
    pub host_promise_rejection_tracker:
        fn(&mut HookCx<'_, Scripted>, ObjectRef, RejectionOperation),
    pub host_load_imported_module: fn(&mut HookCx<'_, Scripted>, LoadRequest),
    pub host_get_import_meta_properties: GetImportMetaProperties,
    pub host_finalize_import_meta: fn(&mut HookCx<'_, Scripted>, ObjectRef, ModuleRef),
    pub host_resize_array_buffer:
        fn(&mut HookCx<'_, Scripted>, ObjectRef, u64) -> Completion<Handled>,
    pub host_grow_shared_array_buffer:
        fn(&mut HookCx<'_, Scripted>, ObjectRef, u64) -> Completion<Handled>,
    pub host_system_utc_epoch_nanoseconds: fn(&mut HookCx<'_, Scripted>, ObjectRef) -> i128,
    pub system_time_zone_identifier: fn(&mut HookCx<'_, Scripted>) -> String,
    pub get_named_time_zone_offset_nanoseconds: fn(&mut HookCx<'_, Scripted>, &str, i128) -> i128,
    pub get_named_time_zone_epoch_nanoseconds:
        fn(&mut HookCx<'_, Scripted>, &str, IsoDateTime) -> Vec<i128>,
    pub available_named_time_zone_identifiers:
        fn(&mut HookCx<'_, Scripted>) -> Vec<TimeZoneIdentifier>,
}

impl Hooks {
    /// The provided hooks as the trait's defaults answer them (copies, in [`provided`]), and
    /// the five required hooks as a panic that names the hook: a required hook has no default,
    /// so a test gives all five.
    pub const PROVIDED: Hooks = Hooks {
        host_ensure_can_compile_strings: provided::host_ensure_can_compile_strings,
        host_get_code_for_eval: provided::host_get_code_for_eval,
        host_ensure_can_add_private_element: provided::host_ensure_can_add_private_element,
        host_has_source_text_available: provided::host_has_source_text_available,
        host_make_job_callback: provided::host_make_job_callback,
        host_call_job_callback: provided::host_call_job_callback,
        host_enqueue_promise_job: provided::host_enqueue_promise_job,
        host_prepare_to_run_job: provided::host_prepare_to_run_job,
        host_clean_up_after_job: provided::host_clean_up_after_job,
        host_enqueue_generic_job: required::host_enqueue_generic_job,
        host_enqueue_timeout_job: required::host_enqueue_timeout_job,
        host_enqueue_finalization_registry_cleanup_job:
            required::host_enqueue_finalization_registry_cleanup_job,
        host_promise_rejection_tracker: provided::host_promise_rejection_tracker,
        host_load_imported_module: required::host_load_imported_module,
        host_get_import_meta_properties: provided::host_get_import_meta_properties,
        host_finalize_import_meta: provided::host_finalize_import_meta,
        host_resize_array_buffer: provided::host_resize_array_buffer,
        host_grow_shared_array_buffer: provided::host_grow_shared_array_buffer,
        host_system_utc_epoch_nanoseconds: required::host_system_utc_epoch_nanoseconds,
        system_time_zone_identifier: provided::system_time_zone_identifier,
        get_named_time_zone_offset_nanoseconds: provided::get_named_time_zone_offset_nanoseconds,
        get_named_time_zone_epoch_nanoseconds: provided::get_named_time_zone_epoch_nanoseconds,
        available_named_time_zone_identifiers: provided::available_named_time_zone_identifiers,
    };
}

/// The one host type of the tests whose host answers hooks of its own: each hook calls its
/// entry of the [`Hooks`] the test gave, and the test's state is held behind `Any`.
///
/// A hook a test does not answer is, in [`Hooks::PROVIDED`], a copy of the trait's provided
/// default, written once in [`provided`]. Nothing ties a copy to the trait's default but this
/// sentence: a change of a provided default in `crates/swell/src/host/hooks.rs` changes its
/// copy here in the same change. The defaults themselves stay tested on `TestHost` and
/// `ProvidedHost` (`tests/hooks.rs`), which keep them. The copy of `host_call_job_callback`
/// panics where the default answers with the agent's fault, which a test crate cannot reach.
///
/// `SUPPORTED_IMPORT_ATTRIBUTES` is a constant of a type, not a value: this type leaves it to
/// the trait (none), and a test that needs another value is a type of its own, marked
/// `own-constant` (`host_types.rs`).
pub struct Scripted {
    hooks: Hooks,
    state: Box<dyn Any>,
    /// The type name of `state`, for the panic of a wrong accessor.
    state_type: &'static str,
}

impl Scripted {
    /// A scripted host with `hooks` and the test's `state`.
    pub fn new<T: Any>(state: T, hooks: Hooks) -> Scripted {
        Scripted {
            hooks,
            state: Box::new(state),
            state_type: type_name::<T>(),
        }
    }

    /// The test's state; a panic when it is not a `T`.
    pub fn state<T: Any>(&self) -> &T {
        match self.state.downcast_ref::<T>() {
            Some(state) => state,
            None => panic!(
                "the scripted host holds a {}, not a {}",
                self.state_type,
                type_name::<T>()
            ),
        }
    }

    /// The test's state, to change; a panic when it is not a `T`.
    pub fn state_mut<T: Any>(&mut self) -> &mut T {
        let held = self.state_type;
        match self.state.downcast_mut::<T>() {
            Some(state) => state,
            None => panic!(
                "the scripted host holds a {held}, not a {}",
                type_name::<T>()
            ),
        }
    }
}

// host-type: scripted
impl Host for Scripted {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        request: &CompileStrings<'_>,
    ) -> Completion<()> {
        let hook = cx.host().hooks.host_ensure_can_compile_strings;
        hook(cx, request)
    }

    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, arg: ObjectRef) -> Option<StringRef> {
        let hook = cx.host().hooks.host_get_code_for_eval;
        hook(cx, arg)
    }

    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        obj: ObjectRef,
    ) -> Completion<()> {
        let hook = cx.host().hooks.host_ensure_can_add_private_element;
        hook(cx, obj)
    }

    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, func: ObjectRef) -> bool {
        let hook = cx.host().hooks.host_has_source_text_available;
        hook(cx, func)
    }

    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        callback: ObjectRef,
    ) -> Option<HostDefined> {
        let hook = cx.host().hooks.host_make_job_callback;
        hook(cx, callback)
    }

    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        host_defined: Option<&HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        let hook = cx.host().hooks.host_call_job_callback;
        hook(cx, callback, host_defined, this, args)
    }

    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, realm: Option<RealmRef>) {
        let hook = cx.host().hooks.host_enqueue_promise_job;
        hook(cx, realm);
    }

    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, realm: RealmRef) {
        let hook = cx.host().hooks.host_prepare_to_run_job;
        hook(cx, realm);
    }

    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        realm: Option<RealmRef>,
        result: Result<(), Value>,
    ) {
        let hook = cx.host().hooks.host_clean_up_after_job;
        hook(cx, realm, result);
    }

    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job) {
        let hook = cx.host().hooks.host_enqueue_generic_job;
        hook(cx, job);
    }

    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, milliseconds: f64) {
        let hook = cx.host().hooks.host_enqueue_timeout_job;
        hook(cx, job, milliseconds);
    }

    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job) {
        let hook = cx
            .host()
            .hooks
            .host_enqueue_finalization_registry_cleanup_job;
        hook(cx, job);
    }

    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        promise: ObjectRef,
        operation: RejectionOperation,
    ) {
        let hook = cx.host().hooks.host_promise_rejection_tracker;
        hook(cx, promise, operation);
    }

    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest) {
        let hook = cx.host().hooks.host_load_imported_module;
        hook(cx, request);
    }

    fn host_get_import_meta_properties(
        cx: &mut HookCx<'_, Self>,
        module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        let hook = cx.host().hooks.host_get_import_meta_properties;
        hook(cx, module)
    }

    fn host_finalize_import_meta(
        cx: &mut HookCx<'_, Self>,
        import_meta: ObjectRef,
        module: ModuleRef,
    ) {
        let hook = cx.host().hooks.host_finalize_import_meta;
        hook(cx, import_meta, module);
    }

    fn host_resize_array_buffer(
        cx: &mut HookCx<'_, Self>,
        buffer: ObjectRef,
        new_byte_length: u64,
    ) -> Completion<Handled> {
        let hook = cx.host().hooks.host_resize_array_buffer;
        hook(cx, buffer, new_byte_length)
    }

    fn host_grow_shared_array_buffer(
        cx: &mut HookCx<'_, Self>,
        buffer: ObjectRef,
        new_byte_length: u64,
    ) -> Completion<Handled> {
        let hook = cx.host().hooks.host_grow_shared_array_buffer;
        hook(cx, buffer, new_byte_length)
    }

    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, global: ObjectRef) -> i128 {
        let hook = cx.host().hooks.host_system_utc_epoch_nanoseconds;
        hook(cx, global)
    }

    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        let hook = cx.host().hooks.system_time_zone_identifier;
        hook(cx)
    }

    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        id: &str,
        epoch_ns: i128,
    ) -> i128 {
        let hook = cx.host().hooks.get_named_time_zone_offset_nanoseconds;
        hook(cx, id, epoch_ns)
    }

    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        id: &str,
        local: IsoDateTime,
    ) -> Vec<i128> {
        let hook = cx.host().hooks.get_named_time_zone_epoch_nanoseconds;
        hook(cx, id, local)
    }

    fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier> {
        let hook = cx.host().hooks.available_named_time_zone_identifiers;
        hook(cx)
    }
}

/// Copies of the provided defaults of the trait `Host` (`crates/swell/src/host/hooks.rs`), one
/// per provided hook, each answering as the default does. A change of a default there changes
/// its copy here in the same change.
pub mod provided {
    use super::Scripted;
    use swell::{
        CompileStrings, Completion, Cx, Handled, HookCx, HostDefined, IsoDateTime, ModuleRef,
        ObjectRef, PropertyKey, RealmRef, RejectionOperation, StringRef, TimeZoneIdentifier, Value,
    };

    /// HostEnsureCanCompileStrings: "Return NormalCompletion(unused)".
    pub fn host_ensure_can_compile_strings(
        _cx: &mut Cx<'_, Scripted>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        Ok(())
    }

    /// HostGetCodeForEval: "Return no-code".
    pub fn host_get_code_for_eval(
        _cx: &mut HookCx<'_, Scripted>,
        _arg: ObjectRef,
    ) -> Option<StringRef> {
        None
    }

    /// HostEnsureCanAddPrivateElement: "Return NormalCompletion(unused)".
    pub fn host_ensure_can_add_private_element(
        _cx: &mut HookCx<'_, Scripted>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        Ok(())
    }

    /// HostHasSourceTextAvailable: "Return true".
    pub fn host_has_source_text_available(
        _cx: &mut HookCx<'_, Scripted>,
        _func: ObjectRef,
    ) -> bool {
        true
    }

    /// HostMakeJobCallback: an empty [[HostDefined]].
    pub fn host_make_job_callback(
        _cx: &mut HookCx<'_, Scripted>,
        _callback: ObjectRef,
    ) -> Option<HostDefined> {
        None
    }

    /// HostCallJobCallback: "Return ? Call(jobCallback.[[Callback]], V, argumentsList)". The
    /// engine gave the handles, so a call that refuses them is a broken engine invariant; the
    /// default answers it with the agent's fault (`Terminated(EngineFault)`), which a test crate
    /// cannot reach, so the copy panics.
    pub fn host_call_job_callback(
        cx: &mut Cx<'_, Scripted>,
        callback: ObjectRef,
        _host_defined: Option<&HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        match cx.call(Value::Object(callback), this, args) {
            Ok(completion) => completion,
            Err(error) => panic!(
                "HostCallJobCallback: Cx::call refused the job callback {callback:?} the engine \
                 gave, with {error:?}: a broken engine invariant, which the trait's default \
                 answers with Terminated(EngineFault)"
            ),
        }
    }

    /// HostEnqueuePromiseJob: nothing more than the agent's queue.
    pub fn host_enqueue_promise_job(_cx: &mut HookCx<'_, Scripted>, _realm: Option<RealmRef>) {}

    /// HTML's steps before a promise job: nothing.
    pub fn host_prepare_to_run_job(_cx: &mut HookCx<'_, Scripted>, _realm: RealmRef) {}

    /// HTML's steps after a promise job: nothing.
    pub fn host_clean_up_after_job(
        _cx: &mut HookCx<'_, Scripted>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
    }

    /// HostPromiseRejectionTracker: nothing.
    pub fn host_promise_rejection_tracker(
        _cx: &mut HookCx<'_, Scripted>,
        _promise: ObjectRef,
        _operation: RejectionOperation,
    ) {
    }

    /// HostGetImportMetaProperties: "Return a new empty List".
    pub fn host_get_import_meta_properties(
        _cx: &mut HookCx<'_, Scripted>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        Ok(Vec::new())
    }

    /// HostFinalizeImportMeta: "Return unused".
    pub fn host_finalize_import_meta(
        _cx: &mut HookCx<'_, Scripted>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
    }

    /// HostResizeArrayBuffer: "Return NormalCompletion(unhandled)".
    pub fn host_resize_array_buffer(
        _cx: &mut HookCx<'_, Scripted>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        Ok(Handled::Unhandled)
    }

    /// HostGrowSharedArrayBuffer: "Return NormalCompletion(unhandled)".
    pub fn host_grow_shared_array_buffer(
        _cx: &mut HookCx<'_, Scripted>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        Ok(Handled::Unhandled)
    }

    /// SystemTimeZoneIdentifier: "UTC".
    pub fn system_time_zone_identifier(_cx: &mut HookCx<'_, Scripted>) -> String {
        "UTC".to_string()
    }

    /// GetNamedTimeZoneOffsetNanoseconds: "Return 0".
    pub fn get_named_time_zone_offset_nanoseconds(
        _cx: &mut HookCx<'_, Scripted>,
        _id: &str,
        _epoch_ns: i128,
    ) -> i128 {
        0
    }

    /// GetNamedTimeZoneEpochNanoseconds: « GetUTCEpochNanoseconds(isoDateTime) », empty where
    /// `swell-num` finds the clause's assertion would fail.
    pub fn get_named_time_zone_epoch_nanoseconds(
        _cx: &mut HookCx<'_, Scripted>,
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

    /// AvailableNamedTimeZoneIdentifiers: « the record of "UTC" ».
    pub fn available_named_time_zone_identifiers(
        _cx: &mut HookCx<'_, Scripted>,
    ) -> Vec<TimeZoneIdentifier> {
        vec![TimeZoneIdentifier {
            identifier: "UTC".to_string(),
            primary: "UTC".to_string(),
        }]
    }
}

/// The entries of the required hooks in [`Hooks::PROVIDED`]: a required hook has no default,
/// so a test that leaves one to `PROVIDED` stops with a panic naming it when the engine calls
/// it.
mod required {
    use super::Scripted;
    use swell::{HookCx, Job, LoadRequest, ObjectRef};

    pub fn host_enqueue_generic_job(_cx: &mut HookCx<'_, Scripted>, _job: Job) {
        unanswered("host_enqueue_generic_job")
    }

    pub fn host_enqueue_timeout_job(_cx: &mut HookCx<'_, Scripted>, _job: Job, _milliseconds: f64) {
        unanswered("host_enqueue_timeout_job")
    }

    pub fn host_enqueue_finalization_registry_cleanup_job(
        _cx: &mut HookCx<'_, Scripted>,
        _job: Job,
    ) {
        unanswered("host_enqueue_finalization_registry_cleanup_job")
    }

    pub fn host_load_imported_module(_cx: &mut HookCx<'_, Scripted>, _request: LoadRequest) {
        unanswered("host_load_imported_module")
    }

    pub fn host_system_utc_epoch_nanoseconds(
        _cx: &mut HookCx<'_, Scripted>,
        _global: ObjectRef,
    ) -> i128 {
        unanswered("host_system_utc_epoch_nanoseconds")
    }

    fn unanswered(hook: &str) -> ! {
        panic!("the test gave the scripted host no {hook}: a required hook has no default")
    }
}
