//! `Math.random` (`#sec-math.random`, 21.3.2.27) over the generator of `swell_num::random`
//! (`docs/design/engine.md`, decision 6, "`Math.random`"): each realm's stream is a function
//! of the agent's `random_seed` and the realm's creation index alone, and a call reads nothing
//! from the host. HOST-26, HOST-27, HOST-144, HOST-147, HOST-212 and HOST-213
//! (`docs/design/host-interface.md`). Run also with the feature `collect-every-safe-point`.
//!
//! The values are read from the host, `Math.random` called through `Cx::call` in entries of at
//! most [`CHUNK`] calls, so that a count of 100,000 costs 100,000 steps of the call and no
//! loop of a script.
#![cfg(test)]

use crate::support;

use std::time::Instant;

use support::host::{TestHost, create_realm, get, get_object, limits, object, realm, seeds, text};
use support::script::{Outcome as Shown, eval, eval_text};
use support::stack::test_path;
use swell::{
    Abrupt, Agent, CompileStrings, Completion, Cx, GlobalObjects, Handled, HookCx, Host,
    IsoDateTime, Job, Limits, LoadRequest, ModuleRef, ObjectRef, Outcome, PropertyKey, Realm,
    RealmRef, RejectionOperation, Seeds, Source, StringRef, TimeZoneIdentifier, Value,
};

/// The most calls of `Math.random` one entry makes.
const CHUNK: usize = 1_000;

/// 2^53: a Number of the generator times it is an integer.
const TWO_TO_THE_53: f64 = 9_007_199_254_740_992.0;

/// An agent of the test host with `seeds` and the tests' limits.
fn agent_seeded(seeds: Seeds) -> Agent<TestHost> {
    Agent::new(TestHost::default(), limits(), seeds).unwrap()
}

/// The next `count` values of `Math.random()` of `realm`, called from the host with `Math` as
/// `this` and no argument, at most [`CHUNK`] calls in one entry.
fn stream(agent: &mut Agent<TestHost>, realm: &Realm, count: usize) -> Vec<f64> {
    let global_root = agent.global_object(realm).unwrap();
    #[expect(
        clippy::disallowed_methods,
        reason = "test code: the `count` values the test asks for"
    )]
    let mut values = Vec::with_capacity(count);
    while values.len() < count {
        let chunk = (count - values.len()).min(CHUNK);
        agent
            .with_cx(realm, |cx| {
                let global = object(cx, &global_root);
                let math = get_object(cx, global, "Math");
                let random = get(cx, math, "random");
                for _ in 0..chunk {
                    match cx.call(random, Value::Object(math), &[]).unwrap() {
                        Ok(Value::Number(number)) => values.push(number),
                        other => panic!("Math.random() answered {other:?}"),
                    }
                }
            })
            .unwrap();
    }
    values
}

/// The bits of each value, which compare a NaN and the sign of a zero too.
fn bits(values: &[f64]) -> Vec<u64> {
    values.iter().map(|value| value.to_bits()).collect()
}

/// A new realm of `agent`, which must be made.
fn new_realm(agent: &mut Agent<TestHost>) -> Realm {
    realm(agent)
}

/// HOST-26: the first 1,000 values of `Math.random()` in the first realm of two agents with the
/// same `Seeds` are equal bit for bit.
#[test]
fn host_26_two_agents_with_one_seed_agree_for_1000_values() {
    let mut a = agent_seeded(seeds());
    let mut b = agent_seeded(seeds());
    let realm_a = new_realm(&mut a);
    let realm_b = new_realm(&mut b);
    let values_a = stream(&mut a, &realm_a, 1_000);
    let values_b = stream(&mut b, &realm_b, 1_000);
    assert_eq!(bits(&values_a), bits(&values_b));
    assert!(values_a.windows(2).any(|pair| pair[0] != pair[1]));
}

/// HOST-27: the first 64 values of `Math.random()` in two realms of one agent differ as
/// sequences.
#[test]
fn host_27_two_realms_of_one_agent_differ_for_64_values() {
    let mut agent = agent_seeded(seeds());
    let first = new_realm(&mut agent);
    let second = new_realm(&mut agent);
    let values_first = stream(&mut agent, &first, 64);
    let values_second = stream(&mut agent, &second, 64);
    assert_ne!(bits(&values_first), bits(&values_second));
}

