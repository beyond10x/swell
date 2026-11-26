#!/bin/sh
# Smoke probe on the node: image, tool, then one hello script per config and flag variants.
# Run: slot --timeout 20m sh bench/smoke.sh <origin digest>  (from the repository root)
set -u
mkdir -p .out/bench-probe
if [ "$(sha256sum /etc/machine-id | cut -c1-16)" = "${1:-}" ]; then echo "smoke: on the machine that started it; refusing"; exit 1; fi
echo "host: $(nproc) cores, $(sed -n "s/^model name[[:space:]]*: //p" /proc/cpuinfo | head -n 1), $(uname -sr)"
sh bench/run.sh prepare > .out/bench-probe/prepare.log 2>&1
echo "prepare exit $?"
cat .out/bench-probe/prepare.log | grep -v '^\s*$' | tail -n 60
image=$(sed -n 's/^image //p' .out/bench-probe/prepare.log)
tool=$(sed -n 's/^tool //p' .out/bench-probe/prepare.log)
echo "image=$image tool=$tool"
[ -x "$tool" ] || exit 1
echo "== cgroup"
cat /proc/self/cgroup
ls -ld /sys/fs/cgroup/system.slice/slot-node.service 2>&1
cat /sys/fs/cgroup/system.slice/slot-node.service/cgroup.subtree_control 2>&1
mkdir /sys/fs/cgroup/system.slice/slot-node.service/bench-probe 2>&1 && echo "cgroup mkdir ok" && rmdir /sys/fs/cgroup/system.slice/slot-node.service/bench-probe
d=.out/bench-probe/smoke
mkdir -p "$d"
cat > "$d/hello.js" <<'EOF'
var o = typeof print === 'function' ? print : function (s) { console.log(s); };
o('hello ' + (1 + 1));
o('performance.now ' + (typeof performance === 'object' && performance !== null && typeof performance.now));
o('preciseTime ' + typeof preciseTime + ' dateNow ' + typeof dateNow + ' require ' + typeof require + ' console ' + typeof console);
try { eval('class A { m() { return 3; } }; o("class ok " + new A().m());'); } catch (e) { o('class failed ' + e); }
try { eval('const { x } = { x: 4 }; let y = () => x; o("destructure ok " + y());'); } catch (e) { o('destructure failed ' + e); }
EOF
: > "$d/empty.js"
run() {
  name=$1; shift
  envs=$1; shift
  line=$(env $envs unshare -rn taskset -c 2,3 prlimit --data=4294967296 "$tool" --stdout "$d/$name.out" --stderr "$d/$name.err" --timeout 60 -- "$@" 2>&1)
  echo "-- $name: $line"
  head -c 600 "$d/$name.out"; head -c 400 "$d/$name.err"
}
e=$image/engines
run v8-jitless "" "$e/v8/d8" --jitless "$d/hello.js"
run v8 "" "$e/v8/d8" "$d/hello.js"
run sm-interp "LD_LIBRARY_PATH=$e/spidermonkey" "$e/spidermonkey/js" --no-ion --no-baseline --no-blinterp --no-native-regexp "$d/hello.js"
run sm "LD_LIBRARY_PATH=$e/spidermonkey" "$e/spidermonkey/js" "$d/hello.js"
run sm-nolib "" "$e/spidermonkey/js" "$d/hello.js"
run jsc-nojit "JSC_useJIT=false JSC_useRegExpJIT=false" "$e/javascriptcore/jsc" "$d/hello.js"
run jsc "" "$e/javascriptcore/jsc" "$d/hello.js"
run jsc-opt "" "$e/javascriptcore/jsc" --useJIT=false "$d/hello.js"
run hermes "" "$e/hermes/hermes" "$d/hello.js"
run hermes-class "" "$e/hermes/hermes" -Xes6-class "$d/hello.js"
run quickjs "" "$e/quickjs/qjs" "$d/hello.js"
run quickjs-ng "" "$e/quickjs-ng/qjs" "$d/hello.js"
run boa "" "$e/boa/boa" "$d/hello.js"
run xs "" "$e/xs/xst" "$d/hello.js"
run xs-s "" "$e/xs/xst" -s "$d/hello.js"
run xs-empty "" "$e/xs/xst" "$d/empty.js"
run hermes-empty "" "$e/hermes/hermes" "$d/empty.js"
run boa-empty "" "$e/boa/boa" "$d/empty.js"
echo "== sm help jit"
LD_LIBRARY_PATH=$e/spidermonkey "$e/spidermonkey/js" --help 2>&1 | grep -i -E 'no-ion|no-baseline|no-blinterp|no-native-regexp|no-jit|jitless' | head
echo "== jsc options"
"$e/javascriptcore/jsc" --options 2>&1 | grep -E '^\s*(useJIT|useRegExpJIT|useBaselineJIT|useDFGJIT|useLLInt)\b' | head
JSC_useJIT=false "$e/javascriptcore/jsc" --options 2>&1 | grep -E '^\s*(useJIT|useRegExpJIT)\b' | head
echo "== xst help"
"$e/xs/xst" -h 2>&1 | head -20
echo "== hermes help"
"$e/hermes/hermes" --help 2>&1 | grep -i -E 'es6|class|jit|lazy' | head
echo "== v8 build config"
cat "$e/v8/v8_build_config.json"
echo "== file"
file "$e"/*/* 2>&1 | grep -v 'directory' | cut -c1-200
echo "done"
