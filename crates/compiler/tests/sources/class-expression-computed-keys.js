// sec-runtime-semantics-classdefinitionevaluation: an anonymous class expression takes its name from the binding; each computed key is evaluated and converted by ToPropertyKey in source order
var B = class { [k]() {} static [k + 1]() {} set [k](v) {} };
