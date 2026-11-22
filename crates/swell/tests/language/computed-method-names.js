// sec-setfunctionname: normal true
// A method, an accessor and an anonymous function under a computed key get their names when the
// literal is evaluated.
var prefix = "do";
var o = {
  [prefix + "It"]() { return this; },
  get [prefix + "Get"]() { return 1; },
  [prefix + "Arrow"]: () => 2
};
var getter = Object.getOwnPropertyDescriptor(o, "doGet").get;
o.doIt.name === "doIt" && o.doIt() === o && getter.name === "get doGet"
  && o.doGet === 1 && o.doArrow.name === "doArrow";
