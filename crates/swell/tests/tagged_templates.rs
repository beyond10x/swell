//! Tagged templates (`#sec-tagged-templates`): the tag called as a call is, with its this value,
//! the template object of the site and the substitutions in order
//! (`#sec-tagged-templates-runtime-semantics-evaluation`); GetTemplateObject
//! (`#sec-gettemplateobject`): one frozen object per site, kept with the script record that
//! holds the code and freed with it (`docs/design/engine.md`, decision 4, "Template objects").
//! Written by `story:tagged-templates`. Run also with the feature `collect-every-safe-point`.
//!
//! A tag reads its arguments as plain parameters or through `arguments`; no script uses an
//! array pattern or a spread.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, limits, realm, stack_with_room};
use support::script::{Outcome, describe, eval, eval_text, parse, run, run_both};
use support::stack::{BUILTINS_REENTRY, on_bounded_stack, reach_through, stack_limits, test_path};
use swell::{Agent, Outcome as HostOutcome, Realm, Root, Termination, Value};

/// What a script that checks its own results answers.
fn passed() -> Outcome {
    Outcome::Normal("true".to_string())
}

/// `source` in sloppy and in strict mode answers `true` in both.
fn both(source: &str) {
    let (sloppy, strict) = run_both(source);
    assert_eq!(sloppy, passed(), "sloppy: {source}");
    assert_eq!(strict, passed(), "strict: {source}");
}

/// Whether the lane `slow` asks for the full count (`SWELL_FULL_COUNTS=1`).
fn full_counts() -> bool {
    std::env::var("SWELL_FULL_COUNTS").is_ok_and(|value| value == "1")
}

/// A count of the gate: `full` in the default mode and at the full count, `collect` with the
/// feature `collect-every-safe-point`, where every instruction collects.
fn gate_count(full: u32, collect: u32) -> u32 {
    if cfg!(feature = "collect-every-safe-point") && !full_counts() {
        collect
    } else {
        full
    }
}

/// The root of what `source` completes with in `realm`, which must complete normally.
fn value_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Root {
    match eval(agent, realm, source) {
        HostOutcome::Normal(root) => root,
        HostOutcome::Throw(_) => panic!("{source} threw"),
        HostOutcome::Terminated(termination) => panic!("{source} ended in {termination:?}"),
    }
}

/// `#sec-gettemplateobject`, steps 1 to 3 (decision 4, "Template objects"): every evaluation of
/// a site gives the same object, from a function called many times and from each closure made
/// from one function in a loop; `eval` of one String twice and the `Function` constructor of
/// one text twice parse two sites, so they give two objects, while one function made by
/// `Function` gives one.
#[test]
fn a_site_gives_the_same_template_object_on_each_evaluation() {
    both(
        "function tag(s) { return s; }\n\
         function site() { return tag`a${0}b`; }\n\
         var first = site();\n\
         var same = true;\n\
         for (var i = 0; i < 100; i++) { if (site() !== first) same = false; }\n\
         var closures = [];\n\
         for (var j = 0; j < 3; j++) closures.push(function () { return tag`c`; });\n\
         var c0 = closures[0]();\n\
         var loop = [];\n\
         for (var k = 0; k < 3; k++) loop.push(tag`d${k}`);\n\
         var g = Function(\"t\", \"return t`f`;\");\n\
         var h = Function(\"t\", \"return t`f`;\");\n\
         same && first.length === 2 && closures[1]() === c0 && closures[2]() === c0\n\
           && loop[0] === loop[1] && loop[1] === loop[2]\n\
           && eval(\"tag`e`\") !== eval(\"tag`e`\")\n\
           && g(tag) === g(tag) && g(tag) !== h(tag);",
    );
}

/// `#sec-gettemplateobject`, step 3: the key is the site, not its text: two sites with the same
/// text in one script give two objects, and so do the same text in two scripts of one realm.
#[test]
fn two_sites_with_the_same_text_give_two_objects() {
    both(
        "function tag(s) { return s; }\n\
         function f() { return tag`x`; }\n\
         var a = tag`x`, b = tag`x`;\n\
         a !== b && f() !== a && f() !== b && f() === f() && a[0] === \"x\" && b[0] === \"x\";",
    );
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let first = "function tag(s) { return s; } var one = tag`x${1}`; one.length;";
    assert_eq!(
        eval_text(&mut agent, &realm, first),
        Outcome::Normal("2".to_string())
    );
    let second = "var two = tag`x${1}`; one !== two && one.raw !== two.raw;";
    assert_eq!(eval_text(&mut agent, &realm, second), passed());
}

