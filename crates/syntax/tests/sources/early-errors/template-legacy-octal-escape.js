// sec-static-semantics-template-early-errors: SyntaxError "invalid escape sequence in a template literal without a tag"
// A LegacyOctalEscapeSequence is no TemplateEscapeSequence: `\0` before a digit, and `\1`.
x = `${a}\01`;
