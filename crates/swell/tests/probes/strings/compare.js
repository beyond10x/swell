var a = "abc" + @N@, b = "abc" + @N@, n = 0; for (var i = 0; i < @N@; i++) if (a === b && !(a < b)) n++; throw new Test262Error('n=' + n);
