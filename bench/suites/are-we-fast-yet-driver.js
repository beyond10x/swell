// Runs one are-we-fast-yet benchmark as the suite's harness.js does: BENCH_ITERATIONS timed
// calls of innerBenchmarkLoop(BENCH_INNER), each checked. Prints "iteration 0 <i> <ms>" per
// call, then "clock <name>" and "done <benchmark>". BENCH_NAME, BENCH_ITERATIONS and
// BENCH_INNER are set by the line bench/run.sh puts first; the modules are bundled before this.
(function () {
  var benchmark = require(BENCH_NAME).newInstance();
  for (var i = 0; i < BENCH_ITERATIONS; i++) {
    var start = benchClock();
    var ok = benchmark.innerBenchmarkLoop(BENCH_INNER);
    var end = benchClock();
    if (!ok) {
      throw new Error('Benchmark failed with incorrect result');
    }
    benchOut('iteration 0 ' + i + ' ' + (end - start));
  }
  benchOut('clock ' + benchClockName);
  benchOut('done ' + BENCH_NAME);
})();
