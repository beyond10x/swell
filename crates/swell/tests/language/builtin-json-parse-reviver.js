// sec-internalizejsonproperty: normal "0=1.0 1=-0 2="a" k=true gone=null 3=- =-|[10,0,"a",{"k":true}]"
var seen = [];
var result = JSON.parse('[1.0, -0, "a", {"k": true, "gone": null}]', function (key, value, context) {
  seen.push(key + "=" + (context.hasOwnProperty("source") ? context.source : "-"));
  if (key === "gone") return undefined;
  if (key === "0") return 10;
  return value;
});
seen.join(" ") + "|" + JSON.stringify(result);
