// sec-template-literal-lexical-components: parses
x = `\n\r\t\b\f\v\0\'\"\\\``;
x = `\x41A\u{41}\u{10FFFF}\a`;
x = `\$\{\}\${`;
x = `line \
continued`;
