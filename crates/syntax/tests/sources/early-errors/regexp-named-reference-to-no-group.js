// sec-patterns-static-semantics-early-errors: SyntaxError "a reference to a group name no group has"
x = /(?<a>b)\k<c>/;