/// `#sec-gettemplateobject`, steps 9 to 15: an Array of `%Array.prototype%`, each element not
/// writable, enumerable and not configurable, its `raw` not writable, not enumerable and not
/// configurable, an Array of the raw strings with the same attributes, both frozen; a write in
/// strict code throws a `TypeError` and changes nothing.
#[test]
fn the_template_object_and_its_raw_are_frozen_with_the_attributes_of_the_spec() {
    both(
        "function tag(s) { return s; }\n\
         var t = tag`a${0}b`;\n\
         var d0 = Object.getOwnPropertyDescriptor(t, \"0\");\n\
         var d1 = Object.getOwnPropertyDescriptor(t, \"1\");\n\
         var dr = Object.getOwnPropertyDescriptor(t, \"raw\");\n\
         var dl = Object.getOwnPropertyDescriptor(t, \"length\");\n\
         var r0 = Object.getOwnPropertyDescriptor(t.raw, \"0\");\n\
         var rl = Object.getOwnPropertyDescriptor(t.raw, \"length\");\n\
         var threw = (function () {\n\
           \"use strict\";\n\
           try { t[0] = \"z\"; return false; } catch (e) { return e instanceof TypeError; }\n\
         })();\n\
         var rawThrew = (function () {\n\
           \"use strict\";\n\
           try { t.raw.extra = 1; return false; } catch (e) { return e instanceof TypeError; }\n\
         })();\n\
         Array.isArray(t) && Array.isArray(t.raw) && Object.isFrozen(t) && Object.isFrozen(t.raw)\n\
           && Object.getPrototypeOf(t) === Array.prototype\n\
           && Object.getPrototypeOf(t.raw) === Array.prototype\n\
           && d0.value === \"a\" && !d0.writable && d0.enumerable && !d0.configurable\n\
           && d1.value === \"b\" && !d1.writable && d1.enumerable && !d1.configurable\n\
           && dr.value === t.raw && !dr.writable && !dr.enumerable && !dr.configurable\n\
           && dl.value === 2 && !dl.writable && !dl.enumerable && !dl.configurable\n\
           && r0.value === \"a\" && !r0.writable && r0.enumerable && !r0.configurable\n\
           && rl.value === 2 && !rl.writable\n\
           && Object.keys(t).join() === \"0,1\" && Object.keys(t.raw).join() === \"0,1\"\n\
           && threw && rawThrew && t[0] === \"a\" && !(\"extra\" in t.raw);",
    );
}

/// `#sec-static-semantics-tv` and `#sec-static-semantics-trv`: in a tagged template a
/// NotEscapeSequence is no early error (`#sec-static-semantics-template-early-errors` reads a
/// template without a tag only); the cooked string of its part is `undefined`, an own
/// property all the same, and its raw string is the source text, a `<CR><LF>` of it a `<LF>`.
#[test]
fn cooked_is_undefined_for_a_not_escape_sequence_and_raw_keeps_its_text() {
    both(
        "function tag(s) { return s; }\n\
         var t = tag`\\unicode and \\u{55}`;\n\
         var u = tag`a${1}\\xg${2}\\01${3}\\u{110000}`;\n\
         var v = tag`line\r\nend\\n`;\n\
         t.length === 1 && t[0] === undefined && (0 in t)\n\
           && t.raw[0] === \"\\\\unicode and \\\\u{55}\"\n\
           && u.length === 4 && u[0] === \"a\" && u[1] === undefined && u[2] === undefined\n\
           && u[3] === undefined && u.raw[1] === \"\\\\xg\" && u.raw[2] === \"\\\\01\"\n\
           && u.raw[3] === \"\\\\u{110000}\"\n\
           && v[0] === \"line\\nend\\n\" && v.raw[0] === \"line\\nend\\\\n\";",
    );
}

