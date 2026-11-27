//! The set methods of `%Set.prototype%` (`story:set-methods`): `union`, `intersection`,
//! `difference`, `symmetricDifference`, `isSubsetOf`, `isSupersetOf` and `isDisjointFrom`
//! (`#sec-set.prototype.union` and the clauses beside it) over a set-like read by GetSetRecord
//! (`#sec-getsetrecord`): the order of the reads, the side each method iterates, the cursor of
//! the walk over `this`, IteratorClose where the clause closes, `-0` read from `keys()`, a plain
//! `Set` as the result, the step and memory budgets, the depth bound, the interrupt, the close
//! of the walk's cursor on a throw, and the keys of a result held across a collection. Run also
//! with the feature `collect-every-safe-point`.
#![cfg(test)]

use crate::support;

use support::host::{TestHost, agent, agent_with, limits, realm, stack_with_room};
use support::script::{Outcome, define_function, eval, eval_text, run_both};
use support::stack::{
    BUILTINS_REENTRY, TestPath, on_bounded_stack, reach_through, stack_limits, test_path,
};
use swell::{
    Agent, Completion, Cx, HostCall, InterruptHandle, Limits, Outcome as HostOutcome, Realm,
    Termination, Value,
};

/// `source` answers the String `expected` in sloppy and in strict code.
fn answers(source: &str, expected: &str) {
    let (sloppy, strict) = run_both(source);
    let expected = Outcome::Normal(format!("\"{expected}\""));
    assert_eq!(sloppy, expected, "sloppy: {source}");
    assert_eq!(strict, expected, "strict: {source}");
}

/// Whether the feature `collect-every-safe-point` is on: the heavy tests take their smaller
/// counts.
fn collect_mode() -> bool {
    cfg!(feature = "collect-every-safe-point")
}

/// The bytes of one place of the cursor list of an entry table (`story:keyed-entry-table`: "A
/// place of the cursor list is 32 bytes"), which a cursor left open keeps charged.
const CURSOR_PLACE_BYTES: u64 = 32;

/// The seven methods, in the order the scripts of these tests write them.
const METHODS: [&str; 7] = [
    "union",
    "intersection",
    "difference",
    "symmetricDifference",
    "isSubsetOf",
    "isSupersetOf",
    "isDisjointFrom",
];

/// `like(size, items)` is a set-like whose `has` answers whether `items` holds its argument and
/// whose `keys()` answers the elements of `items` in order; `log` records each call; `show(x)`
/// writes a Set or a Boolean.
const LIKE: &str = "var log = [];\n\
     function like(size, items) {\n\
       return { size: size,\n\
         has: function (v) { log.push(\"has \" + v); return items.indexOf(v) >= 0; },\n\
         keys: function () { log.push(\"keys\"); var i = 0;\n\
           return { next: function () { log.push(\"next\"); return i < items.length ? { value: items[i++], done: false } : { done: true }; } }; } };\n\
     }\n\
     function show(x) { return typeof x === \"boolean\" ? String(x) : \"[\" + [...x].join() + \"]\"; }\n";

/// Each method with two plain Sets (`#sec-set.prototype.union` and the clauses beside it): the
/// result in the order each clause fills it, the Booleans of the three predicates both ways,
/// `this` and the argument unchanged, and each function's `length` 1 and `name`.
#[test]
fn each_method_answers_its_result_in_insertion_order() {
    answers(
        &[
            LIKE,
            "var a = new Set([1, 2, 3, 4]), b = new Set([6, 4, 5, 3]), r = [];\n\
             r.push(show(a.union(b)), show(a.intersection(b)), show(a.difference(b)), show(a.symmetricDifference(b)),\n\
               show(a.isSubsetOf(b)), show(a.isSupersetOf(b)), show(a.isDisjointFrom(b)));\n\
             r.push(show(b.union(a)), show(b.intersection(a)), show(b.difference(a)), show(b.symmetricDifference(a)));\n\
             r.push(show(new Set([4, 3]).isSubsetOf(a)), show(a.isSupersetOf(new Set([3, 1]))), show(a.isDisjointFrom(new Set([7, 8]))));\n\
             r.push(show(a), show(b));\n\
             r.push([\"union\", \"intersection\", \"difference\", \"symmetricDifference\", \"isSubsetOf\", \"isSupersetOf\", \"isDisjointFrom\"]\n\
               .map(function (n) { var f = Set.prototype[n]; return f.name + \"/\" + f.length; }).join());\n\
             r.join(\"|\");",
        ]
        .concat(),
        "[1,2,3,4,6,5]|[3,4]|[1,2]|[1,2,6,5]|false|false|false|\
         [6,4,5,3,1,2]|[4,3]|[6,5]|[6,5,1,2]|true|true|true|[1,2,3,4]|[6,4,5,3]|\
         union/1,intersection/1,difference/1,symmetricDifference/1,isSubsetOf/1,isSupersetOf/1,isDisjointFrom/1",
    );
}

/// GetSetRecord (`#sec-getsetrecord`) for each of the seven methods: an argument that is not
/// an Object, a `size` that is `undefined` or NaN, a `has` or a `keys` that is not callable each
/// throw a `TypeError`, and a `size` below 0 a `RangeError`, each after exactly the reads before
/// it (a log of getters: `size`, its `valueOf`, `has`, `keys`); a this value that is not a Set
/// throws a `TypeError` before any read. `-0.5` is a size of 0, `"2"` and `Infinity` are sizes.
/// In sloppy and in strict code; in the collect mode in sloppy code only and for `union` and
/// `isDisjointFrom` only (in sloppy code, all seven took 3.7 CPU seconds there at opt-level 0):
/// the seven share one GetSetRecord, the method changes only the message, and no step of these
/// clauses depends on the mode (with a NaN size accepted, the two fail there). Every method is
/// covered in the default mode.
#[test]
fn get_set_record_rejects_a_nan_size_a_negative_size_and_a_non_callable_has_or_keys() {
    let methods: &[&str] = if collect_mode() {
        &["union", "isDisjointFrom"]
    } else {
        &METHODS
    };
    let list: Vec<String> = methods.iter().map(|name| format!("\"{name}\"")).collect();
    let source = format!(
        "var methods = [{}];\n{}",
        list.join(", "),
        GET_SET_RECORD_ERRORS
    );
    let errors = "TypeError ; TypeError get size; TypeError get size,valueOf; TypeError get size; \
                  RangeError get size,valueOf; RangeError get size; TypeError get size,get has; \
                  TypeError get size,get has,get keys; TypeError ; TypeError ";
    let mut expected: Vec<String> = methods
        .iter()
        .map(|name| format!("{name}: {errors}"))
        .collect();
    expected.extend(
        [
            "none get size,valueOf,get has,get keys",
            "none get size,get has,get keys,has",
            "none get size,get has,get keys",
        ]
        .map(String::from),
    );
    let expected = Outcome::Normal(format!("\"{}\"", expected.join(" | ")));
    if collect_mode() {
        assert_eq!(support::script::run(limits(), &source), expected);
    } else {
        let (sloppy, strict) = run_both(&source);
        assert_eq!(sloppy, expected, "sloppy");
        assert_eq!(strict, expected, "strict");
    }
}

