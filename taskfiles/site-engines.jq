# The program of taskfiles/site-engines.sh: engines.json (format swell-engines/1) from the
# catalog, the published rows, the result files and swell's own values. Every value carries a
# source with its label, URL, date, revision and method; a missing value carries its reason.

def repo: "https://github.com/beyond10x/swell/blob/main/";
def gh($path; $line): repo + $path + (if $line == null then "" else "#L\($line)" end);
def ghr($path; $first; $last): repo + $path + "#L\($first)" + (if $last == $first then "" else "-L\($last)" end);
# Passed of a total in percent, floored to one decimal from integer tenths, as status.json does.
def pct($p; $t): if $t == 0 then "0.0" else ((1000 * $p / $t) | floor) as $x | "\(($x / 10) | floor).\($x % 10)" end;
def short: .[0:7];
def notm($reason): {status: "not-measured", reason: $reason};
def capability_rows: ["memory_limit", "step_budget", "interrupt_terminate", "stack_limit"];
def capability_labels: {
  memory_limit: "Memory cap",
  step_budget: "Step or instruction budget",
  interrupt_terminate: "Interrupt or terminate",
  stack_limit: "Stack or recursion limit"
};
def fact_rows: [
  ["language", "language", "Implementation language"],
  ["license", "license", "Licence"],
  ["execution_model", "execution_model", "Execution model"],
  ["interpreter_only", "interpreter_only_mode", "Interpreter-only mode"],
  ["latest_release", "latest_release", "Latest release"],
  ["releases_12mo", "release_cadence_12mo", "Releases in the last 12 months"]
];

$catalog_lines as $catalog
| $published[0] as $pub
| $results[0] as $res
| $status[0] as $st
| $expected[0] as $exp
| $swell[0] as $sw
| ($sw.pin_commit[0:8]) as $pin8
| ($exp | map(select(.kind == "total"))[0]) as $total
| ($exp | map(select(.kind == "dir"))) as $swell_dirs
| ([$swell_dirs[].dir] + [$pub[] | select(.directory != "all") | .directory] | unique) as $dirs

# Configurations: swell first, then every configuration of the catalog in its order.
| ([{id: "swell", engine: "swell", name: "swell", label: "swell", mode: "interpreter", flags: [],
     flags_verified: true, default_set: false, catalog_url: "https://github.com/beyond10x/swell"}]
   + [$catalog[] as $e | $e.configurations[]
      | {id, engine: $e.id, name: $e.name, label, mode, flags, flags_verified,
         default_set: $e.default_set, catalog_url: $e.repo_url.sources[0]}]) as $configs
| if ([$configs[].id] | unique | length) != ($configs | length) then error("two configurations share an id") else . end