/// `#sec-tagged-templates-runtime-semantics-evaluation` and `#sec-evaluatecall`: a tag that is
/// a property reference is called with its base as this value, by name and by computed key; a
/// tag that is no reference gets `undefined` (the global object in sloppy code); a tag that
/// answers a function is applied again (`tag`a``b``).
#[test]
fn a_member_tag_is_called_with_its_base_as_this() {
    both(
        "var o = { f: function () { return this; } };\n\
         function strictThis() { \"use strict\"; return this; }\n\
         function twice(s) { return function (t) { return s[0] + t[0]; }; }\n\
         o.f`x` === o && o[\"f\"]`y` === o && (0, o.f)`w` !== o\n\
           && strictThis`z` === undefined && twice`a``b` === \"ab\";",
    );
    assert_eq!(
        run(
            limits(),
            "var o = { f: function () { return this; } };\n\
             with (o) { f`x` === o; }"
        ),
        passed()
    );
}

/// `#sec-tagged-templates-runtime-semantics-evaluation`: the tag's reference and its value
/// first (a getter runs), then the template object and the substitutions in source order, each
/// once, then the call, which gets them as its arguments; a tag that is not callable throws a
/// `TypeError` after the substitutions ran.
#[test]
fn the_substitutions_are_evaluated_after_the_tag_and_in_order() {
    both(
        "var log = [];\n\
         var o = { get tag() {\n\
           log.push(\"get\");\n\
           return function (s, a, b) { log.push(\"call \" + s.length + a + b + arguments.length); return this; };\n\
         } };\n\
         function sub(x) { log.push(\"sub\" + x); return x; }\n\
         var r = o.tag`a${sub(1)}b${sub(2)}c`;\n\
         var first = log.join();\n\
         log = [];\n\
         var n = null;\n\
         var caught = false;\n\
         try { n`${sub(3)}`; } catch (e) { caught = e instanceof TypeError; }\n\
         r === o && first === \"get,sub1,sub2,call 3123\" && caught && log.join() === \"sub3\";",
    );
}

/// `#sec-tagged-templates`: `new MemberExpression TemplateLiteral` applies the tag first, and
/// `new` calls what it answers, with the `Arguments` that follow when there are any.
#[test]
fn new_applies_the_tag_before_it_constructs() {
    both(
        "var got = [];\n\
         function C(x) { got.push(x); }\n\
         function tag(s) { got.push(s[0]); return C; }\n\
         var a = new tag`one`;\n\
         var b = new tag`two`(2);\n\
         a instanceof C && b instanceof C && got.join() === \"one,,two,2\";",
    );
}

/// The script of the charge tests: a site evaluated `n` times in a loop, its object kept in `t`
/// (for `n` 0 the site is never evaluated and `t` is `undefined`).
fn site_loop(n: u32) -> String {
    format!(
        "function tag(s) {{ return s; }}\n\
         var t;\n\
         for (var i = 0; i < {n}; i++) t = tag`a${{i}}b${{i}}c`;\n\
         typeof t;"
    )
}

/// The bytes the agent's heap is charged after `source` ran in a fresh agent and a collection,
/// with its outcome.
fn heap_after(source: &str) -> (Outcome, u64) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let completion = eval(&mut agent, &realm, source);
    let outcome = describe(&mut agent, &realm, completion);
    agent.collect().unwrap();
    (outcome, agent.heap_bytes().unwrap())
}

/// Decision 4, "Template objects", item 4: the two Arrays are made at the first evaluation of a
/// site and a later evaluation allocates nothing. `heap_bytes()` after a collection, with the
/// object kept: a site evaluated 10,000 times (100 in the collect mode, 10,000 at the full count)
/// holds over the site evaluated once less than the bytes of one template object, which is
/// what the site evaluated once holds over the site never evaluated.
#[test]
fn a_template_object_is_charged_once_per_site() {
    let n = gate_count(10_000, 100);
    let (none_outcome, none) = heap_after(&site_loop(0));
    let (one_outcome, one) = heap_after(&site_loop(1));
    let (many_outcome, many) = heap_after(&site_loop(n));
    assert_eq!(none_outcome, Outcome::Normal("\"undefined\"".to_string()));
    assert_eq!(one_outcome, Outcome::Normal("\"object\"".to_string()));
    assert_eq!(many_outcome, Outcome::Normal("\"object\"".to_string()));
    let object = one - none;
    println!("none {none} one {one} {n} evaluations {many}: one object {object} bytes");
    assert!(object > 0, "no bytes for one template object");
    assert!(
        many.saturating_sub(one) < object,
        "{n} evaluations hold {many} against {one} for one, one object {object}"
    );
}