/// The script of [`get_set_record_rejects_a_nan_size_a_negative_size_and_a_non_callable_has_or_keys`],
/// over the names in `methods`.
const GET_SET_RECORD_ERRORS: &str = "var log = [];\n\
         function rec(size, has, keys) {\n\
           var o = {};\n\
           Object.defineProperty(o, \"size\", { get: function () { log.push(\"get size\"); return size; } });\n\
           Object.defineProperty(o, \"has\", { get: function () { log.push(\"get has\"); return has; } });\n\
           Object.defineProperty(o, \"keys\", { get: function () { log.push(\"get keys\"); return keys; } });\n\
           return o;\n\
         }\n\
         function f() {}\n\
         function sized(n) { return { valueOf: function () { log.push(\"valueOf\"); return n; } }; }\n\
         function caught(g) {\n\
           log = [];\n\
           try { g(); return \"none \" + log.join(\",\"); }\n\
           catch (e) { return (e instanceof RangeError ? \"RangeError\" : e instanceof TypeError ? \"TypeError\" : e) + \" \" + log.join(\",\"); }\n\
         }\n\
         var s = new Set([1]), out = [];\n\
         methods.forEach(function (m) {\n\
           var p = Set.prototype[m];\n\
           out.push(m + \": \" + [\n\
             caught(function () { p.call(s, 1); }),\n\
             caught(function () { p.call(s, rec(undefined, f, f)); }),\n\
             caught(function () { p.call(s, rec(sized(NaN), f, f)); }),\n\
             caught(function () { p.call(s, rec(\"x\", f, f)); }),\n\
             caught(function () { p.call(s, rec(sized(-1), f, f)); }),\n\
             caught(function () { p.call(s, rec(-Infinity, f, f)); }),\n\
             caught(function () { p.call(s, rec(1, 1, f)); }),\n\
             caught(function () { p.call(s, rec(1, f, {})); }),\n\
             caught(function () { p.call({}, rec(1, f, f)); }),\n\
             caught(function () { p.call(new Map([[1, 1]]), rec(1, f, f)); })\n\
           ].join(\"; \"));\n\
         });\n\
         out.push(caught(function () { s.isSubsetOf(rec(sized(-0.5), f, f)); }),\n\
           caught(function () { s.isSubsetOf(rec(\"2\", function () { log.push(\"has\"); return true; }, f)); }),\n\
           caught(function () { s.isSupersetOf(rec(Infinity, f, f)); }));\n\
         out.join(\" | \");";

/// `#sec-set.prototype.intersection`, step 5 against step 6, and the same choice of
/// `difference`, `isDisjointFrom`, `isSubsetOf` and `isSupersetOf`: with `this` not larger than
/// the other's `size`, `has` is called for each entry of `this` and `keys` never; with `this`
/// larger, `keys()` is iterated and `has` never, and the result follows the order of the keys,
/// a key answered twice once. `isSubsetOf` with `this` larger and `isSupersetOf` with `this`
/// smaller answer false with no call.
#[test]
fn intersection_iterates_the_smaller_side() {
    answers(
        &[
            LIKE,
            "var s = new Set([1, 2, 3]), r = [];\n\
             function run(f) { log = []; var x = f(); return show(x) + \" \" + log.join(\",\"); }\n\
             r.push(run(function () { return s.intersection(like(3, [3, 1])); }));\n\
             r.push(run(function () { return s.intersection(like(2, [3, 9, 3, 1])); }));\n\
             r.push(run(function () { return s.difference(like(3, [2])); }));\n\
             r.push(run(function () { return s.difference(like(1, [2])); }));\n\
             r.push(run(function () { return s.isDisjointFrom(like(5, [9])); }));\n\
             r.push(run(function () { return s.isDisjointFrom(like(1, [9])); }));\n\
             r.push(run(function () { return s.isSubsetOf(like(2, [1, 2, 3])); }));\n\
             r.push(run(function () { return s.isSupersetOf(like(4, [1])); }));\n\
             r.join(\"|\");",
        ]
        .concat(),
        "[1,3] has 1,has 2,has 3|[3,1] keys,next,next,next,next,next|\
         [1,3] has 1,has 2,has 3|[1,3] keys,next,next|\
         true has 1,has 2,has 3|true keys,next,next|false |false ",
    );
}

/// The walk over `this` of `isSubsetOf`, `intersection` and `isDisjointFrom`
/// (`#sec-set.prototype.issubsetof`, steps 5 to 7, and the like) is a cursor of its table: an
/// entry the `has` callback deletes before the walk reaches it is not visited, one it adds is,
/// one deleted after its visit and added again is visited again, and a compaction of the table
/// inside the walk (16 entries, 11 deleted and one added) keeps the walk on its next entry.
/// `difference` walks its copy of `this` (step 5.c reads resultSetData), so deletions from
/// `this` do not change what it visits.
#[test]
fn a_has_callback_that_deletes_from_this_is_seen_by_the_cursor() {
    answers(
        "var r = [];\n\
         function mutating(s, seen, answer) {\n\
           return { size: 100, keys: function () {}, has: function (v) {\n\
             seen.push(v);\n\
             if (v === 1) { s.delete(2); s.add(5); }\n\
             if (v === 3) { s.delete(1); s.add(1); }\n\
             return answer(v);\n\
           } };\n\
         }\n\
         var seen = [], s = new Set([1, 2, 3, 4]);\n\
         r.push(s.isSubsetOf(mutating(s, seen, function () { return true; })), seen.join());\n\
         seen = []; s = new Set([1, 2, 3, 4]);\n\
         r.push([...s.intersection(mutating(s, seen, function () { return true; }))].join(), seen.join());\n\
         seen = []; s = new Set([1, 2, 3, 4]);\n\
         r.push(s.isDisjointFrom(mutating(s, seen, function () { return false; })), seen.join());\n\
         seen = []; s = new Set([1, 2, 3, 4]);\n\
         r.push([...s.difference(mutating(s, seen, function (v) { return v % 2 === 1; }))].join(), seen.join(), [...s].join());\n\
         var big = new Set(), i; for (i = 0; i < 16; i++) big.add(i);\n\
         seen = [];\n\
         r.push(big.isSubsetOf({ size: 100, keys: function () {}, has: function (v) {\n\
           seen.push(v);\n\
           if (v === 1) { for (var d = 2; d < 13; d++) big.delete(d); big.add(100); }\n\
           return true;\n\
         } }), seen.join(), [...big].join());\n\
         r.join(\"|\");",
        "true|1,3,4,5,1|1,3,4,5|1,3,4,5,1|true|1,3,4,5,1|2,4|1,2,3,4|3,4,5,1|\
         true|0,1,13,14,15,100|0,1,13,14,15,100",
    );
}

/// IteratorClose (`#sec-iteratorclose`) where the clauses close: `isSupersetOf` closes the
/// iterator of `keys()` once when it reads a key `this` lacks, after which `next` is not called
/// again, and `isDisjointFrom` (with `this` larger) when it reads a key `this` holds; at the end
/// of the keys neither closes. A `return` that answers a non-object throws a `TypeError`, and a
/// throw of `return` is what the method throws.
#[test]
fn is_superset_of_closes_the_iterator_when_it_finds_a_missing_key() {
    answers(
        "var closes = 0, nexts = 0;\n\
         function like(size, items, ret) {\n\
           return { size: size, has: function () { return false; }, keys: function () {\n\
             var i = 0;\n\
             return { next: function () { nexts++; return i < items.length ? { value: items[i++], done: false } : { done: true }; },\n\
               \"return\": ret || function () { closes++; return {}; } };\n\
           } };\n\
         }\n\
         function run(f) { nexts = 0; try { return f() + \" \" + nexts + \" \" + closes; } catch (e) { return (e instanceof TypeError ? \"TypeError\" : e) + \" \" + nexts + \" \" + closes; } }\n\
         var s = new Set([1, 2, 3]), r = [];\n\
         r.push(run(function () { return s.isSupersetOf(like(2, [1, 9, 2])); }));\n\
         r.push(run(function () { return s.isSupersetOf(like(2, [1, 2])); }));\n\
         r.push(run(function () { return s.isDisjointFrom(like(2, [7, 2, 8])); }));\n\
         r.push(run(function () { return s.isDisjointFrom(like(2, [7, 8])); }));\n\
         r.push(run(function () { return s.isSupersetOf(like(1, [9], function () { closes++; return 1; })); }));\n\
         r.push(run(function () { return s.isDisjointFrom(like(1, [3], function () { closes++; throw \"ret\"; })); }));\n\
         r.join(\"|\");",
        "false 2 1|true 3 1|false 2 2|true 3 2|TypeError 1 3|ret 1 4",
    );
}