/// HOST-144: the first 64 values of the first realm of two agents that differ only in
/// `random_seed` differ as sequences, and are equal bit for bit in two agents that differ only
/// in `hash_key`.
#[test]
fn host_144_the_seed_changes_the_stream_and_the_hash_key_does_not() {
    let base = seeds();
    let mut other_seed = base;
    other_seed.random_seed[31] ^= 1;
    let mut other_key = base;
    other_key.hash_key[0] ^= 1;
    let [values_base, values_seed, values_key] = [base, other_seed, other_key].map(|seeds| {
        let mut agent = agent_seeded(seeds);
        let realm = new_realm(&mut agent);
        bits(&stream(&mut agent, &realm, 64))
    });
    assert_ne!(values_base, values_seed, "random_seed differs");
    assert_eq!(values_base, values_key, "hash_key differs");
}

/// The values HOST-212 checks: 100,000, the count of the requirement's sentence, in the default
/// mode, which holds the sentence; a reduced count of 1,000 with the feature
/// `collect-every-safe-point`, in the gate and in the lane `slow` alike (`SWELL_FULL_COUNTS`
/// does not raise it). There every call is a safe point that marks the whole heap: 1,000 calls
/// took 0.41 CPU seconds alone (build node, test profile), so the full count would take about
/// 41, over the bound of a test in that mode and near the kill at 60 s. The claim, a property
/// of each value, shows at any count.
fn host_212_count() -> usize {
    if cfg!(feature = "collect-every-safe-point") {
        1_000
    } else {
        100_000
    }
}

/// HOST-212: each of the first 100,000 values of `Math.random()` in a realm is a Number in
/// [0, 1) with positive sign and `x * 2 ** 53` is an integer. The calls are made from the host
/// in entries of 1,000 under `steps_per_entry` 5,000.
#[test]
fn host_212_100000_values_are_in_range_with_positive_sign_and_53_bits() {
    let count = host_212_count();
    let started = Instant::now();
    let mut agent = Agent::new(
        TestHost::default(),
        Limits {
            steps_per_entry: 5_000,
            ..limits()
        },
        seeds(),
    )
    .unwrap();
    let realm = new_realm(&mut agent);
    let values = stream(&mut agent, &realm, count);
    assert_eq!(values.len(), count);
    for (index, &x) in values.iter().enumerate() {
        assert!((0.0..1.0).contains(&x), "value {index}: {x:?}");
        assert!(x.is_sign_positive(), "value {index}: {x:?}");
        let scaled = x * TWO_TO_THE_53;
        assert_eq!(scaled, scaled.trunc(), "value {index}: {x:?}");
    }
    println!(
        "host_212: {count} values, {} ms",
        started.elapsed().as_millis()
    );
}

/// HOST-213: two agents with the same `random_seed` each make two realms; one calls the second
/// realm's function first and the other the first realm's. The second realm's first 64 values
/// are equal bit for bit between the agents and differ from those of the first realm of either:
/// a realm's stream is its creation index, not the order of calls.
#[test]
fn host_213_a_realms_stream_is_its_creation_index() {
    let mut a = agent_seeded(seeds());
    let first_a = new_realm(&mut a);
    let second_a = new_realm(&mut a);
    let second_of_a = bits(&stream(&mut a, &second_a, 64));
    let first_of_a = bits(&stream(&mut a, &first_a, 64));

    let mut b = agent_seeded(seeds());
    let first_b = new_realm(&mut b);
    let second_b = new_realm(&mut b);
    let first_of_b = bits(&stream(&mut b, &first_b, 64));
    let second_of_b = bits(&stream(&mut b, &second_b, 64));

    assert_eq!(second_of_a, second_of_b);
    assert_eq!(first_of_a, first_of_b);
    assert_ne!(second_of_a, first_of_a);
    assert_ne!(second_of_b, first_of_b);
}

/// HOST-213 and the design ("an index is never used twice"): a `create_realm` whose setup
/// throws uses index 0, so the next realm has index 1; its stream equals that of the second
/// realm of an agent whose first creation succeeded, and differs from that of its first.
#[test]
fn a_failed_creation_uses_its_index() {
    let mut failed = agent_seeded(seeds());
    let mut setup = |_cx: &mut HookCx<'_, TestHost>,
                     _realm: RealmRef|
     -> Completion<GlobalObjects> { Err(Abrupt::Throw(Value::Number(7.0))) };
    let created = create_realm(&mut failed, Some(&mut setup), None).unwrap();
    assert!(matches!(created, Outcome::Throw(_)), "{created:?}");
    let after = new_realm(&mut failed);
    let after_failure = bits(&stream(&mut failed, &after, 64));

    let mut made = agent_seeded(seeds());
    let first = new_realm(&mut made);
    let second = new_realm(&mut made);
    let second_of_made = bits(&stream(&mut made, &second, 64));
    let first_of_made = bits(&stream(&mut made, &first, 64));

    assert_eq!(after_failure, second_of_made);
    assert_ne!(after_failure, first_of_made);
}

