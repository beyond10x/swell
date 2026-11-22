// sec-encodeuricomponent-uricomponent: normal "true,true,true,true"
// Encode and Decode (sec-encode, sec-decode): the reserved set is kept by encodeURI and escaped by encodeURIComponent; each decode function undoes its encode function.
var s = "a b;/?:@&=+$,#-_.!~*'()%é€😀￿\u0000";
var component = encodeURIComponent(s);
var uri = encodeURI(s);
[
  component === "a%20b%3B%2F%3F%3A%40%26%3D%2B%24%2C%23-_.!~*'()%25%C3%A9%E2%82%AC%F0%9F%98%80%EF%BF%BF%00",
  uri === "a%20b;/?:@&=+$,#-_.!~*'()%25%C3%A9%E2%82%AC%F0%9F%98%80%EF%BF%BF%00",
  decodeURIComponent(component) === s,
  decodeURI(uri) === s && decodeURI("%3B%3b%41") === "%3B%3bA"
].join();
