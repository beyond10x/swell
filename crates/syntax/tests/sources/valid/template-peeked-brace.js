// sec-template-literals: parses
// The `}` that ends each substitution is the token the parser peeked at after `await` and
// `async`, lexed before the substitution ended; it is lexed again as a TemplateMiddle and a
// TemplateTail.
x = `a${await}b${async}c`;