/// CanonicalizeKeyedCollectionKey (`#sec-canonicalizekeyedcollectionkey`) of a key read from a
/// set-like's `keys()`: `-0` is found as the key `+0` of `this` and enters a result as `+0`,
/// in `union`, `intersection`, `difference`, `symmetricDifference`, `isSupersetOf` and
/// `isDisjointFrom`.
#[test]
fn keys_read_from_a_set_like_are_canonicalized() {
    answers(
        "function like(items) { return { size: 1, has: function () { return false; }, keys: function () {\n\
           var i = 0; return { next: function () { return i < items.length ? { value: items[i++], done: false } : { done: true }; } }; } }; }\n\
         function keys(s) { return [...s].map(function (k) { return Object.is(k, -0) ? \"-0\" : String(k); }).join(); }\n\
         var r = [];\n\
         r.push(keys(new Set().union(like([-0]))), keys(new Set([0]).union(like([-0]))));\n\
         r.push(keys(new Set([0, 1]).intersection(like([-0]))));\n\
         r.push(keys(new Set([0, 1]).difference(like([-0]))));\n\
         r.push(keys(new Set([1]).symmetricDifference(like([-0]))), keys(new Set([0, 1]).symmetricDifference(like([-0]))));\n\
         r.push(new Set([0]).isSupersetOf(like([-0])), new Set([0, 1]).isDisjointFrom(like([-0])));\n\
         r.join(\"|\");",
        "0|0|0|1|1,0|1|true|false",
    );
}

/// The result of `union`, `intersection`, `difference` and `symmetricDifference` is
/// OrdinaryObjectCreate(%Set.prototype%) filled in place: a new plain `Set` whatever `this` is
/// (an instance of a subclass-like `F`), with no read of `constructor` or of `@@species` of
/// `this`, of `F` or of `Set`, and no call of `add`.
#[test]
fn the_result_is_a_plain_set_whatever_the_species() {
    answers(
        "var reads = 0, other = new Set([2, 3]);\n\
         function F() {} F.prototype = Object.create(Set.prototype);\n\
         Object.defineProperty(F, Symbol.species, { get: function () { reads++; return F; } });\n\
         var s = Reflect.construct(Set, [[1, 2]], F);\n\
         Object.defineProperty(s, \"constructor\", { get: function () { reads++; return F; } });\n\
         Object.defineProperty(Set, Symbol.species, { get: function () { reads++; return F; } });\n\
         var add = Set.prototype.add;\n\
         Set.prototype.add = function (v) { reads++; return add.call(this, v); };\n\
         var results = [s.union(other), s.intersection(other), s.difference(other), s.symmetricDifference(other)];\n\
         Set.prototype.add = add;\n\
         results.map(function (x) {\n\
           return (x !== s && Object.getPrototypeOf(x) === Set.prototype ? \"plain\" : \"other\") + \" \" + [...x].join();\n\
         }).join(\"|\") + \"|\" + reads + \"|\" + (Object.getPrototypeOf(s) === F.prototype);",
        "plain 1,2,3|plain 2|plain 1|plain 1,3|0|true",
    );
}

/// An agent with [`limits`] changed by `change` and its realm.
fn agent_and_realm(change: impl FnOnce(&mut Limits)) -> (Agent<TestHost>, Realm) {
    let mut limits = limits();
    change(&mut limits);
    let mut agent = agent_with(limits).unwrap();
    let realm = realm(&mut agent);
    (agent, realm)
}

