// sec-static-semantics-template-early-errors: SyntaxError "invalid escape sequence in a template literal without a tag"
// A code point past 0x10FFFF is a NotCodePoint, in the TemplateTail.
x = `${a}\u{110000}`;