# A cell from the catalog: the field's text, its sources and how the catalog knows it.
| def catalog_cell($e; $field):
    ($e.fields[$field] // {value: "unknown", sources: [], basis: "unknown"}) as $f
    | {status: (if $f.basis == "unknown" then "unknown" else "documented" end), text: $f.value}
      + (if $f.unsourced_note then {note: "the catalog's text has no source: \($f.unsourced_note)"} else {} end)
      + {source: {label: "engine catalog, \($f.basis)", url: ($f.sources[0] // null),
                  date: $e.catalog_date, revision: $catalog_path, method: $f.basis}
                  + (if ($f.sources | length) > 1 then {urls: $f.sources} else {} end)};

# K1 from a published row.
  def zoo_cell($r):
    ($r.file | sub("^.*/"; "") | sub("\\.tsv$"; "")) as $basis
    | ($r.passed | tonumber) as $p | ($r.total | tonumber) as $t
    | {status: "published", value: $p, of: $t, percent: pct($p; $t), unit: "cases", basis: $basis,
       source: {label: "javascript-zoo-data, zoo runner, test262 \($r.test262_revision | short)",
                url: $r.url, date: $r.run_date, fetched: $r.fetch_date,
                revision: "test262 \($r.test262_revision | short), engine \($r.engine_revision[0:12]), data \($r.data_commit | short)",
                sha256: $r.sha256, file: "\($r.file):\($r.line)",
                method: ("the zoo runner's outcome per file; a file counts as passed when it reads PASS"
                         + (if $r.engine_flags == "" then "" else "; flags \($r.engine_flags)" end))}};

# K1 from swell's own runner: over every case that exists, with the run-case share beside it.
  def swell_k1($passed; $exist; $run; $url; $what):
    {status: "measured", value: $passed, of: $exist, percent: pct($passed; $exist), unit: "cases",
     basis: "swell-runner-\($pin8)",
     secondary: {value: $passed, of: $run, percent: pct($passed; $run), label: "of the cases swell's runner runs"},
     source: {label: "swell's runner, test262 \($pin8)", url: $url, date: null,
              revision: "test262 \($pin8), swell \($st.version)",
              method: "\($what); excluded and not applicable cases count as failed; a case passes when every mode that applies to it passes"}};

# A result row of our runs, for the configuration it names.
  def config_of($row):
    ([$configs[] | select(.id == $row.engine and .mode == $row.mode)]
     + [$configs[] | select(.engine == $row.engine and .mode == $row.mode)]
     + [$configs[] | select(.id == $row.engine)])
    | (.[0].id // null);
  # The rows a derived row (a geomean: `of` names its benchmarks) is made of: the rows of the same
  # file, configuration, mode and KPI whose measurement is "<suite>/<a name of of>".
  def parts_of($file; $row):
    if ($row.of | type) == "array" then
      ($row.suite // ($row.measurement | sub("/[^/]*$"; ""))) as $suite
      | [$row.of[] | "\($suite)/\(.)"] as $names
      | [$file.rows[] | select(.engine == $row.engine and (.config // .engine) == ($row.config // $row.engine)
                               and .mode == $row.mode and .kpi == $row.kpi
                               and (.measurement as $m | any($names[]; . == $m)))]
    else [] end;
  # Whether a figure was measured while another job ran on the build node: the row's loaded and,
  # when loaded, its loaded_reason. A derived figure is loaded when it or any row it is made of
  # is, with the reasons of those rows (one as a string, more as a list, as the result file
  # writes them). A row without loaded gives no field: the file does not say.
  def loaded_of($file; $row):
    [$row, parts_of($file; $row)[] | select(.loaded == true)] as $hot
    | if ($hot | length) > 0 then
        ([$hot[] | .loaded_reason | if type == "array" then .[] else . end | select(. != null)] | unique) as $why
        | {loaded: true} + (if ($why | length) == 0 then {} elif ($why | length) == 1 then {loaded_reason: $why[0]} else {loaded_reason: $why} end)
      elif $row.loaded == false then {loaded: false}
      else {} end;
  def result_cell($file; $row):
    if $row.status == "measured" then
      {status: "measured", value: $row.median, unit: $row.unit, runs: $row.runs,
       min: ($row.values | min), max: ($row.values | max), basis: "swell-bench-\($file.host.runner // "run")"}
      + loaded_of($file; $row)
      + {source: {label: "our run, \($file.host.runner // "build node"), \($file.host.cpus // "?") CPUs",
                url: gh($file.file; null), date: $file.date,
                revision: "swell \($file.commit | tostring | short), \($row.engine) \($row.version)",
                method: "\($row.measurement): median of \($row.runs) runs, min and max beside it"
                        + (if ($row.flags | length) > 0 then "; flags \($row.flags | join(" "))" else "" end)}}
    else notm($row.reason // $row.status) end;

  [$res[] as $file | $file.rows[] | select(.kpi == "K2" or .kpi == "K3" or .kpi == "K4" or .kpi == "K5")
   | {file: $file, row: ., config: config_of(.)}] as $result_rows
| if ([$result_rows[] | select(.config == null)] | length) > 0 then
    error("\([$result_rows[] | select(.config == null)][0] | "\(.file.file): engine \(.row.engine), mode \(.row.mode)") names no configuration of \($catalog_path)")
  else . end
| ([$result_rows[] | {kpi: .row.kpi, measurement: .row.measurement, unit: .row.unit}] | unique_by([.kpi, .measurement])) as $measures

# Rows, grouped by KPI, in the page's order.
| ([{id: "K1.total", kpi: "K1", group: "Conformance", label: "test262, every case", unit: "cases", direction: "higher", kind: "count"}]
   + [$dirs[] | {id: "K1.\(.)", kpi: "K1", group: "Conformance", label: "test262 \(.)/", unit: "cases", direction: "higher", kind: "count"}]
   + (if ($measures | length) == 0 then
        [["K2", "Speed", "Interpreter throughput"], ["K3", "Start", "Start"], ["K4", "Memory", "Peak memory"], ["K5", "Size", "Binary size"]]
        | map({id: .[0], kpi: .[0], group: .[1], label: .[2], unit: null, direction: "lower", kind: "measure"})
      else
        [$measures[] | {id: "\(.kpi).\(.measurement)", kpi, group: ({K2: "Speed", K3: "Start", K4: "Memory", K5: "Size"}[.kpi]),
                        label: .measurement, unit, direction: "lower", kind: "measure"}]
      end)
   + [capability_rows[] | {id: "K6.\(.)", kpi: "K6", group: "Resource control", label: capability_labels[.], unit: null, direction: null, kind: "text"}]
   + [{id: "K7.memory_safe", kpi: "K7", group: "Safety", label: "Memory-safe implementation", unit: null, direction: null, kind: "text"},
      {id: "K7.forbid_unsafe", kpi: "K7", group: "Safety", label: "Crate roots with #![forbid(unsafe_code)]", unit: "crate roots", direction: "higher", kind: "count"},
      {id: "K7.third_party", kpi: "K7", group: "Safety", label: "Packages from outside the repository", unit: "packages", direction: "lower", kind: "count"},
      {id: "K8.deterministic", kpi: "K8", group: "Determinism", label: "Same calls, same outcomes and step counts", unit: null, direction: null, kind: "text"}]
   + [fact_rows[] | {id: "F.\(.[0])", kpi: "facts", group: "Facts", label: .[2], unit: null, direction: null, kind: "text"}]
  ) as $rows

# swell's cells.
| ({"K1.total": swell_k1($total.passed; $total.exist; $total.run; gh($sw.expected; $total.line); "the total line of expected.txt")}
   + ([$swell_dirs[] | {key: "K1.\(.dir)", value: swell_k1(.passed; .exist; .run; ghr($sw.expected; .first; .last); "the sum of the \(.dir)/ lines of expected.txt")}] | from_entries)
   + {"K6.memory_limit": {status: "documented", text: "yes: Limits::memory_bytes, bytes charged over the agent's life; ends in Terminated(MemoryBudget)",
                          source: {label: "swell source", url: gh($sw.limits; $sw.memory_line), date: null, revision: "swell \($st.version)", method: "the field of Limits"}},
      "K6.step_budget": {status: "documented", text: "yes: Limits::steps_per_entry, steps of one outermost entry; ends in Terminated(StepBudget)",
                         source: {label: "swell source", url: gh($sw.limits; $sw.steps_line), date: null, revision: "swell \($st.version)", method: "the field of Limits"}},
      "K6.interrupt_terminate": {status: "documented", text: "yes: InterruptHandle::interrupt; the run ends in Terminated(Interrupted)",
                                 source: {label: "swell source", url: gh($sw.interrupt; $sw.interrupt_line), date: null, revision: "swell \($st.version)", method: "the host's interrupt handle"}},
      "K6.stack_limit": {status: "documented", text: "yes: Limits::frame_depth and reentry_depth end in a RangeError; Limits::min_stack_bytes sizes the thread",
                         source: {label: "swell source", url: gh($sw.limits; $sw.frames_line), date: null, revision: "swell \($st.version)", method: "the fields of Limits"}},
      "K7.memory_safe": {status: "measured", text: "yes: Rust, #![forbid(unsafe_code)] in \($st.crates.roots_forbidding_unsafe) of \($st.crates.roots) crate roots",
                         source: {label: "forbid scan of site-data", url: "https://github.com/beyond10x/swell/tree/main/crates", date: null, revision: "swell \($st.version)", method: "the crate roots forbid-unsafe-check reads"}},
      "K7.forbid_unsafe": {status: "measured", value: $st.crates.roots_forbidding_unsafe, of: $st.crates.roots, unit: "crate roots", basis: "swell-only",
                           source: {label: "forbid scan of site-data", url: "https://github.com/beyond10x/swell/tree/main/crates", date: null, revision: "swell \($st.version)", method: "crate roots holding the line #![forbid(unsafe_code)]"}},
      "K7.third_party": {status: "measured", value: $st.crates.lock_packages_outside, of: $st.crates.lock_packages, unit: "packages", basis: "swell-only",
                         source: {label: "Cargo.lock", url: gh("Cargo.lock"; null), date: null, revision: "swell \($st.version)", method: "packages of Cargo.lock with a source line"}},
      "K8.deterministic": {status: "measured", text: "yes: the same calls give the same hooks, outcomes and step counts, held by \($sw.determinism_test)",
                           source: {label: "host requirement HOST-133", url: gh($sw.requirements; $sw.determinism_line), date: null, revision: "swell \($st.version)", method: "a test replays a corpus and compares"}},
      "F.language": {status: "documented", text: "Rust, edition \($sw.edition)",
                     source: {label: "Cargo.toml", url: gh($sw.manifest; $sw.edition_line), date: null, revision: "swell \($st.version)", method: "the workspace manifest"}},
      "F.license": {status: "documented", text: $sw.license,
                    source: {label: "Cargo.toml", url: gh($sw.manifest; $sw.license_line), date: null, revision: "swell \($st.version)", method: "the workspace manifest"}},
      "F.execution_model": {status: "documented", text: "bytecode compiler and interpreter; no JIT, no machine code generation",
                            source: {label: "design", url: "https://github.com/beyond10x/swell/blob/main/docs/design/engine.md", date: null, revision: "swell \($st.version)", method: "the design and the rule of AGENTS.md"}},
      "F.interpreter_only": {status: "documented", text: "always: there is no JIT",
                             source: {label: "AGENTS.md", url: "https://github.com/beyond10x/swell/blob/main/AGENTS.md", date: null, revision: "swell \($st.version)", method: "the repository's rule: no just-in-time compiler"}},
      "F.latest_release": {status: "documented", text: $st.version,
                           source: {label: "Cargo.toml", url: gh($sw.manifest; $sw.version_line), date: null, revision: "swell \($st.version)", method: "the workspace version"}},
      "F.releases_12mo": notm("site-data reads no release history")}
  ) as $swell_cells

# Every configuration's cells.
| [$configs[] as $c
   | ($catalog | map(select(.id == $c.engine)) | .[0]) as $e
   | ([$pub[] | select(.config == $c.id)]) as $zoo
   | (if $c.id == "swell" then $swell_cells
      else
        (if ($zoo | length) == 0 then
           {"K1.total": notm("not in javascript-zoo-data and not run by us yet")}
           + ([$dirs[] | {key: "K1.\(.)", value: notm("not in javascript-zoo-data and not run by us yet")}] | from_entries)
         else
           ([$zoo[] | {key: (if .directory == "all" then "K1.total" else "K1.\(.directory)" end), value: zoo_cell(.)}] | from_entries)
         end)
        + ([capability_rows[] as $f | {key: "K6.\($f)", value: catalog_cell($e; $f)}] | from_entries)
        + {"K7.memory_safe": catalog_cell($e; "memory_safe"),
           "K7.forbid_unsafe": {status: "not-applicable", reason: "a scan of a Rust workspace's crate roots; swell only"},
           "K7.third_party": {status: "not-applicable", reason: "a count of Cargo.lock packages; swell only"},
           "K8.deterministic": {status: "unknown", reason: "the engine catalog records no determinism field"}}
        + ([fact_rows[] as $f | {key: "F.\($f[0])", value: catalog_cell($e; $f[1])}] | from_entries)
      end) as $own
   | (if ($measures | length) == 0 then
        {"K2": notm("no run yet"), "K3": notm("no run yet"), "K4": notm("no run yet"), "K5": notm("no run yet")}
      else
        ([$measures[] as $m
          | ([$result_rows[] | select(.config == $c.id and .row.kpi == $m.kpi and .row.measurement == $m.measurement)] | last) as $hit
          | {key: "\($m.kpi).\($m.measurement)",
             value: (if $hit == null then notm("not in a run yet") else result_cell($hit.file; $hit.row) end)}] | from_entries)
      end) as $measured
   | ([$dirs[] | "K1.\(.)"] | map({key: ., value: notm("no count for this directory")}) | from_entries) as $dir_gaps
   | $c + {values: ($dir_gaps + $own + $measured)}
  ] as $engines

| {
    format: "swell-engines/1",
    generated_by: "task site-data, taskfiles/site.yml and taskfiles/site-engines.sh",
    inputs: ($inputs | split("\n") | map(select(length > 0) | split(" ") | {path: .[0], sha256: .[1]})),
    published: ([$pub[] | {file: (.file | sub("^.*/"; "")), path: .file, data_commit, test262_revision}] | unique_by(.file)
                | map(. + {label: "javascript-zoo-data at \(.data_commit | short), test262 \(.test262_revision | short)",
                           url: "https://github.com/ivankra/javascript-zoo-data/tree/\(.data_commit)",
                           license: "MIT", license_file: "bench/published/LICENSE-javascript-zoo-data"})),
    swell_test262: {revision: $sw.pin_commit, source: gh($sw.pin; $sw.pin_line)},
    default_selection: ["swell", "v8-jitless", "quickjs", "boa"],
    default_set: [$catalog[] | select(.default_set) | .id],
    rows: $rows,
    engines: $engines
  }
