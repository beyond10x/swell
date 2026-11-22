// sec-static-semantics-template-early-errors: parses
// The rule reads a template without a tag only: in a tagged template a NotEscapeSequence makes
// the TV of its part undefined (sec-static-semantics-tv).
tag`\unicode and \u{55}`;
tag`\xg${0}\01${1}\1`;
tag`\u{110000}`;
