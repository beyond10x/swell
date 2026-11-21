//! Requirements of the host interface that are held by the compiler: each is a `compile_fail`
//! doc test on an item of its own, named after the requirement, and compiled only when the doc
//! tests are collected (`cfg(doctest)`). A `compile_fail` test passes on any compile error, and
//! stable rustdoc does not check the error code it names (measured: the test below passed with
//! `E0599` in place of `E0277`). So each one has a twin that compiles and differs from it in one
//! token only, which shows that the failure comes from that token.
//! (`docs/design/host-interface.md`, decisions 1 and 12.)
//!
//! A host of a doc test implements the five required hooks of decision 16; the hidden lines of
//! each block give the same one.

/// The static assertion of HOST-136: `InterruptHandle` is `Send` (and `Sync`, decision 12).
const _: fn() = || {
    fn send_and_sync<T: Send + Sync>() {}
    send_and_sync::<crate::InterruptHandle>();
};

/// HOST-136: an `Agent` is not `Send` (decision 1). Passing one to a function that requires
/// `Send` does not compile:
///
/// ```compile_fail,E0277
/// # use swell::{HookCx, Job, LoadRequest, ObjectRef};
/// struct NoHooks;
/// impl swell::Host for NoHooks {
/// #   fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
/// #   fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
/// #   fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 { 0 }
/// }
/// fn requires<T: Send>(_: T) {}
/// fn give(agent: swell::Agent<NoHooks>) {
///     requires(agent);
/// }
/// ```
///
/// and the same code with `Sized` in place of `Send` compiles:
///
/// ```
/// # use swell::{HookCx, Job, LoadRequest, ObjectRef};
/// struct NoHooks;
/// impl swell::Host for NoHooks {
/// #   fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
/// #   fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
/// #   fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 { 0 }
/// }
/// fn requires<T: Sized>(_: T) {}
/// fn give(agent: swell::Agent<NoHooks>) {
///     requires(agent);
/// }
/// ```
#[cfg(doctest)]
pub fn host_136_agent_is_not_send() {}

/// HOST-4: `HookCx` has no method that calls a function, constructs or evaluates a script
/// (decision 1). A call through a `HookCx` does not compile:
///
/// ```compile_fail,E0599
/// use swell::{Host, HookCx, Value};
/// fn hook<H: Host>(cx: &mut HookCx<'_, H>) {
///     let _ = cx.call(Value::Undefined, Value::Undefined, &[]);
/// }
/// ```
///
/// and the same call through a `Cx` compiles:
///
/// ```
/// use swell::{Host, Cx, Value};
/// fn hook<H: Host>(cx: &mut Cx<'_, H>) {
///     let _ = cx.call(Value::Undefined, Value::Undefined, &[]);
/// }
/// ```
///
/// A construct through a `HookCx` does not compile:
///
/// ```compile_fail,E0599
/// use swell::{Host, HookCx, Value};
/// fn hook<H: Host>(cx: &mut HookCx<'_, H>) {
///     let _ = cx.construct(Value::Undefined, &[], None);
/// }
/// ```
///
/// and the same construct through a `Cx` compiles:
///
/// ```
/// use swell::{Host, Cx, Value};
/// fn hook<H: Host>(cx: &mut Cx<'_, H>) {
///     let _ = cx.construct(Value::Undefined, &[], None);
/// }
/// ```
///
/// An evaluation of a script through a `HookCx` does not compile:
///
/// ```compile_fail,E0599
/// use swell::{Host, HookCx, ScriptRef};
/// fn hook<H: Host>(cx: &mut HookCx<'_, H>, script: ScriptRef) {
///     let _ = cx.evaluate_script(script);
/// }
/// ```
///
/// Its twin, the same evaluation through a `Cx`, comes with `Cx::evaluate_script`
/// (`story:interpreter-and-script-entry-points`); until then `Cx` has no such method either, so
/// this third block shows only that `HookCx` has none.
#[cfg(doctest)]
pub fn host_4_hook_cx_cannot_call_construct_or_evaluate_a_script() {}

