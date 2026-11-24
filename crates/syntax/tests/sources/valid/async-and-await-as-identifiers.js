// sec-identifiers-static-semantics-early-errors: parses
// sloppy-only
var async = 1, await = 2;
async = await + async;
async: for (;;) break async;
await: for (;;) break await;
function f(async, await) {
  return async(await);
}
function duplicate(async, async) {}
var g = async => async;
var h = await => await;
var o = { async, await, async: 1 };
async(1, 2);
async
function notAsync() {}
var k = async
(1);
async
x => x;
function* gen() {
  var async = await;
}
x = async (1) + async;
x = async in o;
x = async ? await : async;
new async();
(async);
x = `${async}${await}`;
for (async.x of [1]);
for (async in o);
