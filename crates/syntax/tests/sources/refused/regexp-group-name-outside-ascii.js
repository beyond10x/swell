// sec-patterns-static-semantics-early-errors: refused "group name outside ASCII is not supported before slice 5"
x = /(?<é>a)\k<é>/;
