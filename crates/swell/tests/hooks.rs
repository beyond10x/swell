//! HOST-124: each provided hook of the `Host` trait, left as provided, gives the draft's default
//! answer listed in the table of `docs/design/host-interface.md`, decision 16; one test per
//! provided item, but `host_get_import_meta_properties` and `host_finalize_import_meta`
//! (HOST-158, slice 11). The five required hooks have no default to test.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, realm};
use swell::{
    AgentState, Brand, CompilationType, CompileStrings, Completion, Cx, GlobalObjects, Handled,
    HookCx, Host, HostCall, HostObjectSpec, IsoDateTime, ObjectRef, Outcome, Prototype, RealmRef,
    RejectionOperation, TimeZoneIdentifier, Value,
};

/// Runs `f` inside the setup closure of a `create_realm`, the place a host meets a `HookCx`
/// outside a hook, and answers what `f` returned. The realm's global is a host object.
fn in_hook<R>(f: impl FnOnce(&mut HookCx<'_, TestHost>, RealmRef, ObjectRef) -> R) -> R {
    let mut agent = agent().unwrap();
    let mut f = Some(f);
    let mut out = None;
    let mut setup = |cx: &mut HookCx<'_, TestHost>, realm: RealmRef| -> Completion<GlobalObjects> {
        let global = cx
            .new_host_object(HostObjectSpec {
                prototype: Prototype::Null,
                brand: Brand(1),
                data: None,
                error_kind: None,
            })
            .unwrap()?;
        if let Some(f) = f.take() {
            out = Some(f(cx, realm, global));
        }
        Ok(GlobalObjects { global, this: None })
    };
    let created = support::host::create_realm(&mut agent, Some(&mut setup), None).unwrap();
    assert!(matches!(created, Outcome::Normal(_)), "{created:?}");
    assert_eq!(agent.state(), AgentState::Alive);
    assert!(agent.host().calls.is_empty(), "{:?}", agent.host().calls);
    out.unwrap()
}

/// `host_ensure_can_compile_strings` (`#sec-hostensurecancompilestrings`): `Ok(())`.
#[test]
fn host_124_host_ensure_can_compile_strings_default() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let answer = agent
        .with_cx(&realm, |cx| {
            let callee_realm = cx.realm_ref(&realm).unwrap();
            let body = cx.new_string("1").unwrap();
            let request = CompileStrings {
                callee_realm,
                param_strings: &[],
                body_string: body,
                code_string: body,
                compilation_type: CompilationType::DirectEval,
                param_args: &[],
                body_arg: Value::String(body),
            };
            TestHost::host_ensure_can_compile_strings(cx, &request)
        })
        .unwrap();
    assert_eq!(answer, Ok(()));
}

/// `host_get_code_for_eval` (`#sec-hostgetcodeforeval`): `None`, no-code.
#[test]
fn host_124_host_get_code_for_eval_default() {
    let answer = in_hook(|cx, _, global| TestHost::host_get_code_for_eval(cx, global));
    assert_eq!(answer, None);
}

/// `host_ensure_can_add_private_element` (`#sec-hostensurecanaddprivateelement`): `Ok(())`.
#[test]
fn host_124_host_ensure_can_add_private_element_default() {
    let answer = in_hook(|cx, _, global| TestHost::host_ensure_can_add_private_element(cx, global));
    assert_eq!(answer, Ok(()));
}

/// `host_has_source_text_available` (`#sec-hosthassourcetextavailable`): `true`.
#[test]
fn host_124_host_has_source_text_available_default() {
    let answer = in_hook(|cx, _, global| TestHost::host_has_source_text_available(cx, global));
    assert!(answer);
}

/// `host_make_job_callback` (`#sec-hostmakejobcallback`): `None`, an empty [[HostDefined]].
#[test]
fn host_124_host_make_job_callback_default() {
    let answer = in_hook(|cx, _, global| TestHost::host_make_job_callback(cx, global).is_none());
    assert!(answer);
}

/// The steps of the callback of `host_124_host_call_job_callback_default`: counts its calls
/// and answers its `this` when its first argument is 1.
fn callback(cx: &mut Cx<'_, TestHost>, call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    if call.args.first() == Some(&Value::Number(1.0)) && call.new_target.is_none() {
        Ok(call.this)
    } else {
        Ok(Value::Null)
    }
}

/// `host_call_job_callback` (`#sec-hostcalljobcallback`): `Call(callback, V, argumentsList)`.
#[test]
fn host_124_host_call_job_callback_default() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let answer = agent
        .with_cx(&realm, |cx| {
            let realm = cx.realm_ref(&realm).unwrap();
            let function = cx
                .new_host_function(realm, "callback", 1, callback, false, None)
                .unwrap()
                .unwrap();
            TestHost::host_call_job_callback(
                cx,
                function,
                None,
                Value::Boolean(true),
                &[Value::Number(1.0)],
            )
        })
        .unwrap();
    assert_eq!(answer, Ok(Value::Boolean(true)));
    assert_eq!(agent.host().count, 1);
    assert_eq!(agent.state(), AgentState::Alive);
}

