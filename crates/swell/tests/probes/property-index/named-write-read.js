var o = {}; for (var i = 0; i < @N@; i++) o['p' + i] = i; var s = 0; for (var i = 0; i < @N@; i++) s += o['p' + i]; throw new Test262Error('s=' + s);
