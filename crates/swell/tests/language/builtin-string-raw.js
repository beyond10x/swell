// sec-string.raw: normal "a1b2c|xy|"
// String.raw interleaves the raw literals with the substitutions and stops after the literal
// at the raw length; a template literal is slice 6, so the template object is written out.
[String.raw({ raw: ["a", "b", "c"] }, 1, 2, 3), String.raw({ raw: "xy" }), String.raw({ raw: [] }, 1)].join("|");