/// `host_enqueue_promise_job` (`#sec-hostenqueuepromisejob`): nothing.
#[test]
fn host_124_host_enqueue_promise_job_default() {
    in_hook(|cx, realm, _| {
        TestHost::host_enqueue_promise_job(cx, Some(realm));
        TestHost::host_enqueue_promise_job(cx, None);
    });
}

/// `host_prepare_to_run_job` (HTML's steps around a promise job): nothing.
#[test]
fn host_124_host_prepare_to_run_job_default() {
    in_hook(|cx, realm, _| TestHost::host_prepare_to_run_job(cx, realm));
}

/// `host_clean_up_after_job` (HTML's steps around a promise job): nothing.
#[test]
fn host_124_host_clean_up_after_job_default() {
    in_hook(|cx, realm, _| {
        TestHost::host_clean_up_after_job(cx, Some(realm), Ok(()));
        TestHost::host_clean_up_after_job(cx, None, Err(Value::Number(1.0)));
    });
}

/// `host_promise_rejection_tracker` (`#sec-host-promise-rejection-tracker`): nothing.
#[test]
fn host_124_host_promise_rejection_tracker_default() {
    in_hook(|cx, _, global| {
        TestHost::host_promise_rejection_tracker(cx, global, RejectionOperation::Reject);
        TestHost::host_promise_rejection_tracker(cx, global, RejectionOperation::Handle);
    });
}

/// `SUPPORTED_IMPORT_ATTRIBUTES` (`#sec-hostgetsupportedimportattributes`): `&[]`.
#[test]
fn host_124_supported_import_attributes_default() {
    assert_eq!(TestHost::SUPPORTED_IMPORT_ATTRIBUTES, &[] as &[&str]);
}

/// `host_resize_array_buffer` (`#sec-hostresizearraybuffer`): `Ok(Handled::Unhandled)`.
#[test]
fn host_124_host_resize_array_buffer_default() {
    let answer = in_hook(|cx, _, global| TestHost::host_resize_array_buffer(cx, global, 8));
    assert_eq!(answer, Ok(Handled::Unhandled));
}

/// `host_grow_shared_array_buffer` (`#sec-hostgrowsharedarraybuffer`): `Ok(Handled::Unhandled)`.
#[test]
fn host_124_host_grow_shared_array_buffer_default() {
    let answer = in_hook(|cx, _, global| TestHost::host_grow_shared_array_buffer(cx, global, 8));
    assert_eq!(answer, Ok(Handled::Unhandled));
}

/// `system_time_zone_identifier` (`#sec-systemtimezoneidentifier`, "If the implementation only
/// supports the UTC time zone, return "UTC""): `"UTC"`.
#[test]
fn host_124_system_time_zone_identifier_default() {
    let answer = in_hook(|cx, _, _| TestHost::system_time_zone_identifier(cx));
    assert_eq!(answer, "UTC");
}

/// `get_named_time_zone_offset_nanoseconds` (`#sec-getnamedtimezoneoffsetnanoseconds`, the
/// default "Return 0."): 0 at every instant.
#[test]
fn host_124_get_named_time_zone_offset_nanoseconds_default() {
    let answer = in_hook(|cx, _, _| {
        [0, 1, -1, 8_640_000_000_000_000_000_000]
            .map(|epoch_ns| TestHost::get_named_time_zone_offset_nanoseconds(cx, "UTC", epoch_ns))
    });
    assert_eq!(answer, [0, 0, 0, 0]);
}

/// `get_named_time_zone_epoch_nanoseconds` (`#sec-getnamedtimezoneepochnanoseconds`, the
/// default: « GetUTCEpochNanoseconds(isoDateTime) »).
#[test]
fn host_124_get_named_time_zone_epoch_nanoseconds_default() {
    let date = |year, month, day, nanosecond| IsoDateTime {
        year,
        month,
        day,
        hour: 0,
        minute: 0,
        second: 0,
        millisecond: 0,
        microsecond: 0,
        nanosecond,
    };
    let answer = in_hook(|cx, _, _| {
        [
            TestHost::get_named_time_zone_epoch_nanoseconds(cx, "UTC", date(1970, 1, 1, 1)),
            TestHost::get_named_time_zone_epoch_nanoseconds(cx, "UTC", date(2000, 1, 1, 0)),
            TestHost::get_named_time_zone_epoch_nanoseconds(cx, "UTC", date(1969, 12, 31, 0)),
        ]
    });
    assert_eq!(
        answer,
        [
            vec![1],
            vec![946_684_800_000_000_000],
            vec![-86_400_000_000_000],
        ]
    );
}

/// `available_named_time_zone_identifiers` (`#sec-availablenamedtimezoneidentifiers`, "Return
/// « the Time Zone Identifier Record { [[Identifier]]: "UTC", [[PrimaryIdentifier]]: "UTC" } »").
#[test]
fn host_124_available_named_time_zone_identifiers_default() {
    let answer = in_hook(|cx, _, _| TestHost::available_named_time_zone_identifiers(cx));
    assert_eq!(
        answer,
        vec![TimeZoneIdentifier {
            identifier: "UTC".to_string(),
            primary: "UTC".to_string(),
        }]
    );
}