/// `#sec-math.random` and clause 18: `Math.random` is { [[Writable]]: true, [[Enumerable]]:
/// false, [[Configurable]]: true }, its "length" is 0 and its "name" "random", it is no
/// constructor, it is the last String key of `Math`, after `sumPrecise`, it answers a Number,
/// and it ignores its arguments (a `valueOf` that throws is never called).
#[test]
fn a_script_reads_the_length_name_and_descriptor_of_math_random() {
    let mut agent = agent_seeded(seeds());
    let realm = new_realm(&mut agent);
    let cases = [
        (
            "var d = Object.getOwnPropertyDescriptor(Math, \"random\"); \
             [typeof d.value, d.writable, d.enumerable, d.configurable, \
              d.value.length, d.value.name].join()",
            "\"function,true,false,true,0,random\"",
        ),
        (
            "var n = Object.getOwnPropertyNames(Math); n.slice(n.length - 2).join()",
            "\"sumPrecise,random\"",
        ),
        (
            "var x = Math.random({ valueOf: function () { throw 1; } }, 2); \
             typeof x + ',' + (x >= 0 && x < 1)",
            "\"number,true\"",
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(
            eval_text(&mut agent, &realm, source),
            Shown::Normal(expected.to_string()),
            "{source}"
        );
    }
    assert_eq!(
        eval_text(&mut agent, &realm, "new Math.random();"),
        Shown::Throw("TypeError".to_string())
    );
}

/// A host that counts every hook call, the provided hooks included, and answers each with the
/// trait's default.
#[derive(Default)]
struct Counting {
    hooks: u32,
}

impl Counting {
    fn count(cx_host: &mut Counting) {
        cx_host.hooks += 1;
    }
}

impl Host for Counting {
    fn host_ensure_can_compile_strings(
        cx: &mut Cx<'_, Self>,
        _request: &CompileStrings<'_>,
    ) -> Completion<()> {
        Self::count(cx.host_mut());
        Ok(())
    }
    fn host_get_code_for_eval(cx: &mut HookCx<'_, Self>, _arg: ObjectRef) -> Option<StringRef> {
        Self::count(cx.host_mut());
        None
    }
    fn host_ensure_can_add_private_element(
        cx: &mut HookCx<'_, Self>,
        _obj: ObjectRef,
    ) -> Completion<()> {
        Self::count(cx.host_mut());
        Ok(())
    }
    fn host_has_source_text_available(cx: &mut HookCx<'_, Self>, _func: ObjectRef) -> bool {
        Self::count(cx.host_mut());
        true
    }
    fn host_make_job_callback(
        cx: &mut HookCx<'_, Self>,
        _callback: ObjectRef,
    ) -> Option<swell::HostDefined> {
        Self::count(cx.host_mut());
        None
    }
    fn host_call_job_callback(
        cx: &mut Cx<'_, Self>,
        callback: ObjectRef,
        _host_defined: Option<&swell::HostDefined>,
        this: Value,
        args: &[Value],
    ) -> Completion<Value> {
        Self::count(cx.host_mut());
        cx.call(Value::Object(callback), this, args).unwrap()
    }
    fn host_enqueue_promise_job(cx: &mut HookCx<'_, Self>, _realm: Option<RealmRef>) {
        Self::count(cx.host_mut());
    }
    fn host_prepare_to_run_job(cx: &mut HookCx<'_, Self>, _realm: RealmRef) {
        Self::count(cx.host_mut());
    }
    fn host_clean_up_after_job(
        cx: &mut HookCx<'_, Self>,
        _realm: Option<RealmRef>,
        _result: Result<(), Value>,
    ) {
        Self::count(cx.host_mut());
    }
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        Self::count(cx.host_mut());
    }
    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, _job: Job, _milliseconds: f64) {
        Self::count(cx.host_mut());
    }
    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, _job: Job) {
        Self::count(cx.host_mut());
    }
    fn host_promise_rejection_tracker(
        cx: &mut HookCx<'_, Self>,
        _promise: ObjectRef,
        _operation: RejectionOperation,
    ) {
        Self::count(cx.host_mut());
    }
    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, _request: LoadRequest) {
        Self::count(cx.host_mut());
    }
    fn host_get_import_meta_properties(
        cx: &mut HookCx<'_, Self>,
        _module: ModuleRef,
    ) -> Completion<Vec<(PropertyKey, Value)>> {
        Self::count(cx.host_mut());
        Ok(Vec::new())
    }
    fn host_finalize_import_meta(
        cx: &mut HookCx<'_, Self>,
        _import_meta: ObjectRef,
        _module: ModuleRef,
    ) {
        Self::count(cx.host_mut());
    }
    fn host_resize_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        Self::count(cx.host_mut());
        Ok(Handled::Unhandled)
    }
    fn host_grow_shared_array_buffer(
        cx: &mut HookCx<'_, Self>,
        _buffer: ObjectRef,
        _new_byte_length: u64,
    ) -> Completion<Handled> {
        Self::count(cx.host_mut());
        Ok(Handled::Unhandled)
    }
    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        Self::count(cx.host_mut());
        0
    }
    fn system_time_zone_identifier(cx: &mut HookCx<'_, Self>) -> String {
        Self::count(cx.host_mut());
        "UTC".to_string()
    }
    fn get_named_time_zone_offset_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _epoch_ns: i128,
    ) -> i128 {
        Self::count(cx.host_mut());
        0
    }
    fn get_named_time_zone_epoch_nanoseconds(
        cx: &mut HookCx<'_, Self>,
        _id: &str,
        _local: IsoDateTime,
    ) -> Vec<i128> {
        Self::count(cx.host_mut());
        Vec::new()
    }
    fn available_named_time_zone_identifiers(cx: &mut HookCx<'_, Self>) -> Vec<TimeZoneIdentifier> {
        Self::count(cx.host_mut());
        Vec::new()
    }
}