/// Run `source` in `agent`, which must answer normally, and answer `last_entry_steps()`.
fn steps_of(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> u64 {
    let outcome = eval(agent, realm, source);
    let steps = agent.last_entry_steps().unwrap();
    assert!(matches!(outcome, HostOutcome::Normal(_)), "{outcome:?}");
    steps
}

/// A set-like whose `keys()` never ends: `next` answers 0, 1, 2, ... and never done, in one
/// result object it reuses, so a key read makes no object of the script's.
const ENDLESS: &str = "var endless = { size: 0, has: function () { return false; }, keys: function () {\n\
       var i = 0, result = { value: 0, done: false };\n\
       return { next: function () { result.value = i++; return result; } }; } };";

/// The step budget (`docs/design/engine.md`, "The step budget"): `union` of a set-like whose
/// `keys()` never ends, with `steps_per_entry` N (100,000; 2,000 in the collect mode), ends in
/// `Terminated(StepBudget)`, and `last_entry_steps()` is at most N. Then the charges the story
/// names, each held: a walk over `this` costs at least two steps per entry (the entry's own and
/// the call of `has`, here the builtin `Boolean`), and a key read from `keys()` of a Set at least
/// four (the key's own, the call of `next`, the entry `next` visits and the insertion into the
/// result), and an entry of the copy of `this` at least two (the entry's own and its
/// insertion), each read as the difference of `last_entry_steps()` between a run over 2K and
/// one over K (K 200; 20 in the collect mode, where the part with 200 took 4.6 of the test's
/// 5.3 CPU seconds in the test profile; the differences are exact at either K). With any of
/// the three steps of `set/methods.rs` taken out, its difference is K less, and the bound
/// fails. The steps are printed.
#[test]
fn union_with_an_endless_set_like_ends_at_the_step_budget() {
    let budget = if collect_mode() { 2_000 } else { 100_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = budget);
    let outcome = eval_text(
        &mut agent,
        &realm,
        &format!("{ENDLESS}\nnew Set([1, 2]).union(endless);"),
    );
    let steps = agent.last_entry_steps().unwrap();
    println!("{steps} steps of {budget}");
    assert_eq!(outcome, Outcome::Terminated(Termination::StepBudget));
    assert!(steps <= budget, "{steps} steps of {budget}");

    let k: u64 = if collect_mode() { 20 } else { 200 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    steps_of(
        &mut agent,
        &realm,
        &format!(
            "var small = new Set(), large = new Set(), i;\n\
             for (i = 1; i <= {k}; i++) small.add(i);\n\
             for (i = 1; i <= 2 * {k}; i++) large.add(i);\n\
             var boolean = {{ size: 1e9, has: Boolean, keys: function () {{}} }};"
        ),
    );
    let walk: Vec<u64> = ["small", "large"]
        .iter()
        .map(|set| steps_of(&mut agent, &realm, &format!("{set}.isSubsetOf(boolean);")))
        .collect();
    let read: Vec<u64> = ["small", "large"]
        .iter()
        .map(|set| steps_of(&mut agent, &realm, &format!("new Set().union({set});")))
        .collect();
    let copy: Vec<u64> = ["small", "large"]
        .iter()
        .map(|set| steps_of(&mut agent, &realm, &format!("{set}.union(new Set());")))
        .collect();
    println!(
        "steps over {k} and {} entries: isSubsetOf {walk:?}, union reading the keys {read:?}, \
         union copying this {copy:?}",
        2 * k
    );
    assert!(walk[1] - walk[0] >= 2 * k, "walk {walk:?}");
    assert!(read[1] - read[0] >= 4 * k, "read {read:?}");
    assert!(copy[1] - copy[0] >= 2 * k, "copy {copy:?}");
}

/// Collect and answer `heap_bytes()` after it.
fn collected(agent: &mut Agent<TestHost>) -> u64 {
    assert!(matches!(agent.collect().unwrap(), HostOutcome::Normal(_)));
    agent.heap_bytes().unwrap()
}

/// The memory budget (`docs/design/engine.md`, "Memory budget"): `union` of a set-like whose
/// `keys()` never ends, in a function's scope, with a budget of the realm's reading plus
/// 256 KiB (32 KiB in the collect mode, where 64 KiB took 2.1 CPU seconds at opt-level 0; with
/// 16 KiB there the first script itself ended at the budget), after a first script of the same
/// shape over 8 keys that grows the arenas a script needs, fills its result until
/// `Terminated(MemoryBudget)`; once the host dropped the script, one `collect()` brings
/// `heap_bytes()` back within the reading before the script, the result and its table
/// credited. `next` reuses one result object, so the object arena does not grow with the keys
/// (an arena's capacity stays charged after a collection). The step budget is half the room in
/// bytes, about 4.5 times the steps the fill takes in either mode (28,747 at 256 KiB, 3,659 at
/// 32 KiB), so a growth of the table that is not charged ends the run at the step budget, not
/// after the test's 60 s. The readings and the steps are printed.
#[test]
fn union_with_an_endless_set_like_ends_at_the_memory_budget() {
    let room: u64 = if collect_mode() { 32 << 10 } else { 256 << 10 };
    let warmed = |agent: &mut Agent<TestHost>, realm: &Realm| {
        assert_eq!(
            eval_text(
                agent,
                realm,
                "(function () { var eight = { size: 0, has: function () { return false; }, keys: function () {\n\
                   var i = 0, result = { value: 0, done: false };\n\
                   return { next: function () { result.value = i; result.done = i++ >= 8; return result; } }; } };\n\
                   new Set([1, 2]).union(eight); })();"
            ),
            Outcome::Normal("undefined".to_string())
        );
        collected(agent)
    };
    let realm_reading = {
        let (mut probe, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
        warmed(&mut probe, &realm)
    };
    let (mut agent, realm) = agent_and_realm(|limits| {
        limits.steps_per_entry = room / 2;
        limits.memory_bytes = realm_reading + room;
    });
    let before = warmed(&mut agent, &realm);
    assert_eq!(before, realm_reading, "two agents made alike");
    let script = support::script::parse(
        &mut agent,
        &realm,
        &format!("(function () {{ {ENDLESS}\nnew Set([1, 2]).union(endless); }})();"),
    );
    let outcome = agent.evaluate_script(&script).unwrap();
    let at_the_end = agent.heap_bytes().unwrap();
    let steps = agent.last_entry_steps().unwrap();
    assert!(
        matches!(outcome, HostOutcome::Terminated(Termination::MemoryBudget)),
        "{outcome:?} after {steps} steps"
    );
    drop(script);
    let after = collected(&mut agent);
    println!(
        "heap_bytes(): {before} before the script, budget {}, {at_the_end} at its end, {after} \
         after collect(); {steps} steps",
        before + room
    );
    assert!(at_the_end > before + room / 2, "{at_the_end}");
    assert!(after <= before, "{after} after collect(), {before} before");
}

/// Run `body`, a [`reach_through`] at [`BUILTINS_REENTRY`], on seven eighths of
/// `min_stack_bytes()` (decision 3, "Depth bounds") in a child process: the form of every test
/// `*_on_the_promised_stack` of this module, one per way the methods call the set-like's code
/// from Rust (the getter of `size`, `has`, `keys`, `next`, `return`), each for the method whose
/// frames on that way are the largest. What they hold, read from the stack per re-entry
/// measured by bisection of the thread stack at depths 200 and 1,000 in the dev profile (the
/// fixed part, the need at 200 less 200 re-entries, is about 152,000 bytes): in the default
/// mode, depth 1,000 on 12,838,560 bytes, a test fails above about 12,690 bytes per re-entry.
/// In the collect mode the depth is 50, on 921,760 bytes, and a test fails only above about
/// 15,390 bytes per re-entry, more than the promise itself (`STACK_BYTES_PER_REENTRY`, 14,336):
/// there no test of this form can fail for a frame over the bound, and the default mode holds
/// it.
fn on_the_promised_stack(test: TestPath, body: fn()) {
    let stack = stack_with_room(stack_limits(BUILTINS_REENTRY));
    on_bounded_stack(test, stack, body).unwrap();
}

/// `Limits::min_stack_bytes` through `isSubsetOf` (decision 3, "Depth bounds"): the set-like's
/// `has` is a call from Rust, a re-entry, so `f` re-enters itself through it until the re-entry
/// limit, on seven eighths of `min_stack_bytes()`, in a child process
/// ([`on_the_promised_stack`]): 12,463 bytes per re-entry when the seven methods ran in one
/// function, 9,648 after.
#[test]
fn is_subset_of_reaches_the_reentry_limit_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set([0]).isSubsetOf({ size: 1, has: f, keys() {} })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!("is_subset_of_reaches_the_reentry_limit_on_the_promised_stack"),
        body,
    );
}

/// The same through `has` for `intersection`, whose frames on this way are the largest (with
/// those of `difference`): 12,463 bytes per re-entry before, 9,824 after
/// ([`on_the_promised_stack`]).
#[test]
fn intersection_reaches_the_reentry_limit_through_has_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set([0]).intersection({ size: 1, has: f, keys() {} })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!("intersection_reaches_the_reentry_limit_through_has_on_the_promised_stack"),
        body,
    );
}

/// The same through `keys`, which `intersection` calls from Rust with `this` larger than the
/// set-like (GetIteratorFromMethod, `#sec-set.prototype.intersection` step 6.a); its frames on
/// this way are the largest (with those of `difference`): 11,792 bytes per re-entry before,
/// 9,503 after ([`on_the_promised_stack`]).
#[test]
fn intersection_reaches_the_reentry_limit_through_keys_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set([0]).intersection({ size: 0, has() {}, keys: f })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!("intersection_reaches_the_reentry_limit_through_keys_on_the_promised_stack"),
        body,
    );
}

/// The same through `next` for `intersection`, whose frames on this way are the largest (with
/// those of `difference`): 13,568 bytes per re-entry before, 10,287 after
/// ([`on_the_promised_stack`]).
#[test]
fn intersection_reaches_the_reentry_limit_through_next_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set([0]).intersection({ size: 0, has() {}, keys() { return { next: f }; } })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!("intersection_reaches_the_reentry_limit_through_next_on_the_promised_stack"),
        body,
    );
}

/// The same through `return`, which `isDisjointFrom` calls from Rust when a key of the set-like
/// is in `this` (IteratorClose, `#sec-set.prototype.isdisjointfrom` step 5.b.iii.1); its frames
/// on this way are larger than those of `isSupersetOf`, the other method that closes:
/// 13,375 bytes per re-entry before, 9,999 after ([`on_the_promised_stack`]).
#[test]
fn is_disjoint_from_reaches_the_reentry_limit_through_return_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set([0]).isDisjointFrom({ size: 0, has() {}, keys() {\n\
               return { next() { return { value: 0, done: false }; }, return: f };\n\
             } })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!(
            "is_disjoint_from_reaches_the_reentry_limit_through_return_on_the_promised_stack"
        ),
        body,
    );
}

