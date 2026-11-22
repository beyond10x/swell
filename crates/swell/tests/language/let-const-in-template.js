// sec-template-literals-runtime-semantics-evaluation: normal "a1-b2|0,1,2|x3"
let n = 1;
const s = `a${n}-${(function () { let n = 2; return `b${n}`; })()}`;
var parts = [];
for (let i = `ab`.length - 2; i < 3; i++) {
  parts.push(`${i}`);
}
let joined;
{
  const k = 3;
  let t = `x${k}`;
  joined = `${s}|${parts.join(",")}|${t}`;
}
joined;