/// The steps and the hook calls of the script `source`, run in `realm` of `agent`.
fn steps_and_hooks(agent: &mut Agent<Counting>, realm: &Realm, source: &str) -> (u64, u32) {
    let swell::Parsed::Parsed(script) = agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    else {
        panic!("{source} did not parse");
    };
    let before = agent.host().hooks;
    let outcome = agent.evaluate_script(&script).unwrap();
    assert!(
        matches!(outcome, Outcome::Normal(_)),
        "{source}: {outcome:?}"
    );
    let steps = agent.last_entry_steps().unwrap();
    (steps, agent.host().hooks - before)
}

/// Decision 6, "`Math.random`": a call reads nothing from the host and costs the step of the
/// call. Under a host that counts every hook, `Math.random()` makes no hook call, and its steps
/// are those of `Math.max()`, a call of no argument that makes no loop: the step of the call
/// plus none.
#[test]
fn a_call_makes_no_hook_call_and_costs_one_step() {
    let mut agent = Agent::new(Counting::default(), limits(), seeds()).unwrap();
    let Outcome::Normal(realm) = create_realm(&mut agent, None, None).unwrap() else {
        panic!("create_realm did not make the realm");
    };
    let (random_steps, random_hooks) = steps_and_hooks(&mut agent, &realm, "Math.random();");
    let (max_steps, max_hooks) = steps_and_hooks(&mut agent, &realm, "Math.max();");
    assert_eq!(random_hooks, 0, "hook calls of Math.random()");
    assert_eq!(max_hooks, 0, "hook calls of Math.max()");
    assert_eq!(random_steps, max_steps, "Math.random() against Math.max()");
}

/// The variable that makes this test binary a child that prints the output of HOST-147's
/// script.
const CHILD: &str = "SWELL_MATH_RANDOM_CHILD";
/// The lines around the output a child prints.
const BEGIN: &str = "--- math_random output begin ---";
const END: &str = "--- math_random output end ---";

/// The counts of HOST-147's script, values of `Math.random()` and keys of the object: 1,000
/// and 10,000, the counts of the requirement's sentence, in the default mode, which holds the
/// sentence; reduced counts of 25 and 25 with the feature `collect-every-safe-point`, in the
/// gate and in the lane `slow` alike (`SWELL_FULL_COUNTS` does not raise them). There every
/// instruction of the script's loops is a safe point that marks the whole heap: at 100 and 100
/// one process's run of the script took 2.5 CPU seconds (build node, test profile), over the
/// bound of 2, and the full counts would be far over the kill at 60 s.
fn host_147_counts() -> (u32, u32) {
    if cfg!(feature = "collect-every-safe-point") {
        (25, 25)
    } else {
        (1_000, 10_000)
    }
}