/// A tag site of `parts` template strings, `parts - 1` substitutions of `0`.
fn site_of(parts: usize) -> String {
    let mut site = String::from("tag`p");
    for i in 1..parts {
        site.push_str(&format!("${{0}}p{i}"));
    }
    site.push('`');
    site
}

/// The steps of three entries that each call a function evaluating one site of `parts`
/// template strings: the first evaluation of the site, the second and the third.
fn steps_of_site(parts: usize) -> (u64, u64, u64) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let setup = format!(
        "function tag(s) {{ return s.length; }}\n\
         function site() {{ return {}; }}",
        site_of(parts)
    );
    assert_eq!(
        eval_text(&mut agent, &realm, &setup),
        Outcome::Normal("undefined".to_string())
    );
    let mut steps = [0; 3];
    for entry in &mut steps {
        let completion = eval(&mut agent, &realm, "site();");
        *entry = agent.last_entry_steps().unwrap();
        assert_eq!(
            describe(&mut agent, &realm, completion),
            Outcome::Normal(parts.to_string())
        );
    }
    let [first, second, third] = steps;
    (first, second, third)
}

/// Decision 4, "Template objects", item 4: the instruction's step, and at the first
/// evaluation of a site one per element it defines in the two Arrays. The first evaluation of a
/// site of 100 parts costs over its second at least 2 × 99 steps more than the first evaluation
/// of a site of 1 part costs over its second; each later evaluation costs what the second did.
#[test]
fn the_first_evaluation_charges_a_step_per_element() {
    let (first_many, second_many, third_many) = steps_of_site(100);
    let (first_one, second_one, third_one) = steps_of_site(1);
    println!(
        "100 parts: {first_many} {second_many} {third_many}; 1 part: {first_one} {second_one} {third_one}"
    );
    assert_eq!(second_many, third_many);
    assert_eq!(second_one, third_one);
    let made_many = first_many - second_many;
    let made_one = first_one - second_one;
    assert!(
        made_many >= made_one + 2 * 99,
        "making the object of 100 parts {made_many} steps, of 1 part {made_one}"
    );
}

/// The script of [`the_template_objects_are_freed_with_their_code`]: a site of 200 parts, in
/// the script itself (`evaluated` false: it is never evaluated), whose tag is a function
/// expression, so that no function object made from the script outlives its run.
fn record_script(evaluated: bool) -> String {
    format!(
        "var go = {evaluated};\n\
         if (go) {{ {}; }}\n\
         go;",
        site_of(200).replacen("tag", "(function (s) { return s.length; })", 1)
    )
}

/// `live_bytes()` after a collection with the script of [`record_script`] parsed, evaluated
/// and held by the host, and after the host dropped it and a collection freed its record.
fn live_with_and_without_record(evaluated: bool) -> (u64, u64) {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    let script = parse(&mut agent, &realm, &record_script(evaluated));
    let completion = agent.evaluate_script(&script).unwrap();
    assert_eq!(
        describe(&mut agent, &realm, completion),
        Outcome::Normal(evaluated.to_string())
    );
    agent.collect().unwrap();
    let held = agent.live_bytes().unwrap();
    drop(script);
    agent.collect().unwrap();
    (held, agent.live_bytes().unwrap())
}