/// HOST-5: a call on `&mut Agent` made from inside a host function or hook of the same agent
/// does not compile (decision 1). Inside a `with_cx` closure, the agent is borrowed by the call
/// that runs the closure:
///
/// ```compile_fail,E0499
/// # use swell::{Agent, HookCx, Job, Limits, LoadRequest, ObjectRef, Seeds, Value};
/// # struct NoHooks;
/// # impl swell::Host for NoHooks {
/// #   fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
/// #   fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
/// #   fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 { 0 }
/// # }
/// # let limits = Limits { steps_per_entry: 1, memory_bytes: 1 << 20, frame_depth: 1,
/// #     reentry_depth: 1, parser_nesting: 1, string_length: 1, bigint_bits: 1 };
/// # let seeds = Seeds { hash_key: [0; 16], random_seed: [0; 32] };
/// let mut agent = Agent::new(NoHooks, limits, seeds).unwrap();
/// let root = agent.with_cx(|cx| cx.root(Value::Null).unwrap()).unwrap();
/// let _ = agent.with_cx(|cx| {
///     let _ = agent.duplicate(&root);
/// });
/// ```
///
/// and the same call through the context compiles:
///
/// ```
/// # use swell::{Agent, HookCx, Job, Limits, LoadRequest, ObjectRef, Seeds, Value};
/// # struct NoHooks;
/// # impl swell::Host for NoHooks {
/// #   fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
/// #   fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
/// #   fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 { 0 }
/// # }
/// # let limits = Limits { steps_per_entry: 1, memory_bytes: 1 << 20, frame_depth: 1,
/// #     reentry_depth: 1, parser_nesting: 1, string_length: 1, bigint_bits: 1 };
/// # let seeds = Seeds { hash_key: [0; 16], random_seed: [0; 32] };
/// let mut agent = Agent::new(NoHooks, limits, seeds).unwrap();
/// let root = agent.with_cx(|cx| cx.root(Value::Null).unwrap()).unwrap();
/// let _ = agent.with_cx(|cx| {
///     let _ = cx.duplicate(&root);
/// });
/// ```
///
/// A host function is a `fn` item (`HostFn`), which cannot name the agent it runs in:
///
/// ```compile_fail,E0434
/// # use swell::{Agent, Completion, Cx, HookCx, HostCall, Job, Limits, LoadRequest, ObjectRef, Seeds, Value};
/// # struct NoHooks;
/// # impl swell::Host for NoHooks {
/// #   fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
/// #   fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
/// #   fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 { 0 }
/// # }
/// # let limits = Limits { steps_per_entry: 1, memory_bytes: 1 << 20, frame_depth: 1,
/// #     reentry_depth: 1, parser_nesting: 1, string_length: 1, bigint_bits: 1 };
/// # let seeds = Seeds { hash_key: [0; 16], random_seed: [0; 32] };
/// let mut agent = Agent::new(NoHooks, limits, seeds).unwrap();
/// fn steps(cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
///     let _ = agent.host();
///     Ok(Value::Undefined)
/// }
/// ```
///
/// and the same function reaching the host through its context compiles:
///
/// ```
/// # use swell::{Agent, Completion, Cx, HookCx, HostCall, Job, Limits, LoadRequest, ObjectRef, Seeds, Value};
/// # struct NoHooks;
/// # impl swell::Host for NoHooks {
/// #   fn host_enqueue_generic_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_enqueue_timeout_job(_: &mut HookCx<'_, Self>, _: Job, _: f64) {}
/// #   fn host_enqueue_finalization_registry_cleanup_job(_: &mut HookCx<'_, Self>, _: Job) {}
/// #   fn host_load_imported_module(_: &mut HookCx<'_, Self>, _: LoadRequest) {}
/// #   fn host_system_utc_epoch_nanoseconds(_: &mut HookCx<'_, Self>, _: ObjectRef) -> i128 { 0 }
/// # }
/// # let limits = Limits { steps_per_entry: 1, memory_bytes: 1 << 20, frame_depth: 1,
/// #     reentry_depth: 1, parser_nesting: 1, string_length: 1, bigint_bits: 1 };
/// # let seeds = Seeds { hash_key: [0; 16], random_seed: [0; 32] };
/// let mut agent = Agent::new(NoHooks, limits, seeds).unwrap();
/// fn steps(cx: &mut Cx<'_, NoHooks>, _call: HostCall<'_>) -> Completion<Value> {
///     let _ = cx.host();
///     Ok(Value::Undefined)
/// }
/// ```
#[cfg(doctest)]
pub fn host_5_agent_call_inside_a_host_function() {}