/// The output of HOST-147's script in an agent of the tests' `Seeds`: the values of
/// `Math.random()`, each as `Number::toString` writes it, one per line, then the own keys of an
/// object made with a key per index, a third of them array indices, in the order
/// OrdinaryOwnPropertyKeys gives them.
fn host_147_output() -> String {
    let (values, keys) = host_147_counts();
    let source = format!(
        "var out = []; \
         for (var i = 0; i < {values}; i++) out.push(String(Math.random())); \
         var o = {{}}; \
         for (var j = 0; j < {keys}; j++) o[j % 3 === 0 ? String({keys} - j) : 'k' + j] = j; \
         out.push(Object.keys(o).join()); \
         out.join('\\n');"
    );
    let mut agent = agent_seeded(seeds());
    let realm = new_realm(&mut agent);
    let Outcome::Normal(root) = eval(&mut agent, &realm, &source) else {
        panic!("the script of HOST-147 did not complete normally");
    };
    let output = agent
        .with_cx(&realm, |cx| {
            let value = cx.get(&root).unwrap();
            text(cx, value)
        })
        .unwrap();
    assert_eq!(output.lines().count(), usize::try_from(values).unwrap() + 1);
    output
}

/// A child process that runs only HOST-147's test, started and not waited for.
fn spawn_child() -> std::process::Child {
    let test = test_path!("host_147_two_processes_print_the_same_bytes");
    std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", test.as_str(), "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap()
}

/// The output a child of [`spawn_child`] printed, once it ended.
fn child_output(child: std::process::Child) -> String {
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "the child failed: {}\n{stdout}\n{}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let start = stdout.find(BEGIN).map(|i| i + BEGIN.len() + 1);
    let end = stdout.find(END);
    match (start, end) {
        (Some(start), Some(end)) if start <= end => {
            stdout.get(start..end).unwrap().trim_end().to_string()
        }
        _ => panic!("the child printed no output:\n{stdout}"),
    }
}

/// HOST-147: two processes that each create an agent with the same `Seeds` and run the same
/// script, printing 1,000 `Math.random()` values and the key order of an object of 10,000 keys,
/// print the same bytes. The two children run beside each other and beside this process's own
/// agent, whose output they also equal.
#[test]
fn host_147_two_processes_print_the_same_bytes() {
    if std::env::var_os(CHILD).is_some() {
        let output = host_147_output();
        println!("{BEGIN}\n{output}\n{END}");
        return;
    }
    let started = Instant::now();
    let children = [spawn_child(), spawn_child()];
    let own = host_147_output();
    let [first, second] = children.map(child_output);
    assert_eq!(first, second, "the two child processes");
    assert_eq!(first, own, "a child process and this one");
    let (values, keys) = host_147_counts();
    println!(
        "host_147: {values} values and {keys} keys, {} bytes in three processes, {} ms",
        own.len(),
        started.elapsed().as_millis()
    );
}

/// The bits of the first `count` Numbers of the generator of creation index `n` for the tests'
/// `random_seed`, from `swell_num::random` directly.
fn expected_bits(n: u64, count: usize) -> Vec<u64> {
    let mut generator = swell_num::random::Xoshiro256::from_seed(&seeds().random_seed, n);
    (0..count).map(|_| generator.next_f64().to_bits()).collect()
}

/// The bits of the Numbers a script printed joined by commas, each read back from its shortest
/// round-trip text.
fn numbers_shown(shown: Shown) -> Vec<u64> {
    let Shown::Normal(text) = shown else {
        panic!("the script answered {shown:?}");
    };
    text.trim_matches('"')
        .split(',')
        .map(|number| number.parse::<f64>().unwrap().to_bits())
        .collect()
}

