// sec-static-semantics-template-early-errors: SyntaxError "invalid escape sequence in a template literal without a tag"
// `\x` without two hexadecimal digits is a NotEscapeSequence, whose TV is undefined.
x = `a\xg`;