/// Decision 4, "Template objects", items 2 and 3: the template objects are held by the record
/// of the code and freed with it. The template object of eval code that nothing references any
/// more is freed by `collect`: `heap_bytes()` after an eval whose tag drops the object is below
/// the same eval whose tag keeps it in a global by more than half its size. And a script the
/// host holds keeps the object of its evaluated site (`live_bytes()` over the same script whose
/// site is never evaluated); once the host drops it and a collection frees its record, both
/// read the same.
#[test]
fn the_template_objects_are_freed_with_their_code() {
    let eval_of = |tag: &str| {
        format!(
            "var kept;\n\
             function drop(s) {{ return s.length; }}\n\
             function keep(s) {{ kept = s; return s.length; }}\n\
             eval(\"{}\");",
            site_of(200).replacen("tag", tag, 1)
        )
    };
    let (dropped_outcome, dropped) = heap_after(&eval_of("drop"));
    let (kept_outcome, kept) = heap_after(&eval_of("keep"));
    assert_eq!(dropped_outcome, Outcome::Normal("200".to_string()));
    assert_eq!(kept_outcome, Outcome::Normal("200".to_string()));

    let (evaluated_held, evaluated_freed) = live_with_and_without_record(true);
    let (never_held, never_freed) = live_with_and_without_record(false);
    println!(
        "eval: dropped {dropped} kept {kept}; script evaluated {evaluated_held} then \
         {evaluated_freed}, never evaluated {never_held} then {never_freed}"
    );
    let object = evaluated_held.saturating_sub(never_held);
    assert!(object > 0, "the held record keeps no template object");
    assert!(
        kept > dropped + object / 2,
        "kept {kept} against dropped {dropped}, one object {object}"
    );
    assert_eq!(evaluated_freed, never_freed);
}

/// The script of [`many_sites_end_at_the_memory_budget`]: `n` evaluations of eval code, each a
/// site of its own, each object kept in `kept`.
fn many_sites(n: u32) -> String {
    format!(
        "var kept = [];\n\
         function tag(s) {{ kept.push(s); return s; }}\n\
         for (var i = 0; i < {n}; i++) eval(\"tag`a${{i}}b${{i}}c${{i}}d`\");\n\
         kept.length;"
    )
}

