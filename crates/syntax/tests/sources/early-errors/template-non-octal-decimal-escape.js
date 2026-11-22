// sec-static-semantics-template-early-errors: SyntaxError "invalid escape sequence in a template literal without a tag"
// `\8` is a NotEscapeSequence in a template, in sloppy code too.
x = `\8`;