/// HOST-213 and the design (decision 6, "`Math.random`": the creation index n is "0 for the
/// first, and one more for each later call of `create_realm` that starts"): the stream of the
/// realm of creation index n is the generator `from_seed(random_seed, n)`, after allocations and
/// a collection, after a failed creation, after a realm was dropped and collected, and in an
/// agent of other limits and another `hash_key`. The other tests of this file compare two engine
/// streams with each other, which a derivation from `n + 1`, or any fixed shift of the index,
/// passes; this one pins the index.
#[test]
fn the_realm_of_creation_index_n_streams_from_seed_n() {
    let mut agent = agent_seeded(seeds());
    let r0 = new_realm(&mut agent);
    let made = eval_text(
        &mut agent,
        &r0,
        "var a = []; for (var i = 0; i < 20; i++) a.push({ x: i, s: 'k' + i }); a.length",
    );
    assert_eq!(made, Shown::Normal("20".to_string()));
    agent.collect().unwrap();
    let r1 = new_realm(&mut agent);
    let mut setup = |_cx: &mut HookCx<'_, TestHost>,
                     _realm: RealmRef|
     -> Completion<GlobalObjects> { Err(Abrupt::Throw(Value::Number(7.0))) };
    let failed = create_realm(&mut agent, Some(&mut setup), None).unwrap();
    assert!(matches!(failed, Outcome::Throw(_)), "{failed:?}");
    let r3 = new_realm(&mut agent);
    drop(r3);
    agent.collect().unwrap();
    let r4 = new_realm(&mut agent);
    assert_eq!(
        bits(&stream(&mut agent, &r4, 16)),
        expected_bits(4, 16),
        "realm 4"
    );
    assert_eq!(
        bits(&stream(&mut agent, &r1, 16)),
        expected_bits(1, 16),
        "realm 1"
    );
    assert_eq!(
        bits(&stream(&mut agent, &r0, 16)),
        expected_bits(0, 16),
        "realm 0"
    );

    let mut other = seeds();
    other.hash_key = [0xa5; 16];
    let mut b = Agent::new(
        TestHost::default(),
        Limits {
            memory_bytes: 8 * 1024 * 1024,
            ..limits()
        },
        other,
    )
    .unwrap();
    let first = new_realm(&mut b);
    assert_eq!(
        bits(&stream(&mut b, &first, 16)),
        expected_bits(0, 16),
        "agent b"
    );
}

/// `#sec-math.random` ("Each Math.random function created for distinct realms must produce a
/// distinct sequence of values from successive calls") with 9.4.1 and 10.3.1 (the realm of a
/// built-in function's execution context is its [[Realm]]): a call of another realm's
/// `Math.random`, from a script of this realm and from the host in an entry of this realm, takes
/// the next values of that realm's generator and moves no value of this realm's.
#[test]
fn another_realms_math_random_moves_only_its_stream() {
    let mut agent = agent_seeded(seeds());
    let r0 = new_realm(&mut agent);
    let r1 = new_realm(&mut agent);
    let global_0 = agent.global_object(&r0).unwrap();
    let global_1 = agent.global_object(&r1).unwrap();
    agent
        .with_cx(&r0, |cx| {
            let g0 = object(cx, &global_0);
            let g1 = object(cx, &global_1);
            let math_1 = get_object(cx, g1, "Math");
            let random_1 = get(cx, math_1, "random");
            let name = support::host::key(cx, "other");
            assert!(matches!(
                cx.set(g0, name, random_1, true).unwrap(),
                Ok(true)
            ));
        })
        .unwrap();
    let from_script = numbers_shown(eval_text(
        &mut agent,
        &r0,
        "var v = []; for (var i = 0; i < 8; i++) v.push(other()); v.join()",
    ));
    let expected_1 = expected_bits(1, 17);
    assert_eq!(
        from_script,
        expected_1[..8],
        "realm 1's values called from realm 0's script"
    );
    let from_host = agent
        .with_cx(&r0, |cx| {
            let g1 = object(cx, &global_1);
            let math_1 = get_object(cx, g1, "Math");
            let random_1 = get(cx, math_1, "random");
            match cx.call(random_1, Value::Object(math_1), &[]).unwrap() {
                Ok(Value::Number(number)) => number.to_bits(),
                other => panic!("Math.random() answered {other:?}"),
            }
        })
        .unwrap();
    assert_eq!(
        from_host, expected_1[8],
        "realm 1's function called in an entry of realm 0"
    );
    assert_eq!(
        bits(&stream(&mut agent, &r0, 8)),
        expected_bits(0, 8),
        "realm 0"
    );
    assert_eq!(
        bits(&stream(&mut agent, &r1, 8)),
        expected_1[9..17],
        "realm 1"
    );
}

