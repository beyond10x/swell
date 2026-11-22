// sec-json.rawjson: normal "true,false,false,true|{"big":12345678901234567890,"s":"x"}|SyntaxError|TypeError"
var raw = JSON.rawJSON("12345678901234567890");
var d = Object.getOwnPropertyDescriptor(raw, "rawJSON");
var bad, assign;
try { JSON.rawJSON("[1]"); bad = "made"; } catch (e) { bad = e.name; }
(function () {
  "use strict";
  try { raw.x = 1; assign = "assigned"; } catch (e) { assign = e.name; }
}());
[Object.getPrototypeOf(raw) === null, d.writable, d.configurable, JSON.isRawJSON(raw)].join() +
  "|" + JSON.stringify({ big: raw, s: JSON.rawJSON('"x"') }) + "|" + bad + "|" + assign;
