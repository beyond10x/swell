---
title: Embedding
description: The host interface of swell for a Rust program that embeds it - the dependency, the entry points, the limits, the hooks and the outcomes, with examples from the tests.
---

import {DependencyLine} from '@site/src/components/Numbers';

# Embedding

swell is a library crate. A Rust program, the host, creates an agent with a host value, limits and seeds, and runs scripts in it. The interface is specified in [`host-interface.md`](https://github.com/beyond10x/swell/blob/main/docs/design/host-interface.md) and will change before 1.0.

## The dependency

The crates are not on a package registry. A program depends on the repository at the release tag:

<DependencyLine />

## Entry points

An `Agent` is one engine instance with one heap. It is created, run and dropped on one thread, and it is not `Send`.

| call | what it does | answers |
|---|---|---|
| `Agent::new(host, limits, seeds)` | an agent with an empty heap and no realm | `Result<Agent<H>, CreateError>` |
| `create_realm` | a realm with its global object | `Outcome<Realm>` |
| `parse_script(realm, source, host_defined)` | ParseScript, runs no code | `Parsed<Script>` |
| `evaluate_script(script)` | ScriptEvaluation, an outermost entry | `Outcome<Root>` |
| `call`, `construct` | call or construct a function | `Outcome<Root>` |
| `with_cx(realm, f)` | create and read values while no code runs | `Result<R, HostError>` |
| `interrupt_handle()` | a handle that is `Send`, to end a run from another thread | `InterruptHandle` |
| `last_entry_steps()`, `heap_bytes()`, `refusals()`, `state()` | reads that run no code | the count or the state |
| `collect()` | a collection now | `Outcome<CollectReport>` |

Modules, promise jobs and the microtask checkpoint are specified and not built yet: jobs come with slice 10 and modules with slice 11.

## Limits

`Limits` has seven fields: `steps_per_entry`, `memory_bytes`, `frame_depth`, `reentry_depth`, `parser_nesting`, `string_length` and `bigint_bits`. A limit has a unit, a meaning at 0 and one outcome when it is reached. The [architecture page](architecture.md#budgets) lists them. Run the agent on a thread with at least `Limits::min_stack_bytes()` bytes of stack.

`Seeds` holds `hash_key`, the key of the engine's hash maps, and `random_seed` for `Math.random`. `Agent::new` is the only function that takes them, and its `Debug` output does not print them.

## What the host supplies

The host is a type that implements the trait `Host`. A hook is an associated function that receives a context, not `&mut self`. `Cx` can run code, and `HookCx` can create values but runs no code. A hook with a default in ECMA-262 has that default as a provided method.

| required hook | what the host answers |
|---|---|
| `host_system_utc_epoch_nanoseconds` | the clock, for the realm's global object |
| `host_enqueue_generic_job` | takes a job to run later |
| `host_enqueue_timeout_job` | takes a job and its delay |
| `host_enqueue_finalization_registry_cleanup_job` | takes a cleanup job |
| `host_load_imported_module` | starts a module load |

The provided hooks cover the time zone, UTC by default, with `system_time_zone_identifier` and the other time zone operations. They also cover `host_ensure_can_compile_strings` before a string is compiled, `host_get_code_for_eval`, the promise job hooks, the rejection tracker, `import.meta` and the array buffer hooks.

## Outcomes

| type | arms |
|---|---|
| `Outcome<T>` | `Normal(T)`, `Throw(Root)`, `Terminated(Termination)` |
| `Parsed<T>` | `Parsed(T)`, `Errors(Vec<Root>)`, `Terminated(Termination)` |
| `Termination` | `StepBudget`, `MemoryBudget`, `Interrupted`, `EngineFault` |
| `HostError` | `Faulted`, `StaleHandle`, `ForeignHandle`, `WrongKind`, `ModuleStatus`, `ModuleTerminated`, `EvaluationInProgress` |

`Err(HostError)` answers a call the host should not have made, and the call leaves the agent as it was. After `Terminated(EngineFault)` the agent is faulted, and its entry points answer `Err(HostError::Faulted)`.

## Examples

The examples are copied from the tests of `crates/swell`, which the gate compiles and runs. The sentence above an example gives its file and item.

A host, from `crates/swell/tests/support/host.rs`, `impl Host for TestHost`. It records each call of a required hook:

```rust
impl Host for TestHost {
    fn host_enqueue_generic_job(cx: &mut HookCx<'_, Self>, job: Job) {
        let host = cx.host_mut();
        host.calls.push("host_enqueue_generic_job".to_string());
        host.jobs.push(job);
    }

    fn host_enqueue_timeout_job(cx: &mut HookCx<'_, Self>, job: Job, milliseconds: f64) {
        let host = cx.host_mut();
        host.calls
            .push(format!("host_enqueue_timeout_job {milliseconds}"));
        host.jobs.push(job);
    }

    fn host_enqueue_finalization_registry_cleanup_job(cx: &mut HookCx<'_, Self>, job: Job) {
        let host = cx.host_mut();
        host.calls
            .push("host_enqueue_finalization_registry_cleanup_job".to_string());
        host.jobs.push(job);
    }

    fn host_load_imported_module(cx: &mut HookCx<'_, Self>, request: LoadRequest) {
        let host = cx.host_mut();
        host.calls.push("host_load_imported_module".to_string());
        host.loads.push(request);
    }

    fn host_system_utc_epoch_nanoseconds(cx: &mut HookCx<'_, Self>, _global: ObjectRef) -> i128 {
        let host = cx.host_mut();
        host.calls
            .push("host_system_utc_epoch_nanoseconds".to_string());
        host.now
    }
}
```

Limits, seeds and an agent, from the same file, the functions `limits`, `seeds` and `agent_with`:

```rust
pub fn limits() -> Limits {
    Limits {
        steps_per_entry: 1_000_000,
        memory_bytes: 64 * 1024 * 1024,
        frame_depth: 10_000,
        reentry_depth: 256,
        parser_nesting: 1_000,
        string_length: 1 << 30,
        bigint_bits: 1 << 20,
    }
}

pub fn seeds() -> Seeds {
    Seeds {
        hash_key: [7; 16],
        random_seed: [11; 32],
    }
}

pub fn agent_with(limits: Limits) -> Result<Agent<TestHost>, CreateError> {
    Agent::new(TestHost::default(), limits, seeds())
}
```

Parse and evaluate a script, from `crates/swell/tests/support/script.rs`, the function `eval_text`:

```rust
pub fn eval_text(agent: &mut Agent<TestHost>, realm: &Realm, source: &str) -> Outcome {
    match agent
        .parse_script(realm, Source::Utf8(source), None)
        .unwrap()
    {
        Parsed::Parsed(script) => {
            let outcome = agent.evaluate_script(&script).unwrap();
            describe(agent, realm, outcome)
        }
        Parsed::Errors(errors) => Outcome::Errors(messages(agent, realm, &errors)),
        Parsed::Terminated(termination) => Outcome::Terminated(termination),
    }
}
```

An endless loop ends at the step budget, from `crates/swell/tests/budgets.rs`, the test `host_6_endless_loop_ends_in_step_budget`:

```rust
fn host_6_endless_loop_ends_in_step_budget() {
    for steps in [1_u64, 10, 1_000] {
        let mut agent = agent_with(with_steps(steps)).unwrap();
        let realm = realm(&mut agent);
        assert_eq!(
            eval_text(&mut agent, &realm, "while (true) {}"),
            Outcome::Terminated(Termination::StepBudget)
        );
        let spent = agent.last_entry_steps().unwrap();
        assert!(spent <= steps, "{spent} > {steps}");
        assert_eq!(agent.state(), AgentState::Alive);
    }
}
```

Another thread ends a run, from `crates/swell/tests/interrupt.rs`, the test `host_90_interrupt_from_another_thread_ends_the_loop`:

```rust
fn host_90_interrupt_from_another_thread_ends_the_loop() {
    let mut limits = support::host::limits();
    limits.steps_per_entry = 1 << 62;
    let mut agent = support::host::agent_with(limits).unwrap();
    let realm = support::host::realm(&mut agent);
    let script = parse(&mut agent, &realm, "while (true) {}");
    let handle = agent.interrupt_handle();
    let interrupter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        handle.interrupt();
    });
    let outcome = agent.evaluate_script(&script).unwrap();
    interrupter.join().unwrap();
    assert!(matches!(
        outcome,
        Outcome::Terminated(Termination::Interrupted)
    ));
    assert!(agent.last_entry_steps().unwrap() > 0);
    assert_eq!(agent.state(), AgentState::Alive);
}
```

The tests call `unwrap` on a `Result<_, HostError>`. A host handles the `Err` instead.

## The shell binary

`crates/cli` (package `swell-cli`) is a small host that builds the binary `swell`. It runs one script from the command line, for benchmarks and for comparing swell with other engines. It uses the standard library only and has `#![forbid(unsafe_code)]`, like the engine.

Each [release](https://github.com/beyond10x/swell/releases) from the one that adds the shell on has the binary as an asset for five targets: `swell-<version>-<target>.tar.gz` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, and `swell-<version>-x86_64-pc-windows-msvc.zip`. Each archive holds the binary, `README.md`, `LICENSE` and `NOTICE`. The release's `SHA256SUMS` lists the SHA-256 of each archive:

```sh
sha256sum --check --ignore-missing SHA256SUMS
tar -xzf swell-<version>-x86_64-unknown-linux-gnu.tar.gz
swell-<version>-x86_64-unknown-linux-gnu/swell --version
```

The release notes say whether a release's macOS and Windows binaries are signed. A signed macOS `swell` is signed with a Developer ID Application certificate and the hardened runtime and notarized by Apple; a bare binary carries no stapled ticket, so macOS checks the notarization online the first time it runs. To check the signature:

```sh
codesign --verify --strict --verbose=2 swell-<version>-aarch64-apple-darwin/swell
codesign -dv --verbose=2 swell-<version>-aarch64-apple-darwin/swell
```

For a signed binary `codesign -dv` prints `Authority=Developer ID Application: …`, `flags=0x10000(runtime)` and a `Timestamp=` line. `spctl --assess` is for app bundles and installer packages and rejects a bare command-line binary, so it is not the check here.

A signed Windows `swell.exe` is signed through Azure Artifact Signing with a timestamp. In PowerShell:

```powershell
Get-AuthenticodeSignature .\swell-<version>-x86_64-pc-windows-msvc\swell.exe | Format-List Status, SignerCertificate, TimeStamperCertificate
```

`Status` is `Valid`. An unsigned binary (`NotSigned` on Windows; on macOS no `Authority=` line, at most an ad-hoc signature) is one of a release whose notes say so, or of a release made before signing was added; the system may warn about it on first run.

To build it from source:

```sh
cargo build --release -p swell-cli
target/release/swell [flags] script.js
target/release/swell [flags] -e 'print(6 * 7)'
```

The binary creates one agent and one realm, runs the script, then runs the promise jobs the script queued. It runs the agent on a thread with `Limits::min_stack_bytes()` bytes of stack. The realm has two globals that ECMA-262 does not define:

- `print(...args)` converts each argument to a string, joins them with a space and writes them to standard output, followed by a newline.
- `console.log` is the same function.

`Date.now()` reads the system clock through `host_system_utc_epoch_nanoseconds`. The seeds are fixed, so `Math.random` gives the same sequence in every run.

| flag | sets |
|---|---|
| `--steps <n>` | `steps_per_entry`, shared by the script and its promise jobs |
| `--memory <bytes>` | `memory_bytes` |
| `--depth <n>` | `frame_depth` |
| `--reentry <n>` | `reentry_depth` |
| `--nesting <n>` | `parser_nesting` |
| `--string-length <n>` | `string_length` |
| `--bigint-bits <n>` | `bigint_bits` |
| `--timeout <ms>` | interrupts the run from another thread after that much wall time |
| `-e <source>` | runs the source instead of a file |
| `-h`, `--help` | prints the flags, their defaults and the exit statuses |
| `--version` | prints `swell <version>` |

A number is decimal, or `max` for the largest value of its type. A limit that is not given takes the value the test262 runner uses (`DEFAULT_LIMITS` in `crates/cli/src/args.rs`). `swell --help` prints these values.

| exit status | outcome |
|---|---|
| 0 | the script and its jobs ended normally |
| 1 | an uncaught throw; its kind, message and position go to standard error |
| 2 | a syntax error |
| 3 | `Terminated(StepBudget)` |
| 4 | `Terminated(MemoryBudget)` |
| 5 | a depth limit: the `RangeError` of `frame_depth` or `reentry_depth`, or the parse error of `parser_nesting` |
| 6 | `Terminated(Interrupted)`, after `--timeout` |
| 64 | a usage error, or a file that cannot be read or is not UTF-8 |
| 70 | `Terminated(EngineFault)`, or an entry point that answered `Err(HostError)` |
