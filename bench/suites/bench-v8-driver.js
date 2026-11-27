// Runs the one suite of bench-v8 version 7 that base.js and one benchmark file registered:
// for each of its Benchmark objects, Setup, BENCH_ITERATIONS timed calls of run, TearDown.
// Prints "iteration <object> <i> <ms>" per call, then "clock <name>" and "done <suite>".
// BENCH_ITERATIONS is set by the line bench/run.sh puts first.
(function () {
  var suite = BenchmarkSuite.suites[0];
  for (var j = 0; j < suite.benchmarks.length; j++) {
    var benchmark = suite.benchmarks[j];
    benchmark.Setup();
    for (var i = 0; i < BENCH_ITERATIONS; i++) {
      var start = benchClock();
      benchmark.run();
      var end = benchClock();
      benchOut('iteration ' + j + ' ' + i + ' ' + (end - start));
    }
    benchmark.TearDown();
  }
  benchOut('clock ' + benchClockName);
  benchOut('done ' + suite.name);
})();
