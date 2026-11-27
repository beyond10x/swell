// The loop of the property-escape cases of test262: the body of `buildString` of the test262
// harness (`harness/regExpUtils.js`, BSD license, Copyright (C) 2017 Mathias Bynens), run over
// ranges of a fixed size with no regexp match. Read by `taskfiles/perf-loop.sh`
// (story:property-escape-cases-spend-less-in-the-interpreter-loop); needs no harness file.

function buildString(args) {
  // Use member expressions rather than destructuring `args` for improved
  // compatibility with engines that only implement assignment patterns
  // partially or not at all.
  const loneCodePoints = args.loneCodePoints;
  const ranges = args.ranges;
  const CHUNK_SIZE = 10000;
  let result = String.fromCodePoint.apply(null, loneCodePoints);
  for (let i = 0; i < ranges.length; i++) {
    let range = ranges[i];
    let start = range[0];
    let end = range[1];
    let codePoints = [];
    for (let length = 0, codePoint = start; codePoint <= end; codePoint++) {
      codePoints[length++] = codePoint;
      if (length === CHUNK_SIZE) {
        result += String.fromCodePoint.apply(null, codePoints);
        codePoints.length = length = 0;
      }
    }
    result += String.fromCodePoint.apply(null, codePoints);
  }
  return result;
}

// Each round builds a string of 132,097 code points, so that the loop and not the growth of
// `result` takes the time and the run stays within the step budget of the shell's defaults.
const ROUNDS = 80;
let units = 0;
let result = "";
for (let round = 0; round < ROUNDS; round++) {
  result = buildString({
    loneCodePoints: [0x00002A],
    ranges: [
      [0x00DC00, 0x00DFFF],
      [0x010000, 0x02FFFF]
    ]
  });
  units += result.length;
}
if (units !== ROUNDS * result.length) {
  throw new Error("the rounds built strings of different lengths");
}
print(result.length);