/// `#sec-math.random` and clause 18: `call`, `apply`, `bind` and
/// `Function.prototype.call.call` each take the next value of the realm's stream with their
/// arguments ignored; `Math.random` has no "prototype", has `%Function.prototype%` as its
/// prototype and is extensible; `new` on a bound `Math.random` throws a TypeError and takes no
/// value of the stream.
#[test]
fn math_random_through_call_apply_bind_and_new() {
    let mut agent = agent_seeded(seeds());
    let realm = new_realm(&mut agent);
    let expected = expected_bits(0, 6);
    let through = numbers_shown(eval_text(
        &mut agent,
        &realm,
        "[Math.random(), Math.random.call(null, 1), Math.random.apply(undefined, [1, 2]), \
         Math.random.bind(null, 3)(), Function.prototype.call.call(Math.random)].join()",
    ));
    assert_eq!(through, expected[..5]);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "[Math.random.hasOwnProperty('prototype'), \
              Object.getPrototypeOf(Math.random) === Function.prototype, \
              Object.isExtensible(Math.random)].join()"
        ),
        Shown::Normal("\"false,true,true\"".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "new (Math.random.bind(null))();"),
        Shown::Throw("TypeError".to_string())
    );
    let next = numbers_shown(eval_text(&mut agent, &realm, "String(Math.random())"));
    assert_eq!(next, expected[5..6], "the value after a refused construct");
}

/// SipHash-2-4 of the Rust standard library (`std::hash::SipHasher`), independent of
/// `swell_num::siphash`, keyed by the 16 bytes as two little-endian words.
#[expect(
    deprecated,
    reason = "the standard library's SipHash-2-4 is the independent implementation the test compares with"
)]
fn std_siphash24(key: &[u8; 16], message: &[u8]) -> u64 {
    use std::hash::{Hasher, SipHasher};
    let k0 = u64::from_le_bytes(key[0..8].try_into().unwrap());
    let k1 = u64::from_le_bytes(key[8..16].try_into().unwrap());
    let mut hasher = SipHasher::new_with_keys(k0, k1);
    hasher.write(message);
    hasher.finish()
}

/// The bits of the first `count` Numbers of realm `index` of an agent seeded with `seed`, by
/// the design's text alone (decision 6, "`Math.random`"), with nothing of `swell_num`: the four
/// words by [`std_siphash24`] over `seed[16..32]`, `index` little-endian and the byte
/// i under the key `seed[0..16]`; the step of xoshiro256** as the design writes it; the Number
/// `(word >> 11) × 2^-53`.
fn bits_by_the_designs_text(seed: &[u8; 32], index: u64, count: usize) -> Vec<u64> {
    let key: [u8; 16] = seed[0..16].try_into().unwrap();
    let mut s: [u64; 4] = core::array::from_fn(|i| {
        let mut message = [0_u8; 25];
        message[..16].copy_from_slice(&seed[16..32]);
        message[16..24].copy_from_slice(&index.to_le_bytes());
        message[24] = u8::try_from(i).unwrap();
        std_siphash24(&key, &message)
    });
    if s == [0; 4] {
        s[0] = 1;
    }
    (0..count)
        .map(|_| {
            let [mut s0, mut s1, mut s2, mut s3] = s;
            let result = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
            let t = s1 << 17;
            s2 ^= s0;
            s3 ^= s1;
            s1 ^= s2;
            s0 ^= s3;
            s2 ^= t;
            s3 = s3.rotate_left(45);
            s = [s0, s1, s2, s3];
            ((result >> 11) as f64 / TWO_TO_THE_53).to_bits()
        })
        .collect()
}

/// The engine's `Math.random` against the design's text, end to end and
/// independent of `swell_num::random` and `swell_num::siphash`: for the all-zero seed, the
/// all-0xff seed and the seed `00..1f`, the first 16 values of realms 0 and 1 equal
/// [`bits_by_the_designs_text`].
#[test]
fn the_first_16_values_of_realms_0_and_1_equal_the_designs_text() {
    let counting: [u8; 32] = core::array::from_fn(|i| u8::try_from(i).unwrap());
    for random_seed in [[0_u8; 32], [0xff; 32], counting] {
        let mut agent = agent_seeded(Seeds {
            hash_key: [7; 16],
            random_seed,
        });
        let r0 = new_realm(&mut agent);
        let r1 = new_realm(&mut agent);
        assert_eq!(
            bits(&stream(&mut agent, &r1, 16)),
            bits_by_the_designs_text(&random_seed, 1, 16),
            "seed {:02x}{:02x} realm 1",
            random_seed[0],
            random_seed[1]
        );
        assert_eq!(
            bits(&stream(&mut agent, &r0, 16)),
            bits_by_the_designs_text(&random_seed, 0, 16),
            "seed {:02x}{:02x} realm 0",
            random_seed[0],
            random_seed[1]
        );
    }
}

