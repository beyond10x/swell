// sec-object-environment-records-setmutablebinding-n-v-s: normal true
// sloppy-only
NaN = 1;
undefined = 1;
NaN !== NaN && typeof undefined === "undefined";
