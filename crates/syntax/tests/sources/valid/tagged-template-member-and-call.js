// sec-tagged-templates: parses
tag`a`;
tag`a${b}c${d}e`;
o.f`x`;
o[k]`x`;
f()`x`;
tag`a``b`;
new tag`x`;
new tag`x`(1);
(a?.b)`x`;
tag
`on the next line`;
tag`${tag`${0}`}`;