thread_local! {
    /// The handle `interrupt_now` sets.
    static HANDLE: std::cell::Cell<Option<InterruptHandle>> = const { std::cell::Cell::new(None) };
}

/// A host function that sets its agent's interrupt flag and returns `1`, counting its calls.
fn interrupt_now(cx: &mut Cx<'_, TestHost>, _call: HostCall<'_>) -> Completion<Value> {
    cx.host_mut().count += 1;
    HANDLE.with(|slot| {
        if let Some(handle) = slot.take() {
            handle.interrupt();
        }
    });
    Ok(Value::Number(1.0))
}

/// The interrupt (`docs/design/host-interface.md`, decision 12): set by a host function the
/// `has` callback of `isSubsetOf` calls, it ends the run with `Terminated(Interrupted)` after
/// the host function returns; `has` ran once, and the Set keeps its entries. The agent runs a
/// next entry, in which the same Set answers `intersection`.
#[test]
fn a_has_callback_ends_at_an_interrupt() {
    let mut agent = agent().unwrap();
    let realm = realm(&mut agent);
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "var n = 0, s = new Set([1, 2, 3]);\n\
             s.isSubsetOf({ size: 10, has: function () { n++; interruptNow(); n++; return true; }, keys: function () {} });"
        ),
        Outcome::Terminated(Termination::Interrupted)
    );
    assert_eq!(agent.host().count, 1);
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "n + \",\" + s.size + \",\" + [...s.intersection(new Set([3, 1]))].join();"
        ),
        Outcome::Normal("\"1,3,3,1\"".to_string())
    );
}

/// The walk's cursor is closed on an abrupt exit: on one Set `s` of 3 entries, N times each of
/// `s.intersection(o)`, `s.difference(o)`, `s.isSubsetOf(o)` and `s.isDisjointFrom(o)` inside
/// `try`/`catch`, with `o = { size: 10, has() { throw 0; }, keys() {} }`, so each walks with a
/// cursor and the first `has` throws a primitive, the `0` each `catch` counts. `live_bytes()`
/// read after a `collect()` before
/// the calls and after them rises by less than N / 10 places of the cursor list
/// ([`CURSOR_PLACE_BYTES`]). A walk's cursor has no owner, so no collection releases one a throw
/// left: the cursors of `intersection`, `isSubsetOf` and `isDisjointFrom` lie in `s`'s table and
/// would add 3N places; that of `difference` lies in the table of its result, which the throw
/// leaves unreachable, so the collection frees it with its table. N is 1,000, and 10 in the
/// collect mode, the smallest N with a bound above 0 (100 took 3.8 to 5.2 CPU seconds there at
/// opt-level 0); at 10, with the cursor left open on a throw, `live_bytes()` rose there by
/// 1,024 bytes against the bound of 32.
#[test]
fn a_throwing_callback_leaves_no_cursor() {
    let n: u64 = if collect_mode() { 10 } else { 1_000 };
    let (mut agent, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    steps_of(
        &mut agent,
        &realm,
        "var s = new Set([0, 1, 2]), j, thrown = 0;\n\
         var o = { size: 10, has: function () { throw 0; }, keys: function () {} };",
    );
    collected(&mut agent);
    let before = agent.live_bytes().unwrap();
    steps_of(
        &mut agent,
        &realm,
        &format!(
            "for (j = 0; j < {n}; j++) {{\n\
               try {{ s.intersection(o); }} catch (e) {{ if (e === 0) thrown++; }}\n\
               try {{ s.difference(o); }} catch (e) {{ if (e === 0) thrown++; }}\n\
               try {{ s.isSubsetOf(o); }} catch (e) {{ if (e === 0) thrown++; }}\n\
               try {{ s.isDisjointFrom(o); }} catch (e) {{ if (e === 0) thrown++; }}\n\
             }}"
        ),
    );
    collected(&mut agent);
    let after = agent.live_bytes().unwrap();
    assert_eq!(
        eval_text(&mut agent, &realm, "thrown;"),
        Outcome::Normal((4 * n).to_string())
    );
    println!("live_bytes() {before} before {n} rounds of four throws, {after} after");
    assert!(
        after < before + n / 10 * CURSOR_PLACE_BYTES,
        "live_bytes() {before} before, {after} after"
    );
}

/// The keys of a result are held where a root reaches them while the clause fills it (the
/// result `Set` made first and kept): with a collection at every safe point of the loop and of
/// each callback in the mode `collect-every-safe-point`, and each callback allocating an Array of
/// 100 elements, `union` and `symmetricDifference` of a set-like whose `next` answers 50 objects
/// `{ i }` it makes and that nothing else references; `intersection` of a Set of 50 objects with
/// a set-like of `size` 1 whose `next` answers each object of `this` in turn and deletes from
/// `this` the one it answered before; and `difference` with a `has` that deletes its argument
/// from `this` and answers false. After each call the result holds every key the clause adds,
/// in its order, each the object the script made, read back by its `i`. The story's 50 keys in
/// the default mode; 2 in the collect mode (50 took 3.7 to 5.9 CPU seconds there at opt-level
/// 0), where with the result not kept on the kept stack the run ended at 2 in
/// `Terminated(EngineFault)`.
#[test]
fn the_result_keeps_its_keys_across_a_collection() {
    let count = if collect_mode() { 2 } else { 50 };
    let all: Vec<String> = (0..count).map(|i| i.to_string()).collect();
    let all = all.join(",");
    let outcome = support::script::run(
        {
            let mut limits = limits();
            limits.steps_per_entry = 1 << 40;
            limits
        },
        &format!(
            "var count = {count};\n{}",
            "function junk() { return new Array(100).fill(0); }\n\
         function made(count) {\n\
           return { size: 0, has: function () { junk(); return false; }, keys: function () {\n\
             junk(); var i = 0;\n\
             return { next: function () { junk(); if (i >= count) return { done: true }; var o = { i: i }; i++; return { value: o, done: false }; } };\n\
           } };\n\
         }\n\
         function back(set) { var out = []; set.forEach(function (o) { out.push(o.i); }); return out.join(); }\n\
         function fill() { var s = new Set(), k; for (k = 0; k < count; k++) s.add({ i: k }); return s; }\n\
         var r = [];\n\
         r.push(back(new Set().union(made(count))));\n\
         r.push(back(new Set().symmetricDifference(made(count))));\n\
         var t = fill(), it = t.values(), prev;\n\
         r.push(back(t.intersection({ size: 1, has: function () { return false; }, keys: function () {\n\
           return { next: function () {\n\
             junk(); var n = it.next();\n\
             if (prev !== undefined) t.delete(prev);\n\
             if (n.done) return { done: true };\n\
             prev = n.value; return { value: n.value, done: false };\n\
           } };\n\
         } })), t.size);\n\
         var d = fill();\n\
         r.push(back(d.difference({ size: 100, keys: function () {}, has: function (o) { junk(); d.delete(o); return false; } })), d.size);\n\
         r.join(\"|\");"
        ),
    );
    assert_eq!(
        outcome,
        Outcome::Normal(format!("\"{all}|{all}|{all}|0|{all}|0\""))
    );
}

/// The steps of `{walked}.clear()` less those of `{control}.clear()`, two
/// Sets of the same 3 entries: `clear` charges one step per place of the cursor list
/// (`object/keyed_entries.rs`, `EntryTable::clear`), and a table whose last cursor was closed
/// has no list, so the difference counts the places a cursor left open keeps.
fn cursor_places_left(
    agent: &mut Agent<TestHost>,
    realm: &Realm,
    walked: &str,
    control: &str,
) -> i64 {
    let walked_steps = steps_of(agent, realm, &format!("{walked}.clear();"));
    let control_steps = steps_of(agent, realm, &format!("{control}.clear();"));
    i64::try_from(walked_steps).unwrap() - i64::try_from(control_steps).unwrap()
}

/// Every way out of a walk closes its cursor, read exactly by the
/// steps of `clear` ([`cursor_places_left`]) on a long-lived Set: a normal answer of each walk
/// (true, and false early), the result of `difference` kept by the script (its cursor lies on
/// that result's table), a throw of `has`, the depth refusal (a `RangeError` the script
/// catches), the step budget, the interrupt, and the memory budget (a second agent).
#[test]
fn every_way_out_of_a_walk_closes_its_cursor() {
    let budget = if collect_mode() { 2_000 } else { 100_000 };
    let (mut agent, realm) = agent_and_realm(|limits| {
        limits.steps_per_entry = budget;
        limits.reentry_depth = 16;
    });
    define_function(&mut agent, &realm, "interruptNow", 0, interrupt_now, false);
    steps_of(
        &mut agent,
        &realm,
        "var s1 = new Set([0, 1, 2]), c1 = new Set([0, 1, 2]), s2 = new Set([0, 1, 2]), c2 = new Set([0, 1, 2]);\n\
         var s3 = new Set([0, 1, 2]), c3 = new Set([0, 1, 2]), s4 = new Set([0, 1, 2]), c4 = new Set([0, 1, 2]);\n\
         var s5 = new Set([0, 1, 2]), c5 = new Set([0, 1, 2]), s0 = new Set([0, 1, 2]), dw, dc;\n\
         var yes = { size: 10, has: function () { return true; }, keys: function () {} };\n\
         var no = { size: 10, has: function () { return false; }, keys: function () {} };\n\
         var none = { size: 0, has: function () { return false; }, keys: function () { return { next: function () { return { done: true }; } }; } };\n\
         var thrower = { size: 10, has: function () { throw 0; }, keys: function () {} };\n\
         var deep = { size: 10, has: function () { return s3.isSubsetOf(deep); }, keys: function () {} };\n\
         var spin = { size: 10, has: function () { for (;;) {} }, keys: function () {} };\n\
         var stop = { size: 10, has: function () { interruptNow(); return true; }, keys: function () {} };",
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "[s1.intersection(yes).size, s1.intersection(no).size, s1.isSubsetOf(yes), s1.isSubsetOf(no),\n\
              s1.isDisjointFrom(yes), s1.isDisjointFrom(no)].join() + \"|\" +\n\
             [(dw = s0.difference(no)).size, (dc = s0.difference(none)).size].join();"
        ),
        Outcome::Normal("\"3,0,true,false,false,true|3,3\"".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "[s2.intersection, s2.isSubsetOf, s2.isDisjointFrom].map(function (m) {\n\
               try { m.call(s2, thrower); return \"none\"; } catch (e) { return e; } }).join();"
        ),
        Outcome::Normal("\"0,0,0\"".to_string())
    );
    assert_eq!(
        eval_text(
            &mut agent,
            &realm,
            "try { s3.isSubsetOf(deep); \"none\"; } catch (e) { e instanceof RangeError; }"
        ),
        Outcome::Normal("true".to_string())
    );
    assert_eq!(
        eval_text(&mut agent, &realm, "s4.isSubsetOf(spin);"),
        Outcome::Terminated(Termination::StepBudget)
    );
    HANDLE.with(|slot| slot.set(Some(agent.interrupt_handle())));
    assert_eq!(
        eval_text(&mut agent, &realm, "s5.intersection(stop);"),
        Outcome::Terminated(Termination::Interrupted)
    );
    let left: Vec<i64> = [
        ("s1", "c1"),
        ("dw", "dc"),
        ("s2", "c2"),
        ("s3", "c3"),
        ("s4", "c4"),
        ("s5", "c5"),
    ]
    .iter()
    .map(|(walked, control)| cursor_places_left(&mut agent, &realm, walked, control))
    .collect();

    println!(
        "places left: normal {}, difference kept {}, throw {}, depth {}, steps {}, interrupt {}",
        left[0], left[1], left[2], left[3], left[4], left[5]
    );
    assert_eq!(
        left,
        [0, 0, 0, 0, 0, 0],
        "normal, difference, throw, depth, steps, interrupt"
    );
    // The memory budget, in the default mode only (the way out is the one of the step budget;
    // a fill of the room collecting at every safe point took most of 3.6 s): the reading of an
    // agent made alike after the same first script, so the arenas that script grows are grown
    // before the room is set.
    if collect_mode() {
        return;
    }
    let room: u64 = 256 << 10;
    let setup = "var s6 = new Set([0, 1, 2]), c6 = new Set([0, 1, 2]);\n\
         var fill = { size: 10, has: function () { var big = new Set(), i = 0; for (;;) big.add(i++); }, keys: function () {} };";
    let reading = {
        let (mut probe, realm) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
        steps_of(&mut probe, &realm, setup);
        collected(&mut probe)
    };
    let (mut agent, realm) = agent_and_realm(|limits| {
        limits.steps_per_entry = 1 << 40;
        limits.memory_bytes = reading + room;
    });
    steps_of(&mut agent, &realm, setup);
    assert_eq!(
        eval_text(&mut agent, &realm, "s6.isDisjointFrom(fill);"),
        Outcome::Terminated(Termination::MemoryBudget)
    );
    let memory = cursor_places_left(&mut agent, &realm, "s6", "c6");
    println!("places left: memory {memory}");
    assert_eq!(memory, 0, "memory");
}

