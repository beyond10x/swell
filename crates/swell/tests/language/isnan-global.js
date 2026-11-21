// sec-isnan-number: normal "true|false|true|false|true"
isNaN("abc") + "|" + isNaN("12") + "|" + isNaN(new Number(NaN)) + "|" + isFinite(new Number(NaN)) + "|" + isFinite("  0x10  ");