/// The steps of the last entry and the change of `heap_bytes()` and `live_bytes()` over the
/// evaluation of `source` (parsed before the first reading).
fn steps_and_bytes(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> (u64, i128, i128) {
    let script = support::script::parse(agent, realm, source);
    let heap_before = agent.heap_bytes().unwrap();
    let live_before = agent.live_bytes().unwrap();
    let outcome = agent.evaluate_script(&script).unwrap();
    assert!(
        matches!(outcome, Outcome::Normal(_)),
        "{source}: {outcome:?}"
    );
    let steps = agent.last_entry_steps().unwrap();
    let heap_after = agent.heap_bytes().unwrap();
    let live_after = agent.live_bytes().unwrap();
    drop(outcome);
    (
        steps,
        i128::from(heap_after) - i128::from(heap_before),
        i128::from(live_after) - i128::from(live_before),
    )
}

/// Decision 6, "`Math.random`" ("costs the step of the call; it calls no hook
/// and charges nothing else") and the module text of `builtins/math.rs` ("costs the step of the
/// call and allocates nothing"), over a loop: 10,000 calls of `Math.random()` (40 with
/// `collect-every-safe-point`, where each instruction marks the heap) cost the steps of the same
/// loop over `Math.max()` and change `heap_bytes()` and `live_bytes()` by what that loop does;
/// and each call moved the stream by one value.
#[test]
fn a_loop_of_calls_costs_the_steps_and_bytes_of_a_loop_of_math_max() {
    let calls: usize = if cfg!(feature = "collect-every-safe-point") {
        40
    } else {
        10_000
    };
    let mut agent = agent_seeded(seeds());
    let realm = new_realm(&mut agent);
    assert_eq!(
        numbers_shown(eval_text(
            &mut agent,
            &realm,
            "var i = 0; Math.max(); String(Math.random())"
        )),
        expected_bits(0, 1)
    );
    let max = steps_and_bytes(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {calls}; i++) Math.max(); 0"),
    );
    let random = steps_and_bytes(
        &mut agent,
        &realm,
        &format!("for (i = 0; i < {calls}; i++) Math.random(); 0"),
    );
    println!("{calls} calls: Math.max {max:?}, Math.random {random:?} (steps, heap, live)");
    assert_eq!(random.0, max.0, "steps");
    assert_eq!(random.1, max.1, "heap_bytes");
    assert_eq!(random.2, max.2, "live_bytes");
    let next = numbers_shown(eval_text(&mut agent, &realm, "String(Math.random())"));
    assert_eq!(
        next,
        expected_bits(0, calls + 2)[calls + 1..],
        "the value after the loop"
    );
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static INTERRUPT_HANDLE: std::cell::Cell<Option<swell::InterruptHandle>> =
        const { std::cell::Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and returns `1`.
fn interrupt_now(_cx: &mut Cx<'_, TestHost>, _call: swell::HostCall<'_>) -> Completion<Value> {
    INTERRUPT_HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Number(1.0))
}

/// An entry that ends in `Terminated(Interrupted)` (HOST-96: the statement
/// after the host function does not run) keeps the values it took, and the next entry's
/// `Math.random()` is the next value of the realm's stream: the interrupted call that never ran
/// takes none.
#[test]
fn the_stream_resumes_after_an_interrupted_entry() {
    let mut agent = agent_seeded(seeds());
    let realm = new_realm(&mut agent);
    support::script::define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    INTERRUPT_HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    let expected = expected_bits(0, 3);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var a = [Math.random(), Math.random()]; interruptNow(); a.push(Math.random());"
        ),
        Shown::Terminated(swell::Termination::Interrupted)
    );
    assert_eq!(
        numbers_shown(eval_text(&mut agent, &realm, "a.join()")),
        expected[..2]
    );
    assert_eq!(
        numbers_shown(eval_text(&mut agent, &realm, "String(Math.random())")),
        expected[2..3],
        "the value after the interrupted entry"
    );
}