/// The order of every observable read, after the test262 cases
/// `set-like-class-order.js` (on the list `close-later.tsv` since they define a `class`; here
/// with accessors on plain objects): GetSetRecord (`size`, its `valueOf`, `has`, `keys`), then
/// at a `size` equal to `this`'s the walk with `has` (`intersection`, `difference`,
/// `isDisjointFrom`, `isSubsetOf`) and for `isSupersetOf` the keys; with `this` larger the
/// keys: `keys()`, `next` read once, then per key `next()`, `done`, `value`; IteratorClose
/// reads `return` and calls it where `isDisjointFrom` and `isSupersetOf` stop early, and
/// neither when `next` throws nor when it answers a primitive. In the collect mode the six
/// cases at an equal size and of one close only (all 15 took 2.1 to 2.7 CPU seconds there in
/// the test profile; no step of these clauses depends on the mode).
#[test]
fn every_read_of_a_set_like_in_the_order_of_the_clauses() {
    let p = "size,valueOf,get has,get keys";
    let o = "keys,get next";
    let key = "next,done,value";
    let end = "next,done";
    let close = "get return,return";
    let cases = [
        (
            "\"intersection\", 3, [\"c\", \"x\", \"a\"]",
            format!("[a,c] {p},has a,has b,has c"),
        ),
        (
            "\"intersection\", 2, [\"c\", \"x\", \"a\"]",
            format!("[c,a] {p},{o},{key},{key},{key},{end}"),
        ),
        (
            "\"difference\", 3, [\"c\", \"x\"]",
            format!("[a,b] {p},has a,has b,has c"),
        ),
        (
            "\"difference\", 2, [\"c\", \"x\"]",
            format!("[a,b] {p},{o},{key},{key},{end}"),
        ),
        (
            "\"isDisjointFrom\", 3, [\"x\", \"y\"]",
            format!("true {p},has a,has b,has c"),
        ),
        (
            "\"isDisjointFrom\", 3, [\"b\"]",
            format!("false {p},has a,has b"),
        ),
        (
            "\"isDisjointFrom\", 2, [\"x\", \"b\", \"y\"]",
            format!("false {p},{o},{key},{key},{close}"),
        ),
        (
            "\"isSubsetOf\", 3, [\"a\", \"b\", \"c\"]",
            format!("true {p},has a,has b,has c"),
        ),
        (
            "\"isSubsetOf\", 3, [\"a\", \"c\"]",
            format!("false {p},has a,has b"),
        ),
        (
            "\"isSupersetOf\", 3, [\"a\", \"b\", \"c\"]",
            format!("true {p},{o},{key},{key},{key},{end}"),
        ),
        (
            "\"isSupersetOf\", 2, [\"a\", \"z\"]",
            format!("false {p},{o},{key},{key},{close}"),
        ),
        (
            "\"union\", 0, [\"d\", \"a\"]",
            format!("[a,b,c,d] {p},{o},{key},{key},{end}"),
        ),
        (
            "\"symmetricDifference\", 9, [\"d\", \"a\"]",
            format!("[b,c,d] {p},{o},{key},{key},{end}"),
        ),
        (
            "\"isSupersetOf\", 1, [\"a\", \"THROW\"]",
            format!("next threw {p},{o},{key},next"),
        ),
        (
            "\"isDisjointFrom\", 1, [\"x\", \"PRIM\"]",
            format!("TypeError {p},{o},{key},next"),
        ),
    ];
    let taken: Vec<usize> = if collect_mode() {
        vec![0, 2, 4, 5, 6, 9]
    } else {
        (0..cases.len()).collect()
    };
    let calls: Vec<String> = taken
        .iter()
        .map(|&i| format!("run({})", cases[i].0))
        .collect();
    let expected: Vec<&str> = taken.iter().map(|&i| cases[i].1.as_str()).collect();
    let source = format!("{}\n[{}].join(\"|\");", ORDER_LIKE, calls.join(", "));
    answers_once_when_collecting(&source, &expected.join("|"));
}

