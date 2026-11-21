// sec-numeric-types-number-tostring: normal "0.30000000000000004,1e+21,true"
(0.1 + 0.2) + "," + 1e21 + "," + (1 / 0 === Infinity && -1 / 0 === -Infinity);