/// Decision 4, "Template objects", item 4: each template object is charged as any Array when
/// its site is first evaluated. 1,000 sites (100 in the collect mode, 1,000 at the full count),
/// each the site of one eval code, their objects kept: under a memory budget of the reading of
/// none of them, measured on an agent with the test limits, plus a room of half what the 1,000
/// added to it, the script ends in `Terminated(MemoryBudget)`, never in a hang, while the
/// script of none completes under the same budget.
#[test]
fn many_sites_end_at_the_memory_budget() {
    let n = gate_count(1_000, 100);
    let (none_outcome, none) = heap_after(&many_sites(0));
    let (kept_outcome, kept) = heap_after(&many_sites(n));
    assert_eq!(none_outcome, Outcome::Normal("0".to_string()));
    assert_eq!(kept_outcome, Outcome::Normal(n.to_string()));
    assert!(kept > none, "{n} objects hold {kept} against {none}");
    let room = (kept - none) / 2;
    let mut bounded = limits();
    bounded.memory_bytes = none + room;
    println!(
        "none {none} {n} sites {kept}: memory budget {} (room {room})",
        bounded.memory_bytes
    );
    assert_eq!(
        run(bounded, &many_sites(n)),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    assert_eq!(
        run(bounded, &many_sites(0)),
        Outcome::Normal("0".to_string())
    );
}

/// `#sec-gettemplateobject`, step 1, and decision 4, "Template objects", item 2: the template
/// object belongs to the realm of the code, the realm every frame of it runs in. A function of
/// realm `b` that evaluates a site, called from realm `a` by the host, answers an object whose
/// prototype is `%Array.prototype%` of `b`, not of `a`, the same object on each call.
#[test]
fn the_template_object_is_of_the_realm_of_its_code() {
    let mut agent = agent().unwrap();
    let a = realm(&mut agent);
    let b = realm(&mut agent);
    let site = value_of(
        &mut agent,
        &b,
        "(function () { return (function (s) { return s; })`x${0}`; });",
    );
    let undefined = agent
        .with_cx(&a, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    let call = |agent: &mut Agent<TestHost>| match agent.call(&a, &site, &undefined, &[]).unwrap() {
        HostOutcome::Normal(root) => root,
        _ => panic!("the site's function did not answer normally"),
    };
    let first = call(&mut agent);
    let second = call(&mut agent);
    let of_realm = |agent: &mut Agent<TestHost>, realm: &Realm| {
        let check = value_of(
            agent,
            realm,
            "(function (t, u) { return Object.getPrototypeOf(t) === Array.prototype \
               && Object.getPrototypeOf(t.raw) === Array.prototype && t === u; });",
        );
        let undefined = agent
            .with_cx(realm, |cx| cx.root(Value::Undefined).unwrap())
            .unwrap();
        match agent
            .call(realm, &check, &undefined, &[&first, &second])
            .unwrap()
        {
            HostOutcome::Normal(root) => agent.with_cx(realm, |cx| cx.get(&root).unwrap()).unwrap(),
            _ => panic!("the check did not answer normally"),
        }
    };
    assert_eq!(of_realm(&mut agent, &b), Value::Boolean(true));
    assert_eq!(of_realm(&mut agent, &a), Value::Boolean(false));
}

/// `Limits::min_stack_bytes` through a tagged template whose tag re-enters (decision 3, "Depth
/// bounds"): `String.raw` as the tag calls ToString of a substitution whose `toString` is `f`,
/// a call from Rust, so `f` re-enters itself through it until the re-entry limit, on seven
/// eighths of `min_stack_bytes()` (`AGENTS.md`, "Commands and the gate"), in a child process.
/// In the collect mode the depth is [`BUILTINS_REENTRY`].
#[test]
fn a_tag_re_enters_on_the_promised_stack() {
    fn body() {
        reach_through("String.raw`a${ { toString: f } }b`", BUILTINS_REENTRY);
    }
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(
        test_path!("a_tag_re_enters_on_the_promised_stack"),
        stack,
        body,
    )
    .unwrap();
}

/// A ledger with no bound that reads what it holds and its high-water mark.
#[derive(Default)]
struct HighWater {
    held: usize,
    high: usize,
}

impl swell_syntax::Ledger for HighWater {
    fn charge(&mut self, bytes: usize) -> Result<(), swell_syntax::Stop> {
        self.held += bytes;
        self.high = self.high.max(self.held);
        Ok(())
    }

    fn credit(&mut self, bytes: usize) {
        self.held -= bytes;
    }

    fn poll(&mut self) -> Result<(), swell_syntax::Stop> {
        Ok(())
    }
}

/// The frontend's ledger over a source of 10,000 tagged templates (1,000 in the collect mode,
/// whose build is slower; 10,000 at the full count), each with a substitution and a
/// NotEscapeSequence, in the script and in a function: when the parse returns the ledger holds
/// exactly what the tree holds by capacity, and while the compile step runs its high-water
/// mark covers the tree and the code with its site table, after which it holds exactly both.
#[test]
fn many_tagged_templates_are_charged_as_they_are_read() {
    let n = gate_count(10_000, 1_000);
    let sites: Vec<String> = (0..n).map(|i| format!("tag`a${{{i}}}\\xg`;")).collect();
    let text = format!(
        "{}\nfunction f() {{ {} }}",
        sites.join("\n"),
        sites.join(" ")
    );
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut ledger = HighWater::default();
    let options = swell_syntax::ParseOptions { max_nesting: 100 };
    let Ok(tree) = swell_syntax::parse_script_with(&units, options, &mut ledger) else {
        panic!("the source did not parse");
    };
    assert_eq!(ledger.held, tree.capacity_bytes(), "held after the parse");
    assert_eq!(tree.charged, tree.capacity_bytes());
    ledger.high = ledger.held;
    let Ok(code) = swell_compiler::compile_script_with(&tree, &mut ledger) else {
        panic!("the source did not compile");
    };
    let held = tree.charged + code.charged;
    println!(
        "{n} sites twice: tree {} code {} high-water {}",
        tree.charged, code.charged, ledger.high
    );
    assert_eq!(code.charged, code.capacity_bytes());
    assert!(
        ledger.high >= held,
        "high-water {} below {held}",
        ledger.high
    );
    assert_eq!(ledger.held, held, "the compile step credited its scratch");
}