/// The set-like of [`every_read_of_a_set_like_in_the_order_of_the_clauses`]: every read and
/// call logged, `THROW` and `PRIM` as items make `next` throw or answer a primitive.
const ORDER_LIKE: &str = "var log = [], s = new Set([\"a\", \"b\", \"c\"]);\n\
         function getter(o, name, f) { Object.defineProperty(o, name, { get: f }); return o; }\n\
         function like(size, items) {\n\
           var o = {};\n\
           getter(o, \"size\", function () { log.push(\"size\"); return { valueOf: function () { log.push(\"valueOf\"); return size; } }; });\n\
           getter(o, \"has\", function () { log.push(\"get has\"); return function (v) { log.push(\"has \" + v); return items.indexOf(v) >= 0; }; });\n\
           getter(o, \"keys\", function () { log.push(\"get keys\"); return function () {\n\
             log.push(\"keys\"); var i = 0, it = {};\n\
             getter(it, \"next\", function () { log.push(\"get next\"); return function () {\n\
               log.push(\"next\");\n\
               if (items[i] === \"THROW\") { i++; throw \"next threw\"; }\n\
               if (items[i] === \"PRIM\") { i++; return 1; }\n\
               var r = {};\n\
               getter(r, \"done\", function () { log.push(\"done\"); return i >= items.length; });\n\
               return getter(r, \"value\", function () { log.push(\"value\"); return items[i++]; });\n\
             }; });\n\
             return getter(it, \"return\", function () { log.push(\"get return\"); return function () { log.push(\"return\"); return {}; }; });\n\
           }; });\n\
           return o;\n\
         }\n\
         function run(m, size, items) {\n\
           log = []; var x;\n\
           try { x = s[m](like(size, items)); x = typeof x === \"boolean\" ? String(x) : \"[\" + [...x].join() + \"]\"; }\n\
           catch (e) { x = typeof e === \"string\" ? e : e instanceof TypeError ? \"TypeError\" : String(e); }\n\
           return x + \" \" + log.join(\",\");\n\
         }";

/// [`answers`], in the collect mode in sloppy code only (no step of these clauses depends on
/// the mode; both took 5.2 CPU seconds there in the test profile).
fn answers_once_when_collecting(source: &str, expected: &str) {
    if collect_mode() {
        assert_eq!(
            support::script::run(limits(), source),
            Outcome::Normal(format!("\"{expected}\"")),
            "{source}"
        );
    } else {
        answers(source, expected);
    }
}

/// The copy of `this` in `union` and `symmetricDifference` is made
/// after `keys()` (`#sec-set.prototype.union` steps 4 and 5), so a `keys` that deletes and adds
/// to `this` is seen in the result; `difference` copies at its step 4, before `keys()`, and
/// `intersection` reads `this` live at each key. A `next` that deletes from `this` a key it
/// then yields again makes `symmetricDifference` append it (not in `this`, not in the result
/// any more); `intersection`'s walk sees a `clear()` in `has` and the keys it adds after.
#[test]
fn a_set_like_that_mutates_this_during_the_call() {
    answers_once_when_collecting(
        "var s, r = [];\n\
         function after(size, items, mutate, step) {\n\
           return { size: size, has: function (v) { return items.indexOf(v) >= 0; }, keys: function () {\n\
             mutate(); var i = 0;\n\
             return { next: function () { if (step) step(i); return i < items.length ? { value: items[i++], done: false } : { done: true }; } };\n\
           } };\n\
         }\n\
         function show(x) { return [...x].join(); }\n\
         s = new Set([1, 2]); r.push(show(s.union(after(0, [5], function () { s.delete(1); s.add(3); }))));\n\
         s = new Set([1, 2]); r.push(show(s.symmetricDifference(after(0, [2, 5], function () { s.delete(1); s.add(3); }))));\n\
         s = new Set([1, 2, 3]); r.push(show(s.difference(after(1, [2], function () { s.delete(1); s.add(4); }))));\n\
         s = new Set([1, 2, 3]); r.push(show(s.intersection(after(1, [1, 2, 4], function () { s.delete(1); s.add(4); }))));\n\
         s = new Set([1, 2]); r.push(show(s.symmetricDifference(after(0, [2, 2], function () {}, function (i) { if (i === 1) s.delete(2); }))));\n\
         s = new Set([1, 2, 3]); r.push(s.isSupersetOf(after(2, [1, 2], function () {}, function (i) { if (i === 1) s.delete(2); })));\n\
         s = new Set([1, 2, 3]); var once = false, seen = [];\n\
         r.push(show(s.intersection({ size: 10, keys: function () {}, has: function (v) {\n\
           seen.push(v); if (!once) { once = true; s.clear(); s.add(7); s.add(1); } return true; } })), seen.join());\n\
         r.join(\"|\");",
        "2,3,5|3,5|1,3|2,4|1,2|false|1,7|1,7,1",
    );
}

