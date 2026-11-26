// sec-delete-operator-static-semantics-early-errors: SyntaxError "delete of a private reference"
class C { #x; m() { delete this.#x; } }
