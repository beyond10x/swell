// sec-global-environment-records-deletebinding-n: normal "false,true,undefined"
// sloppy-only
var x = 1;
y = 2;
(delete x) + "," + (delete y) + "," + typeof y;