/// GetSetRecord's conversions (`#sec-getsetrecord` steps 2 to 7)
/// on sizes the test of its errors does not give: a Symbol (TypeError after the read of `size`), a
/// `valueOf` that throws (its throw), a `valueOf` that empties `this` (the size of `this` is
/// read after it), `-0` (a size of 0), 2**53 and a String with spaces; a Proxy as the argument
/// (its `get` trap sees `size`, `has`, `keys` in that order) and a Proxy of a Set as `this`
/// (a TypeError before any read).
#[test]
fn get_set_record_converts_each_kind_of_size() {
    answers_once_when_collecting(
        "var log = [];\n\
         function rec(size, has, keys) {\n\
           var o = {};\n\
           Object.defineProperty(o, \"size\", { get: function () { log.push(\"size\"); return size; } });\n\
           o.has = has; o.keys = keys || function () { log.push(\"keys\"); return { next: function () { return { done: true }; } }; };\n\
           return o;\n\
         }\n\
         function f() { log.push(\"has\"); return true; }\n\
         function caught(g) { log = []; try { return \"none \" + g() + \" \" + log.join(); }\n\
           catch (e) { return (typeof e === \"string\" ? e : e instanceof TypeError ? \"TypeError\" : e instanceof RangeError ? \"RangeError\" : String(e)) + \" \" + log.join(); } }\n\
         var s = new Set([1, 2, 3]), r = [];\n\
         r.push(caught(function () { return s.union(rec(Symbol(), f)).size; }));\n\
         r.push(caught(function () { return s.union(rec({ valueOf: function () { log.push(\"valueOf\"); throw \"v\"; } }, f)).size; }));\n\
         r.push(caught(function () { var t = new Set([1, 2, 3]); return t.isSubsetOf(rec({ valueOf: function () { log.push(\"valueOf\"); t.clear(); return 1; } }, f)); }));\n\
         r.push(caught(function () { return new Set().isSubsetOf(rec(-0, f)) + \",\" + new Set([1]).isSubsetOf(rec(-0, f)); }));\n\
         r.push(caught(function () { return s.isSupersetOf(rec(Math.pow(2, 53), f)); }));\n\
         r.push(caught(function () { return s.isSupersetOf(rec(\" 3 \", f)); }));\n\
         r.push(caught(function () {\n\
           var p = new Proxy({ size: 1, has: f, keys: function () { log.push(\"keys\"); return { next: function () { return { done: true }; } }; } },\n\
             { get: function (t, k) { log.push(\"get \" + String(k)); return t[k]; } });\n\
           return [...s.union(p)].join(\"\"); }));\n\
         r.push(caught(function () { return Set.prototype.union.call(new Proxy(new Set([1]), {}), rec(1, f)); }));\n\
         r.join(\"|\");",
        "TypeError size|v size,valueOf|none true size,valueOf|none true,false size,size|\
         none false size|none true size,keys|none 123 get size,get has,get keys,keys|TypeError ",
    );
}

/// A Set of another realm is a Set (the brand is the slot), and the
/// result of `union` is made with `%Set.prototype%` of the realm of the method called, not of
/// `this` (`#sec-set.prototype.union` step 8, the current realm).
#[test]
fn the_result_takes_the_set_prototype_of_the_method_s_realm() {
    let (mut agent, a) = agent_and_realm(|limits| limits.steps_per_entry = 1 << 40);
    let b = realm(&mut agent);
    let root =
        |agent: &mut Agent<TestHost>, realm: &Realm, source: &str| match eval(agent, realm, source)
        {
            HostOutcome::Normal(root) => root,
            other => panic!("{source}: {other:?}"),
        };
    let set_b = root(&mut agent, &b, "new Set([1, 2]);");
    let probe = root(
        &mut agent,
        &a,
        "(function (sb) {\n\
           var u = Set.prototype.union.call(sb, new Set([3])), v = sb.union(new Set([3]));\n\
           return [Object.getPrototypeOf(sb) !== Set.prototype, Object.getPrototypeOf(u) === Set.prototype,\n\
             Object.getPrototypeOf(v) === Object.getPrototypeOf(sb), [...u].join(), [...v].join(),\n\
             Set.prototype.isSubsetOf.call(sb, new Set([1, 2, 3]))].join();\n\
         });",
    );
    let undefined = agent
        .with_cx(&a, |cx| cx.root(Value::Undefined).unwrap())
        .unwrap();
    let outcome = agent.call(&a, &probe, &undefined, &[&set_b]).unwrap();
    assert_eq!(
        support::script::describe(&mut agent, &a, outcome),
        Outcome::Normal("\"true,true,true,1,2,3,1,2,3,true\"".to_string())
    );
}

/// Keys held only by the frame of a callback while a collection
/// can run. `intersection`'s walk over `this` (size of `this` not larger) with a `has` that
/// deletes its argument from `this` and answers true: the entry is then held by nothing but
/// the method until it is appended. `union` and `symmetricDifference` with a `next` whose
/// result has a `done` and a `value` getter, the `value` getter making the key, `{ i }`,
/// referenced from nowhere else. Each callback and getter allocates; the reading is made with
/// `collect-every-safe-point`.
#[test]
fn keys_held_only_by_a_callback_survive_a_collection() {
    let outcome = support::script::run(
        {
            let mut limits = limits();
            limits.steps_per_entry = 1 << 40;
            limits
        },
        "function junk() { return new Array(20).fill(0); }\n\
         function back(set) { var out = []; set.forEach(function (o) { out.push(o.i); }); return out.join(); }\n\
         var t = new Set(), k; for (k = 0; k < 10; k++) t.add({ i: k });\n\
         var r = [];\n\
         r.push(back(t.intersection({ size: 100, keys: function () {}, has: function (o) { junk(); t.delete(o); junk(); return true; } })), t.size);\n\
         function made() { return { size: 0, has: function () { return false; }, keys: function () {\n\
           var i = 0;\n\
           return { next: function () { junk(); var n = i++, res = {};\n\
             Object.defineProperty(res, \"done\", { get: function () { junk(); return n >= 10; } });\n\
             Object.defineProperty(res, \"value\", { get: function () { junk(); return { i: n }; } });\n\
             return res; } };\n\
         } }; }\n\
         r.push(back(new Set().union(made())), back(new Set().symmetricDifference(made())));\n\
         r.join(\"|\");",
    );
    let all = "0,1,2,3,4,5,6,7,8,9";
    assert_eq!(outcome, Outcome::Normal(format!("\"{all}|0|{all}|{all}\"")));
}

/// `Limits::min_stack_bytes` through the set-like's `next`, which `union` calls from Rust
/// (IteratorStepValue, `#sec-set.prototype.union` step 7.a): `f` re-enters itself through it
/// until the re-entry limit, on seven eighths of `min_stack_bytes()`, in a child process
/// ([`on_the_promised_stack`]). 13,568 bytes per re-entry when the seven methods ran in one
/// function (13,720,540 at 1,000 against seven eighths, 12,838,560: the test failed),
/// 10,031 since each runs in its own.
#[test]
fn union_reaches_the_reentry_limit_through_next_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set().union({ size: 0, has() {}, keys() { return { next: f }; } })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!("union_reaches_the_reentry_limit_through_next_on_the_promised_stack"),
        body,
    );
}

/// The same through the getter of `size`, which GetSetRecord reads from Rust
/// (`#sec-getsetrecord` step 2), for `union`, whose frames on this way are the largest (with
/// those of `intersection`, `difference` and `symmetricDifference`): 13,503 bytes per re-entry
/// before (13,656,091 at 1,000: the test failed), 10,304 after ([`on_the_promised_stack`]).
#[test]
fn union_reaches_the_reentry_limit_through_the_size_getter_on_the_promised_stack() {
    fn body() {
        reach_through(
            "new Set().union({ get size() { return f(); }, has() {}, keys() {} })",
            BUILTINS_REENTRY,
        );
    }
    on_the_promised_stack(
        test_path!("union_reaches_the_reentry_limit_through_the_size_getter_on_the_promised_stack"),
        body,
    );
}
