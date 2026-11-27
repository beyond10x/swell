// The harness's prelude, put before every benchmark script by bench/run.sh: the output function
// and the best clock the shell has, each named in the output.
var benchOut = typeof print === 'function' ? print : function (line) { console.log(line); };
var benchClock;
var benchClockName;
if (typeof performance === 'object' && performance !== null && typeof performance.now === 'function') {
  benchClock = function () { return performance.now(); };
  benchClockName = 'performance.now';
} else if (typeof preciseTime === 'function') {
  benchClock = function () { return preciseTime() * 1000; };
  benchClockName = 'preciseTime';
} else if (typeof dateNow === 'function') {
  benchClock = function () { return dateNow(); };
  benchClockName = 'dateNow';
} else {
  benchClock = function () { return Date.now(); };
  benchClockName = 'Date.now';
}
