// sec-async-function-definitions-static-semantics-early-errors: SyntaxError "use strict directive in a function whose parameter list is not simple"
x = {
  async m(a = 0) {
    "use strict";
  },
};
