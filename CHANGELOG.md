# Changelog

What changed in each release of swell. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

A release of swell is a version of its source and, from 0.12.0 on, the `swell` shell binary for
Linux, macOS and Windows (`docs/releasing.md`, "The binary of a release").
Every number below comes from the gate, `task check`, on the released commit.

## [Unreleased]

### Added

- `Math.random`, a story of the fourth slice: each realm holds a xoshiro256** generator
  (`swell_num::random::Xoshiro256`) whose four words are derived, when the realm is made, from
  the agent's `Seeds.random_seed` and the realm's creation index by SipHash-2-4. The agent
  counts every `create_realm` that gets past the check of its state, one that throws or ends
  in a termination included, so no index is used twice. A call reads nothing from the host,
  calls no hook, costs the step of the call and allocates nothing; each value is a multiple
  of 2^-53 in [0, 1 - 2^-53] with positive sign. The function is the last own key of `Math`,
  after `sumPrecise` (`story:math-random`).
- The operators `**` and `**=` (`#sec-exp-operator`, `#sec-assignment-operators`): the parser
  reads `**` right associative and tighter than `*`, `/` and `%`, and a unary expression as its
  left operand outside parentheses (`-2 ** 2`, `typeof a ** 2`, `await a ** 2`) is the
  SyntaxError "a unary expression as the left operand of ** needs parentheses"; `**=` takes the
  targets `*=` takes. The compiler emits one instruction, `Exp`, and the interpreter applies
  Number::exponentiate through `swell_num::math::exponentiate`, the function `Math.pow` calls,
  so `a ** b` and `Math.pow(a, b)` answer the same Number. The compiler folds nothing. The
  refusal "exponentiation operator is not supported before slice 4" is gone
  (`story:exponentiation-operator`).
- The close of slice 4, `numbers-and-dates` (`story:slice-04-conformance-close`): every case
  of the slice passes in every run, `test262-slice 04 1681 3320 0 0 1681 1681 0 0`. Three
  cases that need a later slice move with evidence in `conformance/test262/moves.tsv`: the
  `Proxy` cases `Array/prototype/reverse/length-exceeding-integer-limit-with-proxy` and
  `Array/prototype/slice/length-exceeding-integer-limit-proxied-array` to slice 9, and
  `Math/sumPrecise/throws-on-non-number` (`0n`) to slice 12. 72 cases of
  `test/staging/sm/` that failed at the start of the slice and pass now come into slice 4
  from slice 19 by lines of the block (`docs/plan/slice-04/close-into-4.tsv`), each read
  case by case against the full run at the start commit of the slice. A test holds the
  nine references of the first consumer to `Date` and `Math` in one script, with the
  consumer's own calls and argument values
  (`consumer_slice_04::consumer_nine_references_of_surf_resolve_as_the_standard_says`). A
  second script calls the five parts of slice 4 the consumer calls (`Date.now`,
  `Math.floor`, `Math.min`, `Math.abs`, `Math.ceil`) with arguments that tell each one
  from its absence, and that show the sign of a zero; the four functions of `Math` are told
  from the identity, and `floor` and `ceil` from their rounding neighbours. Five tests run
  it: `parts_script_answers_as_the_standard_says`,
  `parts_script_fails_without_each_part_of_slice_4` (all five parts),
  `parts_script_tells_each_math_function_from_the_identity` (the four functions of
  `Math`), `parts_script_tells_each_rounding_function_from_its_neighbours` (`floor` and
  `ceil`, each against the other three of `floor`, `ceil`, `round` and `trunc`) and
  `parts_script_sees_the_sign_of_each_zero` (the four functions of `Math`). Slice 4 is
  delivered: `conformance/slices-delivered.txt` lists it, and no host requirement is pending
  for it.
- `Reflect`, the first story of the ninth slice (`#sec-reflect-object`): the global `Reflect`,
  an ordinary object of `%Object.prototype%` with its 13 functions in the order of their
  clauses (`apply`, `construct`, `defineProperty`, `deleteProperty`, `get`,
  `getOwnPropertyDescriptor`, `getPrototypeOf`, `has`, `isExtensible`, `ownKeys`,
  `preventExtensions`, `set`, `setPrototypeOf`) and `Reflect[Symbol.toStringTag]` "Reflect".
  Each function calls one internal method of its target after the checks of its clause, and
  throws a TypeError for a target that is no object (for `apply` not callable, for
  `construct` no constructor) before it reads another argument. `apply` and `construct` copy
  their argument list as `Function.prototype.apply` does: a step per element, each element
  charged to the memory budget before it is kept, a re-entry per call; `ownKeys` charges the
  list of its values before it makes it and a step per element of the Array it answers
  (`story:reflect-namespace`).
- `Map` and `Set` with their prototypes, `get [Symbol.species]`, and the Map and Set Iterators
  (`#sec-map-objects`, `#sec-set-objects`), on one ordered entry table
  (`object/keyed_entries.rs`, `EntryTable`): entries in insertion order with a hole per
  deletion, an index map once the table's room passes 8 entries, keys compared by
  SameValueZero after CanonicalizeKeyedCollectionKey (`-0` as `+0`, every NaN one key, a
  String as its atom, so `"0"` and `0` stay two keys), and cursors that a compaction and
  `clear` move, so `forEach` and an iterator visit an entry added before they reach it and
  skip one deleted before it. Every operation charges its steps before its work and every
  growth its bytes; the collector drops the cursors of dead iterators. A cursor id carries a
  generation, so a stale id reaches no cursor (`story:keyed-entry-table`,
  `story:map-and-set`).
- Proxy objects (`story:proxy-objects-and-access-traps`): the global `Proxy` (a constructor of
  `length` 2 with no "prototype"; a call without `new`, or a target or handler that is not an
  object, is a TypeError) and `Proxy.revocable`, whose `{ proxy, revoke }` object holds a
  revoke function of `length` 0 and name "" that drops the Proxy's target and handler on its
  first call. Every internal method of an object has a Proxy arm: `get`, `set`, `has`,
  `deleteProperty`, `apply` and `construct` call their trap on the handler and check its
  answer against the invariants of `#sec-proxy-object-internal-methods-and-internal-slots` (a
  broken one is a TypeError that names the trap and the key), reading the target's descriptor
  and extensibility only at the step of the clause; the seven other methods check for
  revocation and forward to the target. A revoked Proxy throws a TypeError from each method.
  The walks of the prototype chain hand a key over to a Proxy parent's [[Get]],
  [[HasProperty]] or [[Set]] with the same receiver; IsArray and GetFunctionRealm follow the
  target. Every internal method of a Proxy is one re-entry for its duration and spends a step
  before it reads the handler, so a chain of proxies of any length ends in the RangeError of
  `reentry_depth`, not in a stack overflow.
- `super.x` and `super[e]` in the methods, getters and setters of object literals, plain,
  generator, async and async generator ones, and in their parameters
  (`#sec-super-keyword`, `#sec-makesuperpropertyreference`): the base is the [[Prototype]] of
  the method's [[HomeObject]] when the reference is made, the receiver is `this`, and ToObject
  of a `null` base throws a `TypeError` before a computed key is converted. Reads, calls
  (`this` is the receiver), tagged templates, `?.`, assignment, compound and logical
  assignment, `++` and `--`, destructuring targets and `for`-`in` heads work on it; `delete`
  of one throws a `ReferenceError` without converting the key. The compiler emits seven
  instructions, `SuperBase`, `GetSuperNamed`, `GetSuperIndex`, `PutSuperNamed`,
  `PutSuperIndex`, `SuperKey` and `DeleteSuper`. A SuperProperty outside a method (a plain
  function, generator, async function, its parameters, global code, an arrow at the top) is
  the early SyntaxError "super property outside a method", a SuperCall outside a derived class
  constructor is "super call outside a derived class constructor". The refusal `super is not
  supported before slice 8` is gone; `super` in an arrow function inside a method and `super`
  in eval code run since `story:derived-classes-and-super-call`
  (`story:super-property-and-super-syntax`).
- `new.target` parses and runs in functions, arrow functions, generators, `async` functions
  and direct eval code (`story:new-target-meta-property`).
- `WeakMap` and `WeakSet` (`#sec-weakmap-objects`, `#sec-weakset-objects`), a story of the ninth
  slice: the constructors with their adders `set` and `add` (the loop of `Map` and `Set`, a step
  per entry), `WeakMap.prototype.delete`, `get`, `has`, `set`, `getOrInsert` and
  `getOrInsertComputed`, `WeakSet.prototype.add`, `delete` and `has`, and their
  `@@toStringTag`. A key is an object or a Symbol that CanBeHeldWeakly (`#sec-canbeheldweakly`):
  a Symbol of `Symbol.for` is refused with a `TypeError` by `set`, `add` and `getOrInsert*`, a
  well-known Symbol is accepted. The table is the keyed hash map of `heap/map.rs` with the
  heap's hasher, each growth charged before it and credited when the object is freed; a rehash
  of the table in place, once deleted entries used up its room, is charged nothing; the box of
  the kind is charged with the object. The collector holds the entries weakly by linear
  ephemeron marking: a WeakMap's trace reports the value of an entry whose key is marked and
  puts any other entry on a pending list headed at its key's slot, which the mark phase empties
  when the key is marked; after marking, every marked weak table drops its entries whose key is
  not marked, so a key reachable only through the values of such entries is freed with them,
  in one collection (`story:weak-map-and-weak-set`).
- The set methods of `Set.prototype`, a story of the ninth slice: `union`, `intersection`,
  `difference`, `symmetricDifference`, `isSubsetOf`, `isSupersetOf` and `isDisjointFrom`, with
  GetSetRecord over a set-like (its `size`, `has` and `keys`). Each picks the side it iterates
  by the clause's size comparison, walks `this` with a cursor of its table (an entry a `has`
  callback deletes before the walk reaches it is skipped, one it adds is visited), closes the
  set-like's iterator where the clause closes it, and answers a new plain `Set` of the current
  realm (no species). One step per entry of `this` visited and per key read from the set-like,
  the result's table charged to the memory budget, every callback counted against the depth
  bounds. The own keys of `%Set.prototype%` stand in the order of the clauses
  (`story:set-methods`).

- Class declarations and class expressions without `extends`, three stories of the eighth
  slice (`#sec-class-definitions`, `#sec-runtime-semantics-classdefinitionevaluation`). The
  parser reads a class where a declaration stands and as an expression, with methods, getters,
  setters, generator, async and async generator methods, `static` or not, with literal, string,
  number or computed keys, and empty elements; a class is strict code from `class` to its `}`.
  Its early errors are answered: a second `constructor`, a `constructor` that is a special
  method, a `static` method named `prototype`, a reserved or restricted class name, the strict
  rules in its methods, and the redeclaration of its name in a block, a `switch` and a script
  (`story:class-syntax-and-early-errors`). A class runs: the class scope with its own name
  uninitialized and immutable while the computed keys run, the prototype with
  `%Object.prototype%` as [[Prototype]], the constructor F made from the constructor's template
  (or the default base constructor) with the prototype as [[HomeObject]] and its name at
  creation, `prototype` (not writable, not enumerable, not configurable) and `constructor`,
  then the methods, getters and setters in source order, non-enumerable, on the prototype or,
  for a static one, on F; a computed key is converted by ToPropertyKey once, and a static
  element keyed `prototype` throws a `TypeError`. A class constructor called without `new`
  throws a `TypeError` of its own realm; `new` makes `this` from NewTarget (through a bound
  function too). A class at the top of a script binds in the declarative record of the global
  environment; in a block and in eval code it binds lexically. The computed keys of a class in
  non-strict code run as strict mode code (`#sec-strict-mode-code`): an assignment to an
  undeclared name throws a `ReferenceError`, a write to a frozen property and `delete` of a
  non-configurable one throw a `TypeError`, a direct `eval` there keeps its `var` to itself,
  and a class, function or arrow function made there is strict; the code around the class, its
  `catch` and `finally`, and a frame resumed after a `yield` in a key keep their own
  strictness. An anonymous class under a computed key of an object literal takes the key's
  name before its elements are defined, so a static `name` method wins and `Reflect.ownKeys`
  lists `length`, `name`, `prototype` first. `Function.prototype.toString` of a class gives
  its source text from `class` to `}`, of a method its source text without a leading `static`.
  The compiler emits thirteen instructions, `ClassPrototype`, `MakeClassConstructor`,
  `DefineClassMethod`, `DefineClassGetter`, `DefineClassSetter`, their three computed forms,
  `StoreInitializer`, `MakeMethod`, `InitInstance`, `SetStrict` and `SetClassName`; a class
  constructor carries a box of its kind (`ConstructorKind::Base`) and its instance
  initializer, charged with F. The refusal `class is not supported before slice 8` is gone
  (`story:class-definition-evaluation`).
- The parser reads every other element of a class and holds it: fields, public and private,
  static or not, with or without an initializer, ending at `;`, before `}` or where a line
  terminator lets a semicolon be inserted, with `static`, `get`, `set` and `async` as field
  names; private methods and accessors (`#m() {}`, `get #x() {}`, `static async *#m() {}`);
  static blocks; the private name token (`#` and an identifier name, escapes included); the
  member forms `a.#x` and `a?.#x` and `#x in a`, whose private name stands only directly
  before `in` where a relational expression may start. The tree holds the functions of the
  elements: one `FieldInitializer` per run of consecutive static fields, one for the instance
  elements of a class with a field or a private method that is not static, one `StaticBlock`
  per static block. Their early errors are answered: a field named `constructor`, a static
  field named `prototype` or `constructor`, `#constructor`, a private name declared twice but
  for a getter and setter pair of the same placement, `arguments` in a field initializer or a
  static block (arrow functions in them included), a SuperCall there, `await` as an identifier
  and `return` in a static block (a class nested in it included), a YieldExpression or an
  AwaitExpression in a field initializer (refused at its token, a stated difference from the
  pinned draft: no runtime can run one in an initializer's own function), the var and lexical
  names of a static block, AllPrivateIdentifiersValid in a script, a function body, `Function`
  code, eval code and a class body (a class's heritage excepted), `delete` of a private
  reference, and a private name in an object literal or after `super.`. A source that has
  such an element and no early error answers its hold: `class field`, `private method`,
  `private field`, `private name in` or `class static block is not supported before slice 8`;
  a heritage answers `class heritage is not supported before slice 8`. The refusal `private
  name is not supported before slice 8` of the lexer is gone
  (`story:class-element-syntax-and-early-errors`).

- The seven traps of shape of Proxy exotic objects: `getPrototypeOf`, `setPrototypeOf`,
  `isExtensible`, `preventExtensions`, `getOwnPropertyDescriptor`, `defineProperty` and
  `ownKeys` (`#sec-proxy-object-internal-methods-and-internal-slots`, 10.5.1 to 10.5.6 and
  10.5.11). Each is called with the clause's arguments and its answer is checked against the
  target by the 21 invariants of those clauses; the target is read only at the steps of the
  clause and only under their conditions. A broken invariant throws a `TypeError` that names
  the trap, the key and the invariant's number (`I1` to `I21`). The answer of `ownKeys` is
  made a list by CreateListFromArrayLike with a step per element, each check of the clause
  spends a step per key, and the map of the answer's keys and the list of the target's
  non-configurable keys are charged to the memory budget before they are made: a trap that
  answers an array-like of length 2^32 - 1 ends at the step budget or the memory budget. A
  trap runs code and the collector may run inside it, so every value used after a trap is kept
  until its last use (`JSON.stringify` and the reviver walk of `JSON.parse`, `Object.keys`,
  `values` and `entries`, `for`-`in`, OrdinaryHasInstance and
  `Object.prototype.isPrototypeOf`); a trap's arguments are released when it returns, and no
  internal method of a Proxy keeps more than 10 values outside a charged room.
  `for`-`in` over an object whose prototype chain has a Proxy reads whether a key is still a
  property with [[GetOwnProperty]] along the chain its enumeration walked: it calls no `has`
  trap, a `getPrototypeOf` trap is called once per object, and the keys of the prototype such
  a trap answered are visited. A chain of ordinary objects costs the steps it cost before
  (`story:proxy-descriptor-prototype-and-extensibility-traps`).

- Public class fields, instance and static (`#sec-class-definitions`,
  `#sec-runtime-semantics-classfielddefinitionevaluation`, `#sec-definefield`): a computed field
  key runs once, when the class is defined, in source order with the keys of the methods, is
  converted by ToPropertyKey there and kept in a hidden binding of the class scope; each
  construction defines the instance fields in source order on the new object before the
  constructor's body runs, each initializer evaluated (or `undefined`) with the instance as
  `this` and `new.target` undefined, by CreateDataPropertyOrThrow, so a field over a prototype
  accessor is an own data property and a non-extensible object or a non-configurable property
  throws a `TypeError`; the static fields are defined on the class in source order after every
  method, each run of them a function called with the class as `this`; `super.x` in an
  initializer reads through the prototype (or the class, for a static field); an anonymous
  function or class takes the field's name. The initializers run as frames of the run loop
  (`InitInstance`, `Call(0)`), so the step budget, the interrupt and the frame depth reach
  them. No instruction is added. The refusal `class field is not supported before slice 8` is
  gone. A direct `eval` in a field initializer (an arrow in one too) runs since
  `story:eval-in-class-field-initializers`; an indirect `eval` there ran before it
  (`story:public-class-fields`).

- Derived classes and `super(...)` (`#sec-runtime-semantics-classdefinitionevaluation`,
  `#sec-super-keyword-runtime-semantics-evaluation`,
  `#sec-ecmascript-function-objects-construct-argumentslist-newtarget`): `class C extends H`
  in declarations and expressions, with `H` a constructor or `null`. The heritage sets both
  prototype chains (constructor and prototype); `extends null` makes a prototype without a
  parent, and a non-constructor heritage or a `prototype` that is neither an object nor `null`
  throws a `TypeError`. A derived constructor's `this` is a hidden binding of its function
  scope, uninitialized until `super(...)` returns: a read before throws a `ReferenceError`, a
  second `super(...)` throws a `ReferenceError` after the parent ran. `super(...)` works in the
  constructor, in its parameters, in arrow functions within it and in direct eval code in it.
  The derived return rule (an object, `undefined` with `this` bound, else a `TypeError`, or a
  `ReferenceError` when `this` is unbound) throws its errors in the caller's realm, for a frame
  of the running loop and for a [[Construct]] from Rust (a bound function,
  `Reflect.construct`, the host). The default derived constructor passes its arguments to the
  parent without an iterator, a step per argument. A chain of 1,000 derived classes runs one
  frame per level and no re-entry. The fields of a derived class are defined when `super(...)`
  returns. `super` in an arrow within a method and `super.x` in direct eval code in a method
  now run. An arrow inside a derived constructor keeps the constructor's `this` binding in the
  function environment whether or not it reads `this` (184 bytes more per kept construction,
  measured over 100 instances). The compiler emits `ClassHeritage`, `GetSuperConstructor`,
  `SuperCall`, `SuperCallSpread`, `SuperCallKept`, `BindThis`, `BindThisEnv`, `BindThisName`,
  `KeepThis`, `KeepThisEnv` and `UninitializeEnv`. The refusal `class heritage is not
  supported before slice 8` is gone (`story:derived-classes-and-super-call`).

- `eval` in a class field initializer (`#sec-performeval`,
  `#sec-static-semantics-containsarguments`): a direct eval whose GetThisEnvironment is a
  field initializer's (an instance or a static field, or an arrow function in one) runs with
  inFunction and inMethod set, inDerivedConstructor clear and inClassFieldInitializer set,
  read from the template kind `FieldInitializer`: `new.target` in the eval code is
  `undefined`, `super.x` reads through the initializer's home object, `super()` is a
  `SyntaxError` (in a derived class too), `this` is the instance (the class in a static
  field), a `var` stays in the eval's own record, and ContainsArguments of the eval code
  (`arguments` outside every function that is not an arrow function) is the `SyntaxError`
  `arguments in eval code in a class field initializer`, answered before any other early
  error and before a held refusal of the eval code. An indirect eval there has every flag
  clear. The refusal `eval in a class field initializer is not supported before slice 8` is
  gone (`story:eval-in-class-field-initializers`).

- Private methods and accessors run (`#sec-private-names`,
  `#sec-privateelement-specification-type`): each evaluation of a class makes one Private Name
  per private identifier (a heap record of its own, its description the identifier with its
  `#`), held by the class's private record, a lexical scope pushed after the heritage. An
  object holds its private elements in a list of its own, charged to the memory budget before
  it grows: a box of 24 bytes and 44 bytes per element, reserved once per class level for
  exactly the elements that level adds (a getter and a setter of one name are one element), so
  an instance of a class with one private method costs 68 bytes over a plain instance, with 4
  methods 200, with 17 methods 772
  (`private_methods::an_instance_pays_for_the_room_of_its_list`). The instance's private
  methods and accessors are added by the class's instance initializer before any other
  element, the static ones to the constructor before the first static element runs
  (PrivateMethodOrAccessorAdd); an object a base constructor returns gets them too, and a
  second add of the same name is a `TypeError`. `o.#m`, `o.#m()` with `o` as `this`,
  `o?.#m()`, and every assignment form on a private reference (plain, compound, logical,
  update, destructuring and the `for`-`in`/`of` head) run PrivateGet and PrivateSet, which
  throw a `TypeError` for a missing element, a primitive, a write to a method and a read or
  write of an accessor that lacks that half. A find spends a step per comparison past the
  eighth entry, so a construction that adds n private elements of one class level spends
  (n - 9)(n - 8)/2 steps on its adds for n over 9 and none below. Private names are not
  visible to any listing of the language or of the host interface; the collector frees a
  Private Name that nothing holds. One class holds at most 32,768 private methods with names
  of their own (the 65,536 slots of its private record); past it the class is the
  `SyntaxError` `more private record slots in one class than the bytecode can number`. Direct
  eval code that reads a private name an enclosing class declares is refused with the
  `SyntaxError` `private name #x in direct eval code is not supported before slice 8` until
  `story:private-names-in-eval-code`; a name no enclosing class declares, an indirect `eval`
  and `Function` keep the early error `private name #x is not declared in an enclosing
  class`. The hold `private method is not supported before slice 8` is gone
  (`story:private-names-and-private-methods`).

- Host object overrides, a story of the ninth slice: a host object may carry an override
  table, `Overrides<H>`, with one optional function per essential internal method. An
  override's answer, and an answer of the object's record where the table leaves an override
  out, is checked against the invariants of ECMA-262 6.1.7.3, read as facts of the object's
  own earlier answers; a broken invariant ends the run in `Terminated(EngineFault)`. "Host
  interface" below names the surface and the one breaking change, `HostObjectSpec`
  (`story:host-object-overrides`).

- Class static blocks (`#sec-class-definitions`, ClassStaticBlock;
  `#sec-runtime-semantics-classstaticblockdefinitionevaluation`): each block is a function of its
  own, made as a method with home object F and called with `this` F at its place among the static
  elements, in source order with the runs of static fields, after every method and computed key.
  The block has a `var` scope of its own (its `var`, `let`, `const` and function declarations are
  not visible outside it or in the next block), `new.target` is `undefined`, `super.x` reads
  through F, `await` is reserved in it and is a binding name in a function inside it; a throw
  from a block ends the class's definition with the class's own outer binding uninitialized. A
  direct `eval` in a block runs with inFunction and inMethod set and inClassFieldInitializer and
  inDerivedConstructor clear: `eval("arguments")` is the block's own arguments object (made only
  when the block's code has a direct `eval`), `super()` in the eval code is a `SyntaxError`. The
  block runs as a frame of the run loop (`Call(0)`), so the step budget, the interrupt and the
  frame depth reach it. No instruction is added. The refusal `class static block is not supported
  before slice 8` is gone (`story:class-static-blocks`).

- `Map.prototype.getOrInsert`, `Map.prototype.getOrInsertComputed` and `Map.groupBy`, a story
  of the ninth slice (`story:map-upsert-and-group-by`), the group `builtins/map/upsert.rs`.
  `getOrInsert` answers the value of a key the Map holds and appends the entry otherwise;
  `getOrInsertComputed` checks that its callback is callable before the lookup, calls it only
  for a missing key, with the key after CanonicalizeKeyedCollectionKey (`+0` for `-0`), and
  stores its answer with one insertion of the entry table, so a key the callback set keeps its
  place and takes the answer, with no second entry. A present key whose value is `undefined`
  is found by both. `Map.groupBy` runs the loop of GroupBy that `Object.groupBy` runs, now with
  the keyCoercion collection beside property: keys compared by SameValueZero (`-0` and `+0` one
  group, every NaN one group, objects by identity), the result Map made and kept before the
  first element is read, each group's Array stored in its table when its key first appears.
  One step per element before the callback; the table and each group's Array charged to the
  memory budget (a run with one growing group and a run with a new group per element both end
  at the budget; that the table is charged before it grows is held by reading
  `object/keyed_entries.rs`, not by a test); the callbacks and `next` counted against the
  depth bounds; an abrupt callback closes the iterator. The messages of the shared steps name
  the method (`Map.groupBy called on undefined or null`, `Map.groupBy: the callback is not a
  function`); those of `Object.groupBy` read as before. 15 tests in
  `tests/builtins_map_upsert.rs` (14 with `collect-every-safe-point`).

### Changed

- A character class outside `v` keeps its last answer together with the range of code
  points around it that answer the same (`ClassSet::contains_around`, with
  `swell_unicode::Property::contains_around` for the property tables). A run of characters
  from one range costs one search instead of one per character. Steps, memory charges and
  results are unchanged. On a sample of 37 of the 442 generated property-escape files of
  test262, one worker: 52.7 s before, 49.2 to 49.6 s after.
- The lane `slow` runs its collect mode in the profile `collect` (opt-level 1), except for
  the tests that run on a bounded stack, the tests a macro writes and the library's unit
  tests, which stay at opt-level 0. This covers `test-realm-sizes` and the full counts of
  `slow-tests`. A stack test that the rule misses fails the run at opt-level 1 by itself:
  that run sets `SWELL_NO_STACK_TESTS`, and `stack_with_room` and the new `stack_thread` of the
  tests' support panic when it is set; the six direct `Builder::stack_size` calls of the
  tests go through `stack_thread`. The lane builds the profile `collect` in the background
  beside `slow-test262`, and ends that build when it ends. `taskfiles/realm-room-tests.awk`
  gains `-v rule=stack` and reads more forms (a function nested in another, a function passed
  by value, in an array or as a field's value, a path to another module or to the helpers,
  every definition of a twice-defined name); the forms it does not read are listed at its
  head. After the merge it prints 73 stack tests and 304 room tests of 1,644. The count line of
  `slow-tests` counts what runs (a test compiled out of the collect build apart). The
  workflow `slow` also starts on a change of `.cargo/collect-profile.toml`
  (`story:realm-size-lane-within-its-share`).
- The realm holds the three functions more: `heap_bytes()` after `create_realm` reads 394,751
  bytes, before 393,818; `live_bytes()` 282,792, before 281,115 (the test
  `realm_padding::a_padded_realm_holds_k_functions_more`, k = 0; at k = 200 645,964 and 428,053,
  before 645,031 and 426,376). The reading of HOST-187 is 398,621 bytes, unchanged, its bound
  `BINDINGS_REALM_HEAP_BYTES` (506,112) unchanged; `heap_bytes()` in `host_187` after the build
  793,372 and 795,036 (792,439 and 794,103), after the scripts 799,499 in the default mode and
  790,919 in the collect mode (798,566 and 789,986). `host_186` unchanged: a host function
  512, a host object 208, with 4,096 bytes of data 4,624 and 4,304, 300 definitions 107,816.
- The walk of `builtins_symbol_values` reads 347 places
  (`crates/swell/tests/fixtures/realm_places.txt`), before 344, in 2 parts of 183 + 164
  (181 + 163); no row of `WALK_EXPECTED` added (each of the three throws the `TypeError` of
  RequireInternalSlot, RequireObjectCoercible or IsCallable before a Symbol is read).
- The walk that calls every function of the realm with a Symbol in each place runs in 2
  parts and reads the places of the functions from a committed file,
  `crates/swell/tests/fixtures/realm_places.txt` (344 places), which the task
  `realm-places-expected` writes. A place is in the part its line hashes to (FNV-1a, 64 bit),
  so a function the realm gains moves no other place. Each part takes from the file the
  places, calls and rows it owes and fails when it walked, called or met fewer. The test
  `the_realm_places_file_equals_the_walk` makes the list again in the default mode at every
  gate run and fails when the file differs, its two comment lines included; before the writer
  writes, it reads every place back to its function and fails naming one that does not read
  back. The cover test `the_parts_of_the_walk_cover_every_function_of_the_realm` holds the
  file's form, the partition, no empty part, no part under half the mean, at most 8 parts and
  the rows of `WALK_EXPECTED`, two of which (`Array.prototype.with` with the Symbol as
  argument 2 or 3) carry the condition their clause tests, an index at least the length that
  earlier calls left. The slowest part takes 0.022 CPU seconds alone in the default mode,
  0.317 with a collection at every safe point and 0.672 there at the full count, against
  0.069, 4.10 and 6.59 for the one walk before. The gate's steps `build`, `build-tests`, `test`
  and `test-doc` read their flags and nextest arguments from `TEST_RUSTFLAGS` and
  `TEST_NEXTEST_ARGS` of `taskfiles/realm-places.yml`, which the writer reads too, list both
  in `gate-vars`, and fail with one line when either is empty or is not that file's value
  (`story:realm-walk-in-parts-under-the-cpu-bound`).

- The record of every object is 208 bytes, before 200: it holds the box of the object's
  private elements (`private_elements`, empty for an object without one). Every reading that
  counts objects rises by 8 bytes per object, over the readings of the next entry:
  `heap_bytes()` after `create_realm` reads 393,818 bytes (389,722 without the field),
  `live_bytes()` 281,115 (277,947); the reading of HOST-187 is 398,621 bytes (394,525), its
  bound `BINDINGS_REALM_HEAP_BYTES` (506,112) unchanged; a host function reads 512 live bytes
  (504) and a host object 208 (200), with 4,096 bytes of host data 4,624 (4,616) and 4,304
  (4,296) (`host_186`, `host_187`). `JSON.parse` of 100,000 nested brackets, which returned
  its result under a memory budget of 64 MiB, needs 66,304 KiB now
  (`story:private-names-and-private-methods`).

- A realm reads larger. The collector's workspace holds a head word per slot of every arena
  besides the mark word and the room on the worklist, charged with the arenas' capacity
  (`arena::slot_bytes`, one `u32` per slot more), and the realm holds the intrinsics added
  since 0.12.0, among them `Reflect`, `Map`, `Set`, `WeakMap`, `WeakSet` and `Proxy`.
  `heap_bytes()` after `create_realm` reads 389,722 bytes, 357,960 in 0.12.0; `live_bytes()`
  277,947, before 237,374 (the test `realm_padding::a_padded_realm_holds_k_functions_more`
  prints both, in the default mode). The reading of HOST-187, a realm with 300 host functions
  and 40 host objects, is 394,525 bytes, before 386,781; its bound `BINDINGS_REALM_HEAP_BYTES`
  (506,112) is unchanged (`story:weak-map-and-weak-set`).

- A Proxy method called directly by a builtin no longer counts one re-entry more than code
  that the builtin calls: the builtin's activation is that re-entry, as in
  `Agent::enter_code`. `Object.keys` of a Proxy whose `ownKeys` trap recurses reaches
  `reentry_depth`, where it stopped one level short
  (`story:proxy-descriptor-prototype-and-extensibility-traps`).

- The lane `slow` states its bound: `task slow` fails when it ran over 900 s or one of its
  parts over 600 s, naming the part and its seconds (`slow-time-sum`); it also fails a lane
  whose clock ran over 900 s, so a step without its own time line is bounded. `test-realm-sizes`
  runs the tests that read the realm's room (by the rule of `taskfiles/realm-room-tests.awk`)
  in the default mode and with a collection at every safe point at every size of
  `REALM_SIZES`, on the builds `slow-tests` makes; it no longer builds the profile `collect`,
  and it hands nextest its list of tests in a generated configuration file, so it runs at any
  count of tests. `builtins_json_parse::parse_gives_back_its_charges_on_every_way_out` nests
  200 levels in the collect mode without `SWELL_FULL_COUNTS` (2,000 with it and in the
  default mode). The workflow `slow` starts on a change of `taskfiles/gate-time.yml`,
  `taskfiles/realm-room-tests.awk` and `.config/nextest.toml` too
  (`story:realm-size-lane-within-its-share`).
- The lane `slow` builds the stack tests' binary with their modules only:
  `build-stack-profiles` passes `--cfg test_modules` and the modules of `STACK_MODULE_LIST`
  (`taskfiles/stack-profiles.yml`) to the test target of `swell` through `cargo rustc`, so
  the release and `stack-debug` builds compile `limits_scripts`, `eval_stack` and `support`
  (9 tests) and not every test, and the library's release build stays the test262 runner's.
  `test-stack-profiles` builds them first and runs them through cargo-nextest.
  `stack-modules-check` refuses a stack test outside the listed modules and a test named
  twice. The generator `taskfiles/test-main.sh` writes the module cfgs into `swell`'s
  `tests/main.rs`, and `crates/swell/Cargo.toml` declares the cfg `test_modules`, so the lint
  `unexpected_cfgs` still refuses a misspelled feature in a test module; every build of the
  gate compiles every module, as before, and `gate-vars` fails when a caller's `RUSTFLAGS`
  (or `CARGO_ENCODED_RUSTFLAGS`, `CARGO_BUILD_RUSTFLAGS`, `CARGO_TARGET_<triple>_RUSTFLAGS`)
  names `test_modules`.
- The tests over 2 CPU seconds alone in the lane's collect build (a collection at every safe
  point, opt-level 0) run under that bound: smaller counts in collect mode for `host_133`, the
  memory-budget fill of `termination`, five tests of Map and Set, two of async generators,
  `for_await`, `property_index`, `tagged_templates` and two of `new_target`; with
  `SWELL_FULL_COUNTS=1` four of them run their earlier count again, under 2.5 s. The surface
  test of Map and Set is three tests, its random operations two tests over two seeds, the
  re-entry through `super` four, the parse cases of `new.target` two; the `language` scripts
  run in 47 parts, the 13 try-completion scripts each split by `break` and `continue`, and
  `test-realm-sizes` no longer runs the parts, which read no room of the realm. Every smaller
  count was shown red with the defect it tests planted. The collect runs of
  `test-realm-sizes` went from 2,588 to 1,626 test-seconds on the build node (the sums of
  cargo-nextest's JUnit files, measured at different loads)
  (`story:realm-size-lane-within-its-share`).
- `swell_syntax::parse_eval` and `parse_eval_with` take `context: impl Into<EvalContext>` in
  place of `strict_caller: bool`: the new public `EvalContext` holds the caller's strictness
  and PerformEval's four flags (`in_function`, `in_method`, `in_derived_constructor`,
  `in_class_field_initializer`), and `EvalContext: From<bool>` keeps every call with a `bool`
  compiling (`story:new-target-meta-property`).
- Releases ship seven archives of the `swell` binary and `SHA256SUMS`: besides the five
  targets of 0.12.0, `.github/workflows/release-build.yml` builds `x86_64-unknown-linux-musl`
  and `aarch64-unknown-linux-musl`, static binaries that load no shared library; the glibc
  archives stay (glibc 2.34 or newer). The Windows binary links its C runtime in
  (`-C target-feature=+crt-static`) and needs no Visual C++ runtime. After the smoke test each
  musl job fails on a dynamic section entry `NEEDED` or a program interpreter and runs the
  binary in a Docker image that holds nothing but it; the Windows job fails when the import
  table names a `VCRUNTIME` or `api-ms-win-crt` library. The workflow also runs on a pull
  request that changes it (labelled with the pull request's head commit, nothing uploaded)
  and when started by hand. The new gate step `release-targets-check` (part of
  `forge-check`) reads the workflow and the two documents as text and fails when the matrix
  is not the release's seven targets, when the Build step's `case "$TARGET"` gives a musl or
  Windows target no `+crt-static` (or a glibc or macOS target one), when no step that runs
  for a musl or Windows target checks its linking, or when `docs/releasing.md` or
  `website/docs/embedding.md` names or counts other archives than the workflow builds
  (`story:releases-ship-static-binaries`).

- The engine crate `swell` is `#![no_std]` with `alloc`, step 3 of story
  `core-builds-without-std`: its library code names no `std` path, and the program that links
  it provides the allocator. The float functions it called as `std` methods (`trunc`,
  `floor`, `ceil`, `sqrt`, `rem_euclid`) and the `%` of two Numbers (`fmod`, which a no_std
  build would take from the C library) come from `swell_num::fmath`, whose six functions are
  `#[inline]`. The test-build counters of collections, stored Strings and drains of the
  release queue are fields of the heap and the root table instead of thread-locals. The
  public interface and the test262 counts do not change.
- The keyed hash map of the engine (`heap/map.rs`) is written in the crate instead of
  `std`'s `HashMap`: a SwissTable with groups of 16 control bytes, read as two 64-bit words,
  and the SipHash keyed by `hash_key` (HOST-145). It has the growth, deletion and rehashing
  of the map it replaces, so a table grows and is charged at the same operations as before; a
  unit test compares it with `std`'s map over 20,000 operations. Four paths of the engine use
  it: the index of a property list longer than 8 entries (`object/property.rs`, every
  property of a large object and every global variable), the atom table of interned Strings
  (`heap/string.rs`), the registry of `Symbol.for` (`heap/mod.rs`) and the set of visited
  keys of `for`-`in` (`ops/objects.rs`). The `swell` shell against main (`f27d50d7`), user
  CPU, best of 7 on the build node, main's binary run twice as the noise floor (load 48 to 56
  on 32 cores):

  | script | what it does | main | this change | main again |
  |---|---|---|---|---|
  | `num_2k` | an object of 2,000 Number keys: add, read 5 times, delete half; 30 rounds | 0.994 s | +7.8 % | +1.5 % |
  | `num_200k` | the same with 200,000 keys, 1 round | 3.717 s | +4.1 % | -0.8 % |
  | `str_2k` | 2,000 String keys made once | 1.286 s | +2.6 % | +1.5 % |
  | `str_200k` | 200,000 String keys made and interned at run time | 6.656 s | +4.1 % | -0.2 % |
  | `forin_2k` | `for`-`in` over 2,000 keys, 300 rounds | 0.110 s | -11.8 % | -4.5 % |
  | `control` | arithmetic on local variables and a dense array, no keyed map | 0.229 s | -3.9 % | -0.9 % |

  The map alone against `std`'s with the same hasher, `u32` keys, 2,000 entries, three runs
  (first version of this change, then this one):

  | operation | first version | this change |
  |---|---|---|
  | insert | +12 to +19 % | +3 to +8 % |
  | lookup, key present | +22 to +24 % | +12 to +18 % |
  | lookup, key absent | +14 to +17 % | +10 to +14 % |
  | remove | +31 to +45 % | +1 to +8 % |

  The rest of the lookup gap is `std`'s match of a group with SSE2 instructions, which need
  `unsafe`; the engine crates have none, also not for speed.
- `swell-num` builds without `std` (`#![no_std]` with `alloc`), step 2 of story
  `core-builds-without-std`: it names no `std::` path outside its unit tests and calls no
  `f64` method of `std`. The new module `swell_num::fmath` computes `sqrt`, `floor`, `ceil`,
  `trunc`, `fmod` (the `%` of two `f64`, which the compiler otherwise lowers to a call of the
  C math library's `fmod`) and `rem_euclid` in integer arithmetic, ported from fdlibm (FreeBSD
  msun; notice in `NOTICE`); each result is the one IEEE 754 fixes. `sqrt` corrects an `f64`
  estimate in `u128` integers (the fdlibm root bit by bit stays as the oracle of the lib tests),
  and `fmod` of two integers below 2^53 takes the `i64` remainder. A million calls on the build
  node take, `std` against `fmath`: `sqrt` 1.2 ms and 21.0 ms, `floor` 5.6 ms and 5.0 ms,
  `trunc` 1.6 ms and 1.3 ms, `%` of integers 29.4 ms and 12.8 ms, of fractions 23.9 ms and
  33.4 ms. A corpus of 2,124 calls
  (`crates/num/tests/fixtures/fmath/corpus.txt`) holds the results, and exact arithmetic
  holds them again. Against `std` on x86-64 Linux the corpus differs in no result but the bits
  of a NaN: `sqrt` 0 of 313 (149 NaN results with other bits: `std` answers the negative
  default NaN for the root of a negative Number and quiets a signalling NaN, `fmath` answers
  the positive default NaN and a NaN operand as it is), `floor`, `ceil` and `trunc` 0 of 313
  each, `fmod` 0 of 436 (10 NaN results with other bits), `rem_euclid` 0 of 436 (12). The lib
  tests read the longest operand of `Nat` from a value passed in, not from a `thread_local!`.
  The test262 counts do not change (`test262 total 53801 8982 7 44812 25943 18867 2`).
- `swell-unicode`, `swell-syntax`, `swell-compiler` and `swell-regexp` are `#![no_std]` with
  `extern crate alloc`: their library code names `core::` and `alloc::` paths and no `std::`
  path (step 1 of `story:core-builds-without-std`). `SyntaxError` and `CompileError` implement
  `core::error::Error`, the same trait as `std::error::Error`. Public names and behaviour do not
  change; unit tests inside the crates still link `std`.
- The gate builds `swell` and every crate it depends on, the six engine crates, for
  `x86_64-unknown-none`, a target without `std` (`task nostd-build`, `taskfiles/nostd.yml`), and
  reads the undefined symbols of their libraries, weak ones included, with `llvm-nm` of the pinned
  toolchain (`task nostd-symbols`): a symbol that is neither a mangled Rust name (legacy names are
  checked down to their hash, v0 names by their form) nor a routine of the compiler runtime (a C
  library's `fmod` or `sqrt`, a C++ library's `_ZNSt9exceptionD2Ev`, an I/O call) fails it, named
  with its object. `rust-toolchain.toml` adds the target and the component `llvm-tools`; the
  toolchain step of `.github/workflows/check.yml` installs both, and `toolchain-check` fails when
  that step lacks a component or target the file names (arrays on one line or several, strings
  in "" or ''), or reads no value from a key that names them. `HOST-2`'s source scan also refuses a
  `std` path in library code and a crate root without `#![no_std]`; its reading of source now
  takes raw byte and C strings (`br"..."`, `cr#"..."#`) as literals and removes the blanks before
  every `::`, and its blanking of `#[cfg(test)]` items ends at the `,` of a variant or arm with a
  comparison or shift in it and at the `)` after a parameter. A test host of the emulated kind in
  the `std` test binary of `swell` (`crates/swell/tests/emulated_host.rs`: a clock that stands
  still, fixed seeds, no system API and no output macro of the `std` prelude) reads `Date.now()`
  twice and gets the fixed time, and two agents of it draw the same `Math.random` values; module
  loading from memory is slice 11 and not delivered. Step 4 of `story:core-builds-without-std`.
- The engine crate lists the methods that reserve room in a `Vec`, `VecDeque` or `String`
  (`with_capacity`, `reserve`, `reserve_exact`, `try_reserve`, `try_reserve_exact`) as
  disallowed in `crates/swell/clippy.toml`. Each of the 58 calls clippy found (54 in the
  engine, 4 in its integration tests) carries an `expect` whose reason names the charge or
  the limit of its buffer, so a new call without one turns the gate red. The crate denies
  `clippy::allow_attributes` and `clippy::allow_attributes_without_reason`, and a source scan
  refuses what those let through: an inner `allow` of the lint, an `allow` or `expect` of a
  group that holds it, an `expect` with a blank reason, and an attribute naming the lint in
  another crate of the workspace (`story:buffer-growth-outside-the-charged-list-is-listed`).

- `Agent::run_builtin` picks one of four functions by the group of the builtin and makes one
  call, where one match held the arms of every group. In the `stack-debug` profile a re-entry
  through a builtin passes at most 736 bytes of dispatch (1,424 with the arms of `WeakMap`
  and `WeakSet` in the one match), and the stack test through builtins needs 12,195,840 of
  its 12,838,560 bytes at depth 1,000. In the release profile 27 of the 31 measured routes
  are larger, by up to 1,184 bytes per re-entry; the largest takes 5,872 of the 14,336 bytes
  promised per re-entry (measured on x86_64 with rustc 1.99.1).
  `limits_scripts::min_stack_bytes_is_enough_through_builtins` runs a fifth route, `Math.max`
  through a `valueOf`.

### Fixed

- `JSON.parse` frees the room of its stack of open values when the parse ends, before the
  reviver is called; before, that room stayed allocated, uncharged, while the reviver ran
  (found by the review of `story:buffer-growth-outside-the-charged-list-is-listed`).
- The gate no longer overwrites `website/data/engines.json`: the selftest
  `site-data-check-refuses-a-percentage-rounded-up-to-100` wrote its planted counts there on
  every run. Each selftest of `site-data` now writes all its outputs under its own directory,
  and `gate-selftest` fails, naming the files, when its scripts change a tracked file
  (`story:gate-selftests-write-no-tracked-file`).

### Host interface

- `HOST-2` (changed, slice 1) also states that each engine crate declares `#![no_std]` in its
  crate root and names no `std` path in its library code (an item under `#[cfg(test)]` is test
  code); its source scan, `swell::source_scan::host_2_no_ambient_authority_in_engine_crates`,
  refuses both. No public item of `value.rs` or of `host/*.rs` is added or changed.
- `HOST-26`, `HOST-27`, `HOST-144`, `HOST-147`, `HOST-212` and `HOST-213` (slice 4) have tests
  (`swell::math_random::*`): two agents with the same `random_seed` give the same 1,000
  values; two realms of one agent differ; `random_seed` changes the stream and `hash_key` does
  not; two child processes print the same 1,000 values and the key order of an object of
  10,000 keys; 100,000 values are in [0, 1) with positive sign and 53 bits; a realm's stream
  is its creation index, not the order of calls.
- `HOST-25` and `HOST-214` (slice 9) have tests:
  `swell::determinism::host_25_key_order_does_not_depend_on_hash_key` and
  `swell::determinism::host_214_two_processes_print_the_same_map_order`. A `Map` and a `Set`
  give their keys in insertion order under every `hash_key`, and two child processes print
  the same order (`story:map-and-set`).
- `HOST-222` (slice 9): an override of a host object whose `Ok` or `Abrupt::Throw` value
  holds a stale handle or a handle of another agent ends the run with
  `Terminated(EngineFault)`. It was the start gate of `story:host-object-overrides`, which
  holds it with a test.
- `Overrides`: the override table of a host object, generic over the host type
  (`Overrides<H>`), with one optional function per internal method:
  `Overrides::get_prototype_of`, `Overrides::set_prototype_of`,
  `Overrides::is_extensible`, `Overrides::prevent_extensions`, `Overrides::get_own_property`,
  `Overrides::define_own_property`, `Overrides::has_property`, `Overrides::get`,
  `Overrides::set`, `Overrides::delete` and `Overrides::own_property_keys`, as
  `docs/design/host-interface.md` section 15 states them.
  - Each override given runs in place of the ordinary method; each one left `None` runs the
    ordinary one (HOST-114).
  - An override call is one re-entry and spends a step.
  - The handles of its answer are checked first (HOST-222). Then the answer is checked
    against the invariants of the essential internal methods, read as facts of the object's
    own earlier answers.
  - An answer of the object's record, where the table leaves an override out, is checked
    and recorded the same way: an answer is the object's, whoever gives it.
  - A `has_property` false records nothing and faults only for a non-configurable property
    (reported by `get_own_property`, or held by the record), extensible or not.
  - A stale or foreign handle, a descriptor that is not complete (HOST-113), or a broken
    invariant (HOST-112) ends the run in `Terminated(EngineFault)` and faults the agent.
    Nothing is thrown to the script.
  - A global object whose `define_own_property` refuses a property of clause 19 makes
    `create_realm` answer `Throw` of a `TypeError` (HOST-41).
- `HostObjectSpec` changed: it takes the host type, `HostObjectSpec<H>`, and has the field
  `HostObjectSpec::overrides`, of type `Option<&'static Overrides<H>>`.
- `new_host_object` takes `HostObjectSpec<H>`.
- **Breaking**: `HostObjectSpec`. A host that names the type or writes a literal stops
  compiling once. A literal names `overrides` (`overrides: None` for no table), and a type
  position names the host type (`HostObjectSpec<MyHost>`).
- Requirements delivered: HOST-41, HOST-112, HOST-113, HOST-114, HOST-222.

## [0.12.0] - 2026-11-25

swell has a shell binary, `swell`, and a release ships it for Linux, macOS and Windows. The
site compares swell with other engines, and an interrupt set by a host function is read after
every call from Rust. The test262 counts do not change: 25,943 of the 44,812 selected test262
cases pass, as in 0.11.0 (`test262 total 53801 8982 7 44812 25943 18867 2`).

### Added

- The shell binary `swell`, package `swell-cli` in `crates/cli`: `swell [flags] <file.js>` or
  `swell [flags] -e <source>` runs one script and then its promise jobs. The realm has `print`,
  and `console.log` as the same function. `Date.now()` reads the system clock. There is one flag
  per field of `Limits` (`--steps`, `--memory`, `--depth`, `--reentry`, `--nesting`,
  `--string-length`, `--bigint-bits`) and `--timeout <ms>`, which interrupts the run. The exit
  status gives the outcome: 0 normal, 1 uncaught throw, 2 syntax error, 3 step budget, 4 memory
  budget, 5 depth limit, 6 interrupted, 64 usage error, 70 engine fault. `swell --version`
  prints the version. The binary is std only and has `#![forbid(unsafe_code)]`
  (`story:cli-runs-a-script-file`).
- Releases ship the binary: a pushed tag runs `.github/workflows/release-build.yml`, which
  builds, smoke-tests and packages `swell` for `x86_64-unknown-linux-gnu`,
  `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin` (`.tar.gz`) and
  `x86_64-pc-windows-msvc` (`.zip`). The release has the five archives and `SHA256SUMS` as
  assets (`docs/releasing.md`; `story:releases-ship-the-shell-for-three-systems`). The macOS
  and Windows binaries are not signed; the release's notes say how to check and run them.
- The test262 sources digest (`task test262-digest`) now covers only the packages of the
  runner: `swell-test262` and what it depends on in `Cargo.lock`. A change to `crates/cli` or
  `crates/unicode-gen` no longer requires a full test262 run.
- The site's compare page, `/compare`: 2 to 4 engine configurations side by side, rows grouped
  by the KPIs K1 to K8 and the facts, the selection in the URL (`?e=swell,v8-jitless,quickjs,boa`),
  the best of the picked cells marked among values of one source, "not measured" with its
  reason, and a source link in every cell. JIT configurations are offered behind a toggle and
  shown as "JIT reference". The page `/docs/kpis` describes each KPI: definition, unit,
  direction, gate or compare, how it is measured and where each engine's value comes from.
- `bench/engines.jsonl`: a catalog of 35 ECMAScript engines, a source URL per field and
  "unknown" where no source says, the default set of eight marked, and the configurations the
  page compares.
- `task bench-import-zoo` (manual, network): javascript-zoo-data's test262 results at test262
  `4a1e962` for the 35 configurations it has, counted per top directory into
  `bench/published/zoo-test262-4a1e962.tsv` with the data commit, the sha256 of each fetched
  file and the fetch date, its MIT notice, and `bench/published/SHA256SUMS`.
- `task site-data` writes `website/data/engines.json` beside `status.json`, from the catalog,
  the published tables, result files of format `swell-bench/1` (none yet) and swell's own
  committed files; every value carries its source, date, revision and method.
  `site-data-check` holds it, and refuses a published table that differs from its sha256.
- Two designs, of which nothing is built: `docs/design/quality-and-comparison.md`, what makes
  swell good and how it is compared with other engines, and `docs/design/native-code.md`, a
  path that compiles JavaScript ahead of time through codespec's `cir/1` to WebAssembly or
  x86_64, with a JIT later.

### Fixed

- The interrupt flag is read after every call from Rust into a host function, not only after
  a call or a construction: a host function reached by a property read or write (a getter, a
  setter), a conversion (`valueOf`, `toString`), an iterator's `next` or a promise job (a
  thenable's `then`, a reaction handler) that sets the flag ends the run or the microtask
  checkpoint in `Terminated(Interrupted)` when it returns or throws, also when it is the
  script's last operation or the checkpoint's last job, and the thrown value reaches no
  `catch`.
- An embedder that calls a host function with `cx.call` and that function sets the flag now
  gets `Err(Abrupt::Terminated(Termination::Interrupted))` from that call, where it got the
  function's `Ok` value before and the termination only at the next call.

### Host interface

- `HOST-221` (new, slice 1) has a test: an interrupt set by a host getter, setter, `valueOf`,
  iterator `next`, thenable `then` or reaction handler that then throws or returns ends the
  run or the checkpoint, with the flag clear after.

## [0.11.0] - 2026-11-25

`Date`, `Math` and `Number` work: nine more stories of the fourth slice give `Date` its time
zone, setters and text, `Math` all its functions and `Number` its formatting and parsing.
25,943 of the 44,812 selected test262 cases pass (24,838 in 0.10.0); the fourth slice passes
1,533 of its 1,612 cases (`test262-slice 04 1612 3184 0 0 1612 1533 79 0`, 564 in 0.10.0), and
the line reads `test262 total 53801 8982 7 44812 25943 18867 2`. One case of the eighteenth
slice, not delivered, no longer passes: `annexB/built-ins/Date/prototype/toGMTString/value.js`
held while `toGMTString` and `toUTCString` were both undefined; `toUTCString` exists now.

### Added

- `Date` reads the host's time zone, a story of the fourth slice: the constructor with two or
  more components, `Date.UTC`, and the 17 getters of `Date.prototype` in local time and in UTC.
  Local time comes from the host's time zone hooks; every hook call is charged one step before
  it is made, an offset out of range is clamped, and a host that has no instant for a local
  time makes the date invalid after a bounded search.
- `Date` setters, a story of the fourth slice: the 14 setters of `Date.prototype`, local and
  UTC, with the conversions in the order the clauses give. They no longer throw a `TypeError`.
- `Date` text, a story of the fourth slice: `Date.parse`, a String argument to the
  constructor, `Date()` called without `new`, and `toString`, `toDateString`, `toTimeString`,
  `toUTCString`, `toISOString`, `toJSON`, `toLocaleString`, `toLocaleDateString`,
  `toLocaleTimeString` and `[Symbol.toPrimitive]`. A String is read in place: `Date.parse`
  charges one step per code unit past 32 before it reads a unit, so a long String ends at the
  step budget without a copy.
- `Math` constants and exact functions, a story of the fourth slice: the eight constants and
  `abs`, `ceil`, `clz32`, `floor`, `fround`, `imul`, `min`, `round`, `sign`, `sqrt` and `trunc`
  beside `max`. Each is fixed by IEEE 754 or gives the same result on every platform.
- `Math` approximated functions, a story of the fourth slice: the 22 functions IEEE 754 does
  not fix (`acos` to `tanh`, `atan2`, `hypot`, `pow` and the others). A script prints the same
  bytes for them on every platform. `Math.hypot` charges one step per argument and its list to
  the memory budget before it reads it.
- `Math.sumPrecise`, a story of the fourth slice: the exact sum of an iterable of Numbers,
  computed in a fixed accumulator that does not grow with the count of items. It charges one
  step per item, closes the iterator once when an item is not a Number, and checks the count
  against 2^53 - 1 before it checks the type of an item.
- `Number.prototype` formatting, a story of the fourth slice: `toFixed`, `toExponential`,
  `toPrecision` and `toLocaleString`, with exact digits. A call charges its temporary to the
  memory budget before it computes and credits it after. `toLocaleString` gives the text of
  `toString()`.
- `Number.isFinite`, `Number.isInteger`, `Number.isNaN` and `Number.isSafeInteger`, a story of
  the fourth slice.
- `parseInt` and `parseFloat`, a story of the fourth slice: both are globals, and
  `Number.parseInt` and `Number.parseFloat` are the same objects. A String is read in place
  after one step per code unit past 32 is charged, and the one buffer of script-controlled
  size is charged to the memory budget before the call and credited when it returns.

### Changed

- The placeholders of 0.9.0 for `Date` are gone: a String argument or two or more arguments
  to the constructor no longer give an invalid date, `Date()` without `new` no longer answers
  `"Invalid Date"`, and `Date.UTC`, `Date.parse` and the other setters no longer throw a
  `TypeError`. Each does what its clause says.
- The tests of `swell` hold at any size of the realm: the test host pads the realm by
  `SWELL_TEST_REALM_FUNCTIONS` host functions, and tests that compared against a room fitted
  to one size now read their room from agents of their own. `task test-realm-sizes` runs them
  in both modes at each size of `REALM_SIZES` and is a step of the lane `slow`. No file under
  `crates/swell/src` changes with this story.

### Host interface

- `HOST-211` (changed, slice 4): its last clause states the budget by the step charged before
  the call of `system_time_zone_identifier`. With that budget the call ends after that hook
  and before `get_named_time_zone_offset_nanoseconds`, and one step lower it ends before
  either hook. No public item of `value.rs` or of `host/*.rs` is added or changed.
- `HOST-127`, `HOST-128`, `HOST-135`, `HOST-209`, `HOST-210` and `HOST-211` have a test: the
  provided zone hooks, an offset of one hour, the bytes of `Math` on every platform, an offset
  out of range, a host with no instant for any local time, and the step charged to every zone
  hook call.

## [0.10.0] - 2026-11-25

The host reads the length of a String without a copy, and a String or a Symbol that host data
lists in its trace is held to survive a collection. Nothing else changes: 24,838 of the 44,812
selected test262 cases pass, as in 0.9.1 (`test262 total 53801 8982 7 44812 24838 19972 2`).

### Host interface

- `HOST-218` (new, slice 1) has a test and `HOST-219` (new, slice 12) is pending: a String
  or a Symbol that host data lists in its `trace` with `Tracer::value` survives `collect`, as
  an object does, and a BigInt will once slice 12 makes one. The design says a listed handle
  keeps a value alive, not only an object.
- `to_utf16_len` (new, on `Cx` and `HookCx`) answers the length of a String in code units
  without copying it, held by `HOST-220` (new, slice 1, with a test). Additive, not breaking.

## [0.9.1] - 2026-11-25

An interrupt set while a host function runs now ends the run when that function throws, as it
did when the function returned. Nothing else changes: 24,838 of the 44,812 selected test262
cases pass, as in 0.9.0 (`test262 total 53801 8982 7 44812 24838 19972 2`).

### Fixed

- The interrupt flag is read after a builtin or host function called by a call or a
  construction, with or without spread arguments, answers by a throw, as after a return: a
  set flag ends the run in `Terminated(Interrupted)` and the thrown value reaches no `catch`.

### Host interface

- `HOST-216` and `HOST-217` (new, slice 1) have a test: an interrupt set by a host function
  that then throws ends the run, and a host function that answers
  `Terminated(Interrupted)` ends it with the agent alive.

## [0.9.0] - 2026-11-24

Promises and `async` functions run: the tenth delivery slice is delivered whole, with async
generators, `for await` and `Array.fromAsync`, and the fifth and the seventh are closed, with
the flag `v` of regular expressions and generator methods. `Date` exists over the host's
clock, and a buffer the engine holds while it calls into script is charged to the memory
budget. 24,838 of the 44,812 selected test262 cases pass (20,229 in 0.8.0).

### Added

- `Date`, a story of the fourth slice: the constructor, `Date.now()`, and `getTime`, `setTime`
  and `valueOf` of `Date.prototype`. `new Date()` and `Date.now()` ask the host's clock hook
  once, after one step, floor the answer to milliseconds and clamp an answer out of range; the
  engine reads no clock of its own. `Date.UTC`, `Date.parse` and the other 14 setters exist
  with their name and length and throw a `TypeError` until their stories are built. Until
  then a String argument, or two or more arguments, to the constructor give an invalid date,
  and `Date()` without `new` answers `"Invalid Date"`. 330 more test262 cases pass with it.
- Slice 10, promises and `async` functions, is delivered: all 3,759 of its test262 cases pass
  in all 7,258 runs. At its close 141 cases that need a later slice move there, each with its
  evidence (116 to slice 8 for `class`, `super` or `new.target`, 20 to slice 12 for a BigInt
  literal, 4 to slice 9 for `Proxy` or `Map`, 1 to slice 13 for `ArrayBuffer`), and 21 cases
  of slices 18 and 19 that pass with what slice 10 built and use no later part count in
  slice 10.
- Slice 7, generators, is delivered: all 1,472 of its test262 cases pass in all 2,801 runs.
  At its close 40 cases that need `class` or `super` move to slice 8, each with its
  evidence, and 21 cases that failed only on generators count in slice 7 instead of slices
  18 and 19.
- `Array.fromAsync`, a story of the tenth slice: it reads an async iterable, a sync iterable
  or an array-like, awaits each value and the result of a mapper, and returns a promise of
  the array. It is the first built-in async function: its `await` goes through the hooks
  an `await` of a script function uses, a continuation spends one step before it runs, and
  a termination leaves its promise pending. An array-like without end stops at the step
  budget or at the memory budget.
- `yield*` in an async generator, a story of the tenth slice: it delegates to an async
  iterator, or to a sync iterable through the Async-from-Sync Iterator, and awaits the
  result of each call of the inner iterator's `next`, `throw` or `return`. A received
  `return` is awaited before it goes to the inner iterator. Where the inner iterator has
  no `throw` method, it is closed and the delegation throws a `TypeError`. A `next` through
  n levels of delegation costs 15 steps per level.
- `%AsyncGeneratorFunction%`, the constructor that makes an async generator function from
  source text, a story of the tenth slice. It is not a global: a script reaches it as the
  `constructor` of the prototype of an async generator function. The hook of compile
  strings is asked first, and the parse and the compile are charged as those of `Function`
  are.
- `async *m() {}` in an object literal, a story of the tenth slice: the method is an async
  generator with its object as home object, also under a computed name, with the early
  errors of its form.
- Async generators, a story of the tenth slice: `async function*` as a declaration and as
  an expression, `%AsyncGeneratorPrototype%` with `next`, `return` and `throw`, and the
  queue of requests behind them. Each call returns a promise of an iterator result; calls
  made while the generator runs or waits are queued and answered in their order. `return`
  with a value awaits it. A request is charged to the memory budget when it is queued, and
  the queue is credited when the generator is freed; a script that queues requests without
  end stops at the memory budget, and a generator that never ends stops at the step
  budget.
- `for await (x of y)` in an `async` function, a story of the tenth slice, with
  `%AsyncIteratorPrototype%`, `Symbol.asyncIterator` and the Async-from-Sync Iterator that
  wraps a sync iterable. Each result of `next` is awaited, and the iterator is closed on
  every exit of the loop. A loop over an iterator that never ends stops at the step budget;
  a wrapped iterator is charged to the memory budget until it is freed.
- The statics of `Promise`, a story of the tenth slice: `Promise.all`, `Promise.allSettled`,
  `Promise.any`, `Promise.race`, `Promise.try` and `Promise.withResolvers`, with
  `Promise.prototype.finally` and `AggregateError`. `Promise.all` over an iterator that
  never ends stops at the step budget or the memory budget and keeps nothing; the values
  of promises that never settle are charged to the memory budget and credited when the
  promises are collected.
- The grammar of async generators, a story of the tenth slice. `async function*`, as a
  declaration and as an expression, is parsed whole, and its grammar errors and early
  errors are reported.
- `%AsyncFunction%`, the constructor of `async` functions from source text, a story of the
  tenth slice. It is reached through the `constructor` of the prototype of an `async`
  function and is no property of the global object. Called or constructed, it takes the
  path of `Function`: the host's hook for the compilation of strings decides, the limits
  of nesting and of re-entry hold, and the parse is charged to the memory budget and reads
  the interrupt.
- `async` methods of object literals run, a story of the tenth slice: `{ async m() {} }`,
  with a computed name too. A call returns a promise and the body awaits as the body of an
  `async` function does; the method has no `prototype` and cannot be constructed. `super`
  in an `async` method is refused as in a plain method.
- Slice 5, regular expressions, is delivered: all 2,724 of its test262 cases pass in all
  5,441 runs. At its close 18 cases that need `Date`, `Math`, `parseInt`, the operator
  `**`, `class` or `Map` move to slices 4, 8 and 9, each with its evidence, and 18 cases of
  the pattern grammar of Annex B.1.2 count in slice 5 instead of slice 18.
- The engine calls the host's rejection tracker, a story of the tenth slice:
  `host_promise_rejection_tracker` with `Reject` when a promise is rejected and has no
  handler, and with `Handle` when a handler is added to such a promise later. A host reads
  the state of a promise with its result, and whether it is handled, without running code,
  calling a hook or spending a step, and marks a promise as handled; a promise it marked
  causes no call of the tracker.
- `async` functions run, the third story of the tenth slice: declarations, expressions and
  arrow functions. A call makes the function's promise and runs the body to its first
  `await`; an `await` suspends the activation and a promise job resumes it at a checkpoint.
  A return resolves the promise and an uncaught throw rejects it. A suspended activation
  is charged to the memory budget while it waits and credited when its promise settles or
  it is collected. An `await` in a loop that never ends ends the checkpoint at the step
  budget; a termination in a resumed activation leaves its promise pending and the jobs
  behind it queued. `%AsyncFunction.prototype%` exists.
- `Promise`, the second story of the tenth slice: the constructor with its resolving
  functions, `Promise.prototype.then` and `catch`, `Promise.resolve`, `Promise.reject` and
  the getter of `Promise[Symbol.species]`. A reaction and the job of a thenable are entries
  of the agent's queue and run at a checkpoint in the order they were queued. The engine
  announces a promise job to the host before it can run, and calls
  `host_prepare_to_run_job` with the job's realm, `host_call_job_callback` and
  `host_clean_up_after_job` with what the job threw. A job that queues itself ends the
  checkpoint at the step budget; a termination in a job leaves its promise pending and the
  jobs behind it queued. A reaction of a pending promise and a queued job are charged to
  the memory budget until the promise settles or the job is taken off.
- Regular expressions with the flag `v`: classes with union, intersection (`&&`),
  subtraction (`--`) and nested classes, `\q{...}` with its strings, and the seven
  properties of strings (`\p{RGI_Emoji}` and the six it is made of) from a sixteenth
  generated table of the pinned Unicode data. With `i` the set is folded before a
  complement is taken. A class costs one step per operand, and the lengths of its strings
  besides. No construct of the fifth slice is refused any more; five patterns that the
  grammar of `v` forbids are early errors.
- Generator methods of object literals, `{ *g() {} }`, with their early errors, the fifth
  story of the seventh slice. A `yield` in the computed name of one belongs to the generator
  around the literal.
- The parser reads `async` function declarations, expressions and arrow functions with
  their early errors, and `await` where the grammar's [Await] is set. A refusal that names
  a later slice comes after the grammar errors and early errors of the source, which now
  come first.
- The microtask queue of the agent and its checkpoint, the first story of the tenth slice. A
  host queues a microtask with the realm it runs in; a checkpoint runs every entry in the
  order it was queued, under the step budget of one entry, and spends one step for each
  entry it takes off. A checkpoint asked for inside a checkpoint returns at once. A
  termination leaves the entries behind it queued.
- The test262 runner runs a case with the flag `async`: it performs a checkpoint after the
  script and decides the case by the first line `Test262:AsyncTestComplete` or
  `Test262:AsyncTestFailure:` the script prints.

### Changed

- A buffer whose size a script controls and that the engine holds while it calls into script
  (the arguments of `apply`, `call` and `new`, the lists of `JSON.parse`, `JSON.stringify` and
  `flat`, the text a replacer builds, a copy of a String's code units) is charged to
  `memory_bytes` before it is made and credited when it is dropped, or is not made and the
  String is read in place. A script that passes the budget this way ends in
  `Terminated(MemoryBudget)`.
- The microtask queue grows through the memory budget: a job queued by script past the room
  ends that entry in `Terminated(MemoryBudget)`.

### Host interface

- New: `Agent::queue_microtask`, `Agent::perform_microtask_checkpoint`,
  `Cx::queue_microtask`, `Cx::perform_microtask_checkpoint`, of the agent
  `pending_microtasks` and `clear_microtasks`, the type `Microtask` and the struct
  `CheckpointReport` with its fields `CheckpointReport::promise_jobs` and
  `CheckpointReport::host_microtasks`.
- `HOST-77`, `HOST-79`, `HOST-81`, `HOST-149`, `HOST-192`, `HOST-193`, `HOST-196` and
  `HOST-197` have a test.
- New: the struct `PromiseCapability` with its fields `PromiseCapability::promise`,
  `PromiseCapability::resolve` and `PromiseCapability::reject`, and of the context
  `new_promise_capability`, which hands a host a pending promise with the two functions
  that settle it.
- `HOST-78`, `HOST-80`, `HOST-82`, `HOST-83`, `HOST-84`, `HOST-148`, `HOST-198`, `HOST-199`
  and `HOST-200` have a test.
- `HOST-194` and `HOST-195` have a test. No name of the host interface changes with
  `async` functions.
- New: the enum `PromiseState` with its variants `PromiseState::Pending`,
  `PromiseState::Fulfilled` and `PromiseState::Rejected`, the two settled ones with the
  result as a new root; `Agent::promise_state`, `Agent::promise_is_handled` and
  `Agent::mark_promise_as_handled`; and of the contexts `promise_state`,
  `promise_is_handled` and `mark_promise_as_handled`. A root that holds no promise
  answers `WrongKind`.
- `HOST-46`, `HOST-100`, `HOST-101` and `HOST-121` have a test.
- Breaking: `ErrorKind` is `#[non_exhaustive]`. A `match` over it in a host needs a wildcard
  arm; a host that has one compiles as before. The list of error kinds follows the
  standard, which grows.
- New: `ErrorKind::AggregateError`. `new_error` with it makes an `AggregateError` with no
  `errors`, and `error_summary` of an `AggregateError` answers the kind and the message
  without reading `errors`. `ErrorKind::ALL` keeps its seven entries.
- `HOST-201` and `HOST-202` have a test.
- New requirements of the clock and the time zones, pending for slice 4 (no code reads them
  yet): `HOST-209`, an offset a host answers outside ±86,399,999,999,999 ns is clamped to
  that range; `HOST-210`, a host that answers no instant for every local time gets NaN
  after 31 hook calls and no exception; `HOST-211`, every call of a clock or time zone hook
  is charged one step before it is made.
- New requirements of `Math.random`, pending for slice 4 (no code reads them yet):
  `HOST-212`, each value is a Number in [0, 1) with positive sign on the grid of 2^-53;
  `HOST-213`, the sequence of a realm follows from the agent's `random_seed` and the index
  of the realm's creation, not from the order in which the realms' functions are called.
- New: `WellKnownSymbol::AsyncIterator`, the thirteenth well-known symbol a host names. A
  host object with a method under it is the iterable of a `for await`.
- `HOST-203` has a test.
- `HOST-204`, `HOST-205` and `HOST-206` have a test.
- `HOST-207` and `HOST-208` have a test. No requirement is pending for the tenth slice.
- `HOST-147` no longer names a `Map`: two processes with the same seeds print the same
  `Math.random()` values and the same key order of an object. The key order of a `Map` is the
  new requirement `HOST-214`, pending for the ninth slice.
- The model of the interface (`spec/`) states the microtask checkpoint with its answers at the
  step budget and at an interrupt, and a job that will not run to its end (`Abandoned`). Two
  scenarios are new: a promise job runs at the next checkpoint and not before, and a rejection
  without a handler is reported to the tracker and reported again when a handler is added.
  18 of the model's 60 scenarios pass on the engine (14 of 58 before); the other 42 name the
  later slice or the reason they wait for. The model has 53 places marked as not expressible
  (51 before).
- Breaking: `HostError::MemoryBudget` is a new variant, and `HostError` is not
  `#[non_exhaustive]`: a `match` over it in a host needs an arm for the variant or a wildcard
  arm. `queue_microtask` answers it when the memory budget refuses the growth of the microtask
  queue, with nothing queued.
- `HOST-215` has a test: a buffer the engine holds while it calls into script is charged to
  `memory_bytes` before it is made, or not made.

### Limits of this release

- Not in the language yet: classes with `super` and private names, modules with dynamic
  `import` and `import.meta`, the operator `**`, and the web-compatibility semantics of
  function declarations in blocks (Annex B.3.2). A script that uses a construct of a later
  slice is refused with a `SyntaxError` that names the delivery slice that brings it.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices, what each brings and the order they are delivered in.
- The builtins end at the core library, what the sixth slice adds to it, `RegExp` of the
  fifth slice, the generator objects of the seventh and, of the tenth, `Promise`, async
  functions and async generators. Of the fourth slice, which is in work, `Date` exists with
  its constructor over a time value, `Date.now()`, `getTime`, `setTime` and `valueOf`;
  `Date.UTC`, `Date.parse` and the other 14 setters throw a `TypeError`, and the getters of
  the fields and the methods that write a date as text do not exist. `parseInt`,
  `parseFloat` and all of `Math` but `max` do not exist, nor do `Map`, `Set`, `Reflect`,
  `Proxy`, `BigInt` and typed arrays.
- test262: 24,838 of the 44,812 cases the gate's digest stands for pass, 55.4 %. The
  line reads `test262 total 53801 8982 7 44812 24838 19972 2`: 53,801 cases exist, 8,982
  are excluded by recorded decision, 7 do not apply, 44,812 run, 24,838 pass, 19,972
  fail, two end at the memory budget. The first, second, fifth, sixth, seventh and tenth
  delivery slices pass all their cases in all their runs, and the third all but one
  (`test262-slice 01 31 62 0 0 31 31 0 0`,
  `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 05 2724 5441 0 0 2724 2724 0 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`,
  `test262-slice 07 1472 2801 0 0 1472 1472 0 0`,
  `test262-slice 10 3759 7258 0 0 3759 3759 0 0`). Of the fourth, which is in work, 564 of
  1,612 cases pass
  (`test262-slice 04 1612 3184 0 0 1612 564 1048 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 148 of its 215 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the seven delivered slices, and 12 are pending for
  the fourth. 18 of the 60 scenarios of the interface's model pass on the
  engine, and the other 42 name what they wait for. The interface will change
  before 1.0.
- The engine's own tests: `17 test binaries: 1536 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 1167 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 964 ignored`; the
  964 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.9.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.8.0] - 2026-11-24

Generator functions run, and a regular expression takes the flag `i`, Unicode property escapes
and group names outside ASCII: four of the six stories of the seventh delivery slice and
thirteen of the fifteen of the fifth are delivered. 20,229 of the 44,812 selected test262
cases pass (18,155 in 0.7.0).

### Added

- `String.prototype.matchAll`, `RegExp.prototype[Symbol.matchAll]` with its iterator, and
  `RegExp.escape`. Each match the iterator gives spends 9 steps beside the steps of the
  matcher; `RegExp.escape` spends 2 steps per code unit it copies and 3 per code unit it
  escapes.
- Unicode property escapes in a regular expression with the flag `u`: `\p{...}` and
  `\P{...}` with a General_Category value, `Script=` and `Script_Extensions=` with a script,
  and the binary properties, each by its name or an alias as written. A name or value the
  standard does not list is the `SyntaxError` "invalid property name". A class spends one
  step per property escape it holds, and a test of one property is at most seven binary
  searches of a table, so a step of the matcher still stands for a bounded amount of work.
- Case-insensitive matching in a regular expression: the flag `i` and the modifiers
  `(?i:...)` and `(?-i:...)`. Under `u` or `v` a character matches by its simple case
  folding, under neither by the uppercase mapping of its code unit. A character, each code
  unit a backreference compares and a class cost at most four lookups more than without
  `i`; no pattern costs more to compile for it.
- Identifiers with code points outside ASCII (`café`, `π`, `\u{e9}`), by `ID_Start` and
  `ID_Continue` of Unicode 17, and names of regular expression groups written the same way
  (`(?<é>a)`). A code point outside ASCII costs one binary search of a table; ASCII source
  costs what it did.
- Generator functions run: `function*` declarations and expressions, `yield`, and generator
  objects with `next`, `return` and `throw`, usable in `for`-`of`, spread and array
  patterns. A suspended generator holds its frame in its object: the frame's bytes stay
  charged to the memory budget until the generator completes or is collected, a suspended
  generator takes none of the depth bound of calls, and the steps it runs are charged to
  the entry that resumes it. Generator methods of object literals are still refused.
- `yield*` delegates to an iterator. When the iterator is a generator object with the
  `next`, `throw` and `return` of `%GeneratorPrototype%`, the delegating frame resumes the
  inner frame itself: a chain of n delegating generators holds n frames, counted against
  the depth bound of frames, and takes no re-entry per level; a `next` through 101 levels
  spends 500 steps more than through one, 5 per level. Any other iterator is called as a
  step of `for`-`of` calls it.
- `%GeneratorFunction%`, the constructor a script reaches as
  `Object.getPrototypeOf(function* () {}).constructor`: called or constructed, it makes a
  generator function from source text by the steps of the `Function` constructor, with the
  same host hook (`host_ensure_can_compile_strings`, compilation type `Function`) and the
  same charges for the source text and its parses.

### Changed

- A store to an Array at a Number index (`a[i] = v`) no longer makes the text of the index
  and a property key from it. A loop of 1,114,112 such stores took 2.313 CPU seconds in a
  release build and takes 0.830, 35.9 %; the steps a script is charged are the same. The
  full test262 run, which the gate's digest stands for, took 598 s in one job with this
  change (1,599 s before).
- The full test262 run can be made in parts, each a pool job, and counted from their files to
  the lines the whole run prints: `task test262-part TEST262_PART=<k>/<n>` and
  `TEST262_FROM_PARTS` of `task test262`, `test262-expected` and `slow-test262`.
- The gate's test steps cost less on few cores: the tests with a collection at every safe
  point are built in a profile of their own at opt-level 1, and the release build of the
  test262 runner is made before the clock. The gate refuses a run in which a test step
  builds, and `check-runner` bounds the CPU seconds of the test steps on a machine whose
  core count has an entry in its table. The budget of 120 seconds and the steps it is
  counted over are unchanged. 66 planted defects, from 65.

### Host interface

- `HOST-191` (changed): of the parts of a regular expression it names as refused and
  counted, the flag `v` is left; the flag `i`, a property escape and a group name outside
  ASCII run.
- `HOST-192`, `HOST-193`, `HOST-194`, `HOST-195`, `HOST-196`, `HOST-197`, `HOST-198`,
  `HOST-199` and `HOST-200` (new, pending for the tenth slice, held by no test yet): the
  microtask checkpoint as one outermost entry under one step budget, the realm of a host
  microtask, what `last_entry_steps()` reads after a checkpoint, where the interrupt ends
  one, what stays queued after a termination, a call that empties the queue, and the hooks
  an `await` calls. No function they name exists in this release.
- `HOST-201`, `HOST-202`, `HOST-203`, `HOST-204`, `HOST-205`, `HOST-206`, `HOST-207` and
  `HOST-208` (new, pending for the tenth slice, held by no test yet): the kind of an
  AggregateError and how a host reads its `errors`, `Symbol.asyncIterator` as a well-known
  symbol, the request queue of an async generator under the memory budget and what a
  termination leaves of it, and what a built-in async function spends and leaves. `HOST-201`
  says that `ErrorKind` gets the variant `AggregateError` and `#[non_exhaustive]` in the
  release that brings `Promise.any`: a `match` on `ErrorKind` without a wildcard arm will
  not compile from that release on.

### Limits of this release

- Not in the language yet: in a regular expression the flag `v` with its class sets and
  properties of strings, refused with a `SyntaxError` the agent counts (`HOST-191`);
  generator methods of object literals; the operator `**`; and of what came
  after ES5 everything but the sixth slice and generator functions: template literals with
  and without a tag, `let` and `const`, arrow functions, default and rest parameters,
  object and array destructuring, spread, computed property names, `for`-`of` and function
  declarations in blocks (without the web-compatibility semantics of Annex B.3.2). Classes,
  promises, async functions and modules are refused with a `SyntaxError` that names the
  delivery slice that brings them. [`docs/design/engine.md`](docs/design/engine.md),
  section 12, lists the nineteen delivery slices, what each brings and the order they are
  delivered in.
- The builtins end at the core library, what the sixth slice adds to it, of the fifth slice
  `RegExp` with `exec`, `test`, its methods keyed by a well-known symbol and
  `RegExp.escape`, and of the seventh the generator objects with `%GeneratorFunction%`.
  `Map`, `Set`, `Promise`, `Reflect`, `Proxy`, `Date`, `parseInt`, `parseFloat` and all of
  `Math` but `max` do not exist.
- test262: 20,229 of the 44,812 cases the gate's digest stands for pass, 45.1 %. The
  line reads `test262 total 53801 8982 7 44812 20229 24581 2`: 53,801 cases exist, 8,982 are excluded by recorded decision, 7
  do not apply, 44,812 run, 20,229 pass, 24,581 fail, two end at the memory budget. The
  first, second and sixth delivery slices pass all their cases in all their runs, and the
  third all but one (`test262-slice 01 31 62 0 0 31 31 0 0`,
  `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`). Of the fifth, which is in work, 2,523 of
  2,724 cases pass (`test262-slice 05 2724 5441 0 0 2724 2523 201 0`), and of the seventh, in work too, 1,168 of 1,491
  (`test262-slice 07 1491 2841 0 0 1491 1168 323 0`). A refusal never counts as a pass, also when the script catches it.
- The host interface: 114 of its 208 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the four delivered slices, for the fifth or for the
  seventh, and 31 are pending for the tenth. 14 of the 58 scenarios of the interface's
  model pass on the engine, and the other 44 name what they wait for. The interface will
  change before 1.0.
- The engine's own tests: `17 test binaries: 1215 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 934 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 758 ignored`; the
  758 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.8.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.7.0] - 2026-11-23

`String.prototype.match` and `search` exist, and `replace`, `replaceAll` and `split` take a
regular expression: the methods of `RegExp.prototype` keyed by `Symbol.match`,
`Symbol.replace`, `Symbol.search` and `Symbol.split` are delivered, the ninth of the fifteen
stories of the fifth delivery slice.

### Added

- `String.prototype.match` and `search`, and the methods of `RegExp.prototype` keyed by
  `Symbol.match`, `Symbol.replace`, `Symbol.search` and `Symbol.split`: `replace`,
  `replaceAll` and `split` of `String` take a regular expression, with the replacement
  patterns of named and numbered groups, a function as the replacement, the flags `g` and
  `y`, and a species constructor for the splitter. A regular expression method spends, beside
  the steps of its matches, 11 steps per match of a global `replace` and 7 per match of a
  global `match` or per piece of `split`. 18,155 of the 44,812 selected test262 cases pass
  (17,775 in 0.6.0), 1,667 of the 2,724 of the fifth slice (1,327).

### Host interface

- None.

### Limits of this release

- Not in the language yet: in a regular expression the flags `i` and `v`, a property escape
  and a group name outside ASCII, each refused with a `SyntaxError` the agent counts
  (`HOST-191`); identifiers outside ASCII; the operator `**`; and of what came after ES5
  everything but the sixth slice: template literals with and without a tag, `let` and
  `const`, arrow functions, default and rest parameters, object and array destructuring,
  spread, computed property names, `for`-`of` and function declarations in blocks (without
  the web-compatibility semantics of Annex B.3.2). Classes, generators, promises, async
  functions and modules are refused with a `SyntaxError` that names the delivery slice that
  brings them. [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the
  nineteen delivery slices, what each brings and the order they are delivered in.
- The builtins end at the core library, what the sixth slice adds to it, and of the fifth
  slice `RegExp` with `exec`, `test` and its methods keyed by `Symbol.match`,
  `Symbol.replace`, `Symbol.search` and `Symbol.split`. `String.prototype.matchAll`,
  `RegExp.prototype[Symbol.matchAll]` and `RegExp.escape` do not exist. `Map`, `Set`,
  `Promise`, `Reflect`, `Proxy`, `Date`, `parseInt`, `parseFloat` and all of `Math` but `max`
  do not exist.
- test262: 18,155 of the 44,812 cases the gate's digest stands for pass, 40.5 %. The line
  reads `test262 total 53801 8982 7 44812 18155 26655 2`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 18,155 pass, 26,655 fail, two
  end at the memory budget. The first, second and sixth delivery slices pass all their
  cases in all their runs, and the third all but one
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`). Of the fifth, which is in work, 1,667 of
  2,724 cases pass (`test262-slice 05 2724 5441 0 0 2724 1667 1057 0`). A refusal never
  counts as a pass, also when the script catches it.
- The host interface: 113 of its 191 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the four delivered slices or for the fifth. 14 of the 58
  scenarios of the interface's model pass on the engine, and the other 44 name what they
  wait for. The interface will change before 1.0.
- The engine's own tests: `17 test binaries: 1104 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 864 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 695 ignored`; the
  695 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.7.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.6.0] - 2026-11-23

Regular expressions run: the literal, `RegExp`, `exec` and `test`, on the new crate
`swell-regexp`, whose matcher spends the step and memory budgets of its entry. The case
methods, `normalize` and `localeCompare` of `String` take every String. The fifth delivery
slice is in work, with eight of its fifteen stories delivered.

### Added

- Regular expressions: the literal, evaluated to a new object each time, and `RegExp` with
  `exec`, `test`, `toString`, `lastIndex` and the accessors `flags`, `source`, `global`,
  `hasIndices`, `ignoreCase`, `multiline`, `dotAll`, `sticky`, `unicode` and `unicodeSets`;
  named groups, lookbehind, match indices, the flags `d`, `g`, `m`, `s`, `u` and `y` and the
  modifiers `m` and `s`. The new crate `swell-regexp` parses a pattern, compiles it and
  matches it by backtracking with an explicit stack; a match spends the steps and the memory
  of its entry, so `/(a+)+$/` against a long run of `a` ends at the step budget. Not yet
  built, each a counted `SyntaxError` (`HOST-191`): the flags `i` and `v`, a property escape
  and a group name outside ASCII; `replace`, `replaceAll` and `split` given a regular
  expression; `match`, `matchAll` and `search` are missing. 17,775 of the 44,812 selected
  test262 cases pass (16,412 in 0.5.0), 1,327 of the 2,724 of the fifth slice (52).
- `String.prototype.replace`, `replaceAll` and `split` with a String to search for or to split
  at: the replacement patterns of GetSubstitution, a function as the replacement, and a method
  an object keys by `@@replace` or `@@split`.
- The pattern of a regular expression literal is checked when its script is parsed, by the
  grammar of the modes `u` and `v` and of Annex B.1.2 with neither: a pattern with an error
  is a `SyntaxError` of the script at the line and column of the literal, and a pattern
  nested past `parser_nesting` is a `SyntaxError` that names the limit (`HOST-142`).
- `toLowerCase`, `toUpperCase`, `toLocaleLowerCase` and `toLocaleUpperCase` map every String:
  the unconditional mappings of `SpecialCasing.txt`, the simple mappings of `UnicodeData.txt`
  and Final_Sigma, from five tables generated from the vendored Unicode 17.0.0 data. A case
  method charges a step per code unit read and per code unit written.
- `String.prototype.normalize` with the forms NFC, NFD, NFKC and NFKD of Unicode 17.0.0, and
  `localeCompare` for every String: it orders two Strings by the code units of their NFC
  forms and answers `+0` for canonically equivalent Strings. `localeCompare` costs three
  steps per code unit, not one. No method of `String.prototype` refuses a String outside
  ASCII any more.

### Changed

- The test steps of the gate cost about 270 CPU seconds, from about 300: tests of twelve files take a
  gate count, with the full count in the lane `slow`, five tests of the test262 runner that
  counted one fixture tree are one test, and `gate-selftest` starts the next planted script
  when one ends.

### Host interface

- `BINDINGS_REALM_HEAP_BYTES` (changed): 506,112, from 381,696. A realm of 300 host functions and
  40 host objects raises `heap_bytes()` by 449,848 since `String.prototype` has `replace`,
  `replaceAll` and `split`: an arena grows once more inside the window `HOST-187` measures. A
  host that sizes its memory budget by this ceiling raises it.
- `HOST-191` (new): a refusal that `RegExp` raises at run time for a part of the fifth slice
  that is not built yet is counted by the agent, like a refusal of the parser.

### Limits of this release

- Not in the language yet: in a regular expression the flags `i` and `v`, a property escape
  and a group name outside ASCII, each refused with a `SyntaxError` the agent counts
  (`HOST-191`); identifiers outside ASCII; the operator `**`; and of what came after ES5
  everything but the sixth slice: template literals with and without a tag, `let` and
  `const`, arrow functions, default and rest parameters, object and array destructuring,
  spread, computed property names, `for`-`of` and function declarations in blocks (without
  the web-compatibility semantics of Annex B.3.2). Classes, generators, promises, async
  functions and modules are refused with a `SyntaxError` that names the delivery slice that
  brings them. [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the
  nineteen delivery slices, what each brings and the order they are delivered in.
- The builtins end at the core library, what the sixth slice adds to it, and of the fifth
  slice `RegExp` with `exec` and `test`. `String.prototype.match`, `matchAll` and `search` do
  not exist, and `replace`, `replaceAll` and `split` given a regular expression end in a
  counted refusal. `Map`, `Set`, `Promise`, `Reflect`, `Proxy`, `Date`, `parseInt`,
  `parseFloat` and all of `Math` but `max` do not exist.
- test262: 17,775 of the 44,812 cases the gate's digest stands for pass, 39.7 %. The line
  reads `test262 total 53801 8982 7 44812 17775 27036 1`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 17,775 pass, 27,036 fail, one
  ends at the memory budget. The first, second and sixth delivery slices pass all their
  cases in all their runs, and the third all but one
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`). Of the fifth, which is in work, 1,327 of
  2,724 cases pass (`test262-slice 05 2724 5441 0 0 2724 1327 1397 0`). A refusal never
  counts as a pass, also when the script catches it.
- The host interface: 113 of its 191 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the four delivered slices or for the fifth. 14 of the 58
  scenarios of the interface's model pass on the engine, and the other 44 name what they
  wait for. The interface will change before 1.0.
- The engine's own tests: `17 test binaries: 1087 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 847 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 679 ignored`; the
  679 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.6.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.5.0] - 2026-11-23

The sixth delivery slice is delivered: `Symbol`, the iterator protocol, `for`-`of`, array
patterns, spread, computed property names and tagged templates run, and every test262 case
the slice counts passes.

### Added

- The rest of the sixth delivery slice. The `Symbol` builtin: the constructor, `Symbol.for` and
  `Symbol.keyFor`, `Symbol.prototype`, twelve well-known symbols and
  `Object.getOwnPropertySymbols`. Computed property names, object spread and object rest. The
  iterator protocol with the Array and String iterators: `Array.prototype.entries`, `keys`
  and `values`, `@@iterator` of an Array, a String and an arguments object. Function
  declarations in blocks and in `switch` cases, without the web-compatibility semantics of
  Annex B.3.2. The `for`-`of` statement, which calls the iterator's `return` once on `break`,
  on a `continue` to an outer loop, on `return` and on a throw from the body. `Array.from`,
  `Object.fromEntries` and `Object.groupBy`. Array patterns as bindings and as assignment
  targets, with elisions, defaults, nesting and a rest element, in declarations, parameters,
  `catch`, assignments and the heads of `for`, `for`-`in` and `for`-`of`; a pattern closes its
  iterator when it ends before the iterator does. Spread in array literals and in the
  arguments of a call, an optional call and `new`. Tagged templates: a site answers the same
  frozen template object at every evaluation, with its `raw` strings, and the object is
  freed with the code that holds the site.
- The sixth delivery slice, `bindings-and-iteration`, is delivered: every case it counts
  passes in every run (`test262-slice 06 3898 7351 0 0 3898 3898 0 0`; 1,364 of 4,436 in
  0.4.0). 143 cases that need a later slice moved there, each with its evidence in
  `conformance/test262/moves.tsv`.
- test262 after these: `test262 total 53801 8982 7 44812 16412 28399 1`, 16,412 of the 44,812
  cases that run pass (36.6 %; 13,575 in 0.4.0). `test262-refused 36178` of the 85,556 runs
  meet a refusal (41,572 in 0.4.0).
- The host interface: 190 numbered requirements, 110 of them with a test
  (`host-requirements: 190 requirement(s), 190 line(s): 110 test, 80 pending`; 108 in 0.4.0).
  None is pending for a delivered slice.

### Changed

- A test262 case is counted in the slice delivered last among the slices it needs, by the
  order of delivery the design's block names (1, 2, 3, 6, 5, 7, 10, 4, 9, 8, 11 to 21), and
  no longer in the slice with the highest number. The slices are not delivered in the order
  of their numbers, so a `RegExp` case that uses `Symbol` can first pass after slice 5, not
  after slice 6. Slice 4 counts 1,598 cases, slice 5 2,724, slice 6 3,898, slice 8 9,144 and
  slice 10 3,879 (1,516, 2,394, 4,436, 6,292 and 6,651 in 0.4.0, where the moves of the
  close also count). No run changed its outcome through the rule.
- The heaviest tests are lighter, so that none is near the 60 s limit of a test when the
  machine is loaded: no test of the gate takes over 1.5 CPU seconds alone in the default mode,
  and none of the lane `slow` over 2.5 at its full count, but for one test of determinism
  that runs its corpus three times. The fuzz test of the host interface runs 400 sequences
  at its full count (2,000 in 0.4.0); the gate's count is unchanged. The full count of the
  parser's stack-bound check is 7,500 levels on three quarters of the stack of 10,000
  levels. Only test code changed.
- One case of slice 18 that passed in 0.4.0 no longer passes:
  `test/annexB/built-ins/Array/from/iterator-method-emulates-undefined.js`. It passed only on
  the error of the missing `Array.from`; the method exists now, and the case needs
  `$262.IsHTMLDDA`, which slice 18 brings.

### Host interface

- `HostIntrinsic::ArrayPrototypeValues`, `HostIntrinsic::ArrayPrototypeEntries`,
  `HostIntrinsic::ArrayPrototypeKeys`, `HostIntrinsic::ArrayPrototypeForEach` and
  `WellKnownSymbol::Iterator` (new): what Web IDL gives an iterable interface (`HOST-185`).

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, the
  operator `**`, and of what came after ES5 everything but the sixth slice: template
  literals with and without a tag, `let` and `const`, arrow functions, default and rest
  parameters, object and array destructuring, spread, computed property names, `for`-`of`
  and function declarations in blocks (without the web-compatibility semantics of Annex
  B.3.2). Classes, generators, promises, async functions and modules are refused with a
  `SyntaxError` that names the delivery slice that brings them.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices, what each brings and the order they are delivered in.
- The builtins end at the core library and what the sixth slice adds to it: `Symbol`, the
  Array and String iterators, `Array.from`, `Object.fromEntries` and `Object.groupBy`. `Map`,
  `Set`, `Promise`, `Reflect`, `Proxy`, `Date`, `RegExp`, `parseInt`, `parseFloat` and all of
  `Math` but `max` do not exist. A String method that needs the Unicode tables of a later
  slice refuses a code unit outside ASCII (`HOST-165`).
- test262: 16,412 of the 44,812 cases the gate's digest stands for pass, 36.6 %. The line
  reads `test262 total 53801 8982 7 44812 16412 28399 1`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 16,412 pass, 28,399 fail, one
  ends at the memory budget. The first, second and sixth delivery slices pass all their
  cases in all their runs, and the third all but one
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 3898 7351 0 0 3898 3898 0 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 110 of its 190 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the four delivered slices. 14 of the 58 scenarios of the
  interface's model pass on the engine, and the other 44 name what they wait for. The
  interface will change before 1.0.
- The engine's own tests: `15 test binaries: 977 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 796 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 629 ignored`; the
  629 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.5.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.4.0] - 2026-11-22

Storage is charged before it is allocated and work before it is done, the host interface gains
what an embedder that generates bindings needs, and the first five parts of the sixth
delivery slice run.

### Added

- The first parts of the sixth delivery slice: template literals without a tag, and `let`,
  `const` and block scopes with the temporal dead zone and a fresh record per iteration of a
  `for (let ...)` loop. A tagged template is refused by name until its story. Arrow functions
  with an expression body or a block body: `this` and `arguments` are those of the code that
  made the arrow, and an arrow has no [[Construct]] and no `prototype`. Default parameters and
  rest parameters, in functions, arrows and the `Function` constructor: a default is evaluated
  only for `undefined`, and a parameter read or written before its initializer has run throws
  a `ReferenceError` that names it. Object destructuring in declarations, parameters,
  assignments, `for` heads and `catch`, with shorthand names, renamed targets, nested object
  patterns and defaults; a rest property, a computed name and array patterns are refused by
  name until their stories.
- test262 after these: `test262 total 53801 8982 7 44812 13575 31236 1`, 13,575 of the 44,812
  cases that run pass (30.2 %; 12,120 in 0.3.0). The sixth slice passes 1,364 of its 4,436
  cases (`test262-slice 06 4436 8413 0 0 4436 1364 3072 0`; 100 in 0.3.0).
  `test262-refused 41572` of the 85,556 runs meet a refusal. One case ends at the memory
  budget, `test/staging/sm/regress/regress-610026.js`: its source of about 4.19 million empty
  blocks has a tree that the 512 MiB of the runner do not hold, now that the tree is charged.
- The host interface: 190 numbered requirements, 108 of them with a test
  (`host-requirements: 190 requirement(s), 190 line(s): 108 test, 82 pending`).

### Changed

- The ESS pins moved from 0.96.0 to 0.97.4 (`spec/ess-inputs.yaml`, the conformance workspace,
  the forge's workflow). The suite is byte-identical: 58 scenarios, 41 refusals, 58 passing on
  the interpreter and 14 against the engine, as before.
- The gate now fails when a public item of the host interface or a `HOST-<n>` requirement
  changed since the last release and the changelog does not name it under Host interface, in
  the name form of `docs/releasing.md`.
- Storage is charged before it is allocated and work before it is done
  (`docs/design/engine.md`, "Memory budget" and "The step budget"). What that changed:
  - The parser's tree, the scope analysis, the compile step's scratch and the code are charged
    to `memory_bytes` before they grow and credited when they drop, and the code's charge
    passes to its script record. Every parse reads the interrupt at least once per 4,096 code
    units or nodes; `eval` and `Function` read it before their copy of the source, and `eval`
    charges a step per code unit of its source.
  - The key list of an object is charged, steps and bytes, before the list is built, and
    credited when its holder drops it. Index keys added out of order are sorted in charged
    passes; a list whose index keys are in order costs the steps it did.
  - The key list a `for`-`in` loop holds is charged to `memory_bytes` before it grows and
    while the loop runs, and credited when the loop ends, by `break`, `return`, a throw or a
    termination included.
  - Setting an array's length, compacting a property or binding list, and calling a bound
    function charge their steps before the work; the totals are as before. The list of keys
    that a shorter length deletes is charged before it is made, and the arguments of a bound
    function are not copied before the gathered list fits the budget.
  - Comparing two Strings, and using a String of more than 32 code units as a property key,
    charge a step per code unit past 32 before the work; shorter ones cost what they did.
  - Builtins no longer copy a whole String they do not use and charge a String's steps before
    they copy it. `JSON.stringify` compares the Strings of a replacer list in place, a step
    per code unit past 32. `JSON.parse` and `JSON.rawJSON` charge the step per code unit of
    their text before they copy it: a text parsed to its end costs what it did, and one that
    fails early has paid for its whole text.
  - The parameter map of a mapped arguments object and the arguments a call records are
    charged to the memory budget before they are made; a call no longer copies its
    function's parameter list.
  - A bound function's arguments, the argument list of a bound call and the arguments a
    builtin call records are charged to the memory budget before they are copied and stay
    charged while they are held.
  - The atom table grows charged first, also when deleted buckets would make it grow.
  - The compile step charges what a node's expansion holds as it is made: the script `1`
    needs 2,228 bytes of room for its parse and compile.
- A termination that ends the deletions of an Array length write leaves the Array's old
  length, so its length is never below an index it still has.
- A pending interrupt ends `parse_script` before the source is copied: it answers
  `Parsed::Terminated(Interrupted)` and consumes the flag (`HOST-166`, `HOST-167`).
- Each crate's integration tests build into one test binary, `tests/main.rs`, generated from
  the test files and checked by the gate: 72 test executables became 15.
- The dev and test builds carry no debug information, and the gate step
  `test-binaries-check` fails when a test executable has a `.debug_` section. The stack
  requirements are also tested in the release profile and in a profile without debug
  information, in the lane `slow`.

### Fixed

- Nested arrow functions keep the parser within `ParseOptions::stack_bytes` (229.3 bytes a
  level, now 181.3; at `parser_nesting` 10,000 the stated stack overflowed), and the gate's
  check of the stack bound runs on a quarter of the stack of 10,000 levels, so it fails wherever
  the full count does.

### Host interface

- Four entries below break a host written for 0.3.0, each marked "Breaking": `get`,
  `HostObjectSpec::error_kind`, the reading of `heap_bytes`, and `HOST-70`.
- Breaking: `get` takes `&mut self`. The value read is handed to the running scope and stays
  valid until the scope ends, also when the host drops the root and calls back into the
  engine. A caller that read through a shared `&Cx` or `&HookCx` takes `&mut` instead
  (`HOST-170`).
- `live_bytes` (new, on `Agent`): what the live records hold, slots included. It falls by
  everything a sweep frees and sets the collection threshold (`HOST-171`).
- Breaking: `heap_bytes` keeps its signature and changes its reading, with `memory_bytes`: an arena's
  slots are counted by capacity, charged when the arena grows and kept while the agent lives,
  together with the collector's workspace and the atom table's buckets. A collection lowers
  `heap_bytes` only by what the freed records owned. Before, a slot was charged with each
  record and credited by the sweep that freed it, and the collector's workspace was not
  charged. A String with no code units that lands in a spare slot is no longer refused by a
  budget with no room left.
- `get_prototype_of`, `set_prototype_of`: new methods of `Cx`, the [[GetPrototypeOf]] and
  [[SetPrototypeOf]] of an object, so an interface object takes its parent's interface object
  as [[Prototype]] (`HOST-180`).
- `charge_host_bytes` (new, on `Cx` and `HookCx`): charges bytes the host holds for the agent
  to `memory_bytes` and answers a `HostBytes` (`HOST-175`, `HOST-176`).
- `HostBytes` (new): bytes the host charged for the agent; neither `Clone` nor `Copy`; dropping
  it gives them back, in a collection or at the next charge.
- `resize_host_bytes` (new, on `Cx` and `HookCx`): changes that charge in place; growing
  charges the difference first, shrinking gives it back at once.
- `credit_host_bytes` (new, on `Agent`): gives one charge back at once and answers its bytes
  (`HOST-176`).
- `charge_steps` (new, on `Cx`): charges the steps of a host function's own work to the
  running entry (`HOST-177`).
- `intrinsic` and `well_known_symbol` (new, on `Cx` and `HookCx`), `HostIntrinsic` (new, with
  `HostIntrinsic::ObjectPrototype`, `HostIntrinsic::FunctionPrototype`,
  `HostIntrinsic::ErrorPrototype`) and `WellKnownSymbol` (new, with
  `WellKnownSymbol::ToStringTag`, `WellKnownSymbol::Unscopables`): the host names these
  intrinsics and symbols from a closed list (`HOST-183`, `HOST-184`).
- A host function charges the box of its data and of its steps, and four constants state
  what a host that generates bindings can count on (`HOST-186`, `HOST-187`), each the
  measured rise plus an eighth: `HOST_FUNCTION_LIVE_BYTES` (576, a host function with a name
  of at most 32 code units, its data apart), `HOST_OBJECT_LIVE_BYTES` (256, a host object,
  its data apart), `PROPERTY_LIVE_BYTES` (448, one definition of a new property, averaged
  over 300 on one object) and `BINDINGS_REALM_HEAP_BYTES` (381,696, a realm of 300 host
  functions and 40 host objects with 600 definitions, arena growth included). A release
  that raises one names it here.
- Breaking: `HostObjectSpec::error_kind` (new field; a literal adds `error_kind: None`) gives
  a host object an [[ErrorData]] slot, as Web IDL gives a `DOMException`: `Error.isError`
  answers true for it (`HOST-179`).
- `error_summary` (new, on `Cx` and `HookCx`), `Agent::error_summary` (new) and `ErrorSummary`
  (new), with `ErrorSummary::kind` and `ErrorSummary::message`: the host reads whether a value
  is an error object, the kind of the constructor that made it and its own `message`, without
  running code (`HOST-179`).
- `last_entry_error_site` (new, on `Agent`): after an entry that answered `Parsed::Errors` or
  `Throw`, where the error arose; after any other entry `None` (`HOST-172`, `HOST-173`,
  `HOST-174`).
- `ErrorSite` (new): what that read answers. `ErrorSite::line` and `ErrorSite::column` are the
  position of the first parse error or of the throwing instruction, counted from 1, a column
  in UTF-16 code units; `ErrorSite::dynamic` is true when the position is in eval code or in
  the source text of a `Function` constructor; `ErrorSite::host_defined` is the
  [[HostDefined]] of the script whose code raised it, `None` for `Function` code and indirect
  eval.
- Breaking: after `Terminated(MemoryBudget)` the agent stays `Alive`, and the next entry runs when the
  free room, `memory_bytes` less `heap_bytes()`, covers it; it ends in
  `Terminated(MemoryBudget)` again when it does not, so a host makes room first: it drops
  roots and calls `collect`, or gives bytes back with `credit_host_bytes`. `HOST-70` said
  before that the script `1` runs whatever room is left. `HOST-188`: the calls that take no
  memory answer with a room of 0. `HOST-189`: an entry the room does not cover ends the same
  way each time. `HOST-190`: with at least 3,072 bytes of room the script `1` answers
  `Normal(1)` as in a fresh agent.
- A handle the host keeps without a root is a stated weak reference (`HOST-178`): it reads
  its object while the object lives and answers `StaleHandle` after a collection freed it,
  also after its slot holds a new object. No code changed for it; it is tested now.
- `HOST-166`, `HOST-167`, `HOST-168`, `HOST-169`, `HOST-170`, `HOST-171`, `HOST-172`, `HOST-173`, `HOST-174`, `HOST-175`, `HOST-176`, `HOST-177`, `HOST-178`, `HOST-179`, `HOST-180`, `HOST-181`, `HOST-182`, `HOST-183`, `HOST-184`, `HOST-185`, `HOST-186`, `HOST-187`, `HOST-188`, `HOST-189`, `HOST-190`: new requirements.
- `HOST-29`, `HOST-52`, `HOST-70`, `HOST-102`: changed requirements.

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, and of
  what came after ES5 everything but template literals without a tag, `let` and `const`,
  arrow functions, default and rest parameters and object destructuring. Array patterns,
  spread, `for`-`of`, computed property names, classes, generators, promises and modules are
  refused with a `SyntaxError` that names the delivery slice that brings them.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- The builtins end at the core library. `Symbol`, `Map`, `Set`, `Promise`, `Reflect`, `Proxy`,
  `Date`, `RegExp`, `parseInt`, `parseFloat` and all of `Math` but `max` do not exist. A
  String method that needs the Unicode tables of a later slice refuses a code unit outside
  ASCII (`HOST-165`).
- test262: 13,575 of the 44,812 cases the gate's digest stands for pass, 30.2 %. The line
  reads `test262 total 53801 8982 7 44812 13575 31236 1`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 13,575 pass, 31,236 fail, one
  ends at the memory budget. The first two delivery slices pass all their cases in all their
  runs, the third all but one, and the sixth 1,364 of 4,436
  (`test262-slice 01 31 62 0 0 31 31 0 0`, `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`,
  `test262-slice 06 4436 8413 0 0 4436 1364 3072 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 108 of its 190 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the first three slices. 14 of the 58 scenarios of the
  interface's model pass on the engine, and the other 44 name what they wait for. The
  interface will change before 1.0.
- The engine's own tests: `15 test binaries: 858 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 2 test binaries: 684 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 524 ignored`; the
  524 are the tests that step's filter leaves out).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.4.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.3.0] - 2026-11-22

The third delivery slice, the core library, is delivered: 7,010 of its 7,011 test262 cases
pass.

### Added

- The core library of the third delivery slice: the integrity, prototype, enumeration and copy
  methods of `Object`; `Function.prototype.apply` and the source text of a function;
  `globalThis`, the URI functions and `Error.isError`; `JSON.parse` with a reviver and raw
  JSON; the iteration, search, copy, mutation and sort methods of `Array`; the access, search,
  transform and compare methods of `String`. The records of the waves name each builtin
  (`docs/plan/slice-03-wave-1.md` to `-3.md`).
- An object with more than 8 properties finds a property through an index map, and an array
  keeps its elements dense. The scan of a property list that 0.2.0 named among its limits is
  gone.
- The third delivery slice is delivered with 7,010 of its 7,011 cases
  (`test262-slice 03 7011 13829 0 0 7011 7010 1 0`). The one case that does not pass,
  `test/built-ins/String/prop-desc.js`, fails in the pinned test262 harness before the
  engine's answer is judged; `docs/design/engine.md`, section 12, states it. 624 more cases
  that need a later slice moved there with their evidence
  (`test262-moves: 1656 case(s) moved`).
- test262 after these: `test262 total 53801 8982 7 44812 12120 32692 0`, 12,120 of the 44,812
  cases that run pass (27.0 %; 7,742 in 0.2.0). `test262-refused 46581` of the 85,556 runs
  meet a refusal of a construct the parser does not have.
- A documentation site under `website/`. It types no number: `task site-data` writes its data
  from the gate's committed files, and the gate step `site-data-check` fails when the data
  differs from them.
- The host interface: 165 numbered requirements, 84 of them with a test
  (`host-requirements: 165 requirement(s), 165 line(s): 84 test, 81 pending`; 164 and 80 in
  0.2.0). `HOST-165`: a String method that needs the tables of a later slice refuses a code
  unit outside ASCII with a counted refusal.

### Changed

- The gate runs its tests under cargo-nextest, ends a test at 60 seconds, builds every test
  binary before its clock and holds its test steps to 120 seconds
  (`gate-time test-steps 100.7 of 120`). It no longer makes the full test262 run: it compares a
  digest of the run's sources with a committed one, and the full run is the lane `slow`
  (`task slow`), which a release waits for.
- The compiler is pinned in `rust-toolchain.toml` (1.99.1) for the gate and the forge alike,
  and `task check-runner` runs the gate with the forge runner's variables.
- On the forge a change of planning or prose files alone skips the build and the tests, and a
  change of the documentation site alone runs the site's data check.
- `Object.getOwnPropertySymbols` is counted with the sixth slice, where `Symbol` is.
- Eight cases of later slices that passed in 0.2.0 no longer pass. Each passed only on the
  error of a method that was missing; the method exists now, and the case fails on the part
  of a later slice it needs: one of slice 5 (`Function/prototype/apply/this-not-callable.js`,
  a regular expression literal), five of slice 6 (String cases that call a method with a
  Symbol) and two of slice 18 (`trimLeft/reference-trimStart.js`,
  `trimRight/reference-trimEnd.js`). `docs/plan/slice-03-wave-2.md` names them.

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, and
  everything after ES5 (`let` and `const`, arrow functions, classes, generators, promises,
  modules, and the rest). A script that uses one is refused with a `SyntaxError` that names
  the delivery slice that brings it.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- The builtins end at the core library. `Symbol`, `Map`, `Set`, `Promise`, `Reflect`, `Proxy`,
  `Date`, `RegExp`, `parseInt`, `parseFloat` and all of `Math` but `max` do not exist. A
  String method that needs the Unicode tables of a later slice refuses a code unit outside
  ASCII (`HOST-165`).
- test262: 12,120 of the 44,812 cases the gate's digest stands for pass, 27.0 %. The line
  reads `test262 total 53801 8982 7 44812 12120 32692 0`: 53,801 cases exist, 8,982 are
  excluded by recorded decision, 7 do not apply, 44,812 run, 12,120 pass, 32,692 fail, none
  ends at a budget. The first two delivery slices pass all their cases in all their runs and
  the third all but one (`test262-slice 01 31 62 0 0 31 31 0 0`,
  `test262-slice 02 4259 7543 0 0 4259 4259 0 0`,
  `test262-slice 03 7011 13829 0 0 7011 7010 1 0`). A refusal never counts as a pass, also
  when the script catches it.
- The host interface: 84 of its 165 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit; none is pending for the first three slices. 14 of the 58 scenarios of the
  interface's model pass on the engine, and the other 44 name what they wait for. The
  interface will change before 1.0.
- The engine's own tests: `68 test binaries: 667 passed, 0 failed, 1 ignored`, the tests of
  the engine crate in the mode that collects garbage at every safe point
  (`test-collect: 49 test binaries: 506 passed, 0 failed, 1 ignored`), and the test of an
  injected engine fault (`test-faults: 1 test binaries: 1 passed, 0 failed, 0 ignored`).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.3.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.2.0] - 2026-11-21

The second release. swell now runs the whole ES5 level of the language: a script may leave out
its semicolons, label its loops, and use `with`, `arguments`, `eval` and the global
`Function`. The first two of nineteen delivery slices are delivered, and every test262 case
of both passes. Most of the standard library and everything after ES5 is still missing; the
limits below say how much.

### Added

- The language of the second delivery slice, first part: automatic semicolon insertion,
  labelled statements, `??`, `?.`, logical assignment, optional catch binding, trailing commas
  in argument lists, numeric separators, a hashbang line, plain method definitions and
  shorthand properties in object literals.
- `Number` (with `toString` for every radix), `Boolean`, `isNaN` and `isFinite`, and the
  `toString` and `valueOf` of the String wrapper.
- The conformance runner writes one line per run with what ended it, and its host has
  `$262.evalScript`.
- The `with` statement and the `arguments` object, mapped and unmapped. A name inside the body
  of a `with` is looked up at run time, each record visited charged as one step; every other
  name keeps the slot the compiler gave it.
- `eval`, direct and indirect, and the global `Function`. A non-strict direct `eval` can add
  bindings to the scope of its caller.
- The host is asked before a string is compiled (`host_ensure_can_compile_strings`) and may
  give the code for an object passed to `eval` (`host_get_code_for_eval`).
- `Agent::refusals()`: a count of the refusals the engine raised, at parse or run time, caught
  by the script or not. The conformance runner fails a run in which it moved.
- `Array.prototype.toString`, before the rest of `Array`: `[] + ""` is `""`.
- The second delivery slice is delivered: all of its 4,259 test262 cases pass in all 7,543
  runs. 1,008 cases that need a later slice moved there, each with its evidence in
  `conformance/test262/moves.tsv`, which the gate checks (`test262-moves-check`).
- test262 after these: `test262 total 53801 8982 7 44812 7742 37070 0`, 7,742 of the 44,812
  cases that run pass (17.3 %; 3,969 in 0.1.0).
  46,493 of the 85,556 runs meet a refusal of a construct the parser does not have
  yet (53,564 ended at one in 0.1.0, where a refusal a script caught was not counted).
- The gate runs the scenarios of the host interface model against the engine itself
  (`conformance/ess`, the task `conform-engine`): `58 scenarios: 14 passed, 0 failed, 0 error,
  44 unsupported`. An unsupported scenario names the later slice or the reason it waits for,
  and a planted defect (an interrupt that does nothing) must fail a scenario.
- The model states whose realm it is: a script is parsed in a realm of the agent that is
  called, and a realm of another agent is refused as `ForeignHandle`.
- The first delivery slice is delivered: no requirement of the host interface is pending for
  it, and the gate fails if one ever is again.

### Changed

- `Limits::min_stack_bytes()` asks for 14 KiB per re-entry instead of 30 KiB. In 0.1.0 a debug
  build with full debug information could overflow the promised stack: one re-entry took up
  to 31,314 bytes. The run loop no longer holds the locals of every instruction in its frame:
  one re-entry takes up to 11,100 bytes in a debug build (through `toJSON`), and an agent
  with `reentry_depth` 1,000 and `parser_nesting` 1,000 needs a thread of 14,672,640 bytes
  (31,073,024 in 0.1.0).
- The completion value of a `try` statement whose `finally` block is left by `break` or
  `continue` is `undefined`, as the standard says; it was the value of the `try` or `catch`
  block. A script sees it through `eval` and as the result of a script.
- The design gives the global `Function`, plain method definitions and shorthand properties,
  `Number`, `Boolean`, `isNaN` and `isFinite` to the second delivery slice. The host interface
  gains three requirements (`HOST-162` to `HOST-164`), among them `Agent::refusals()`, a count
  of the refusals the engine raised. 80 of the 164 requirements have a test
  (`host-requirements: 164 requirement(s), 164 line(s): 80 test, 84 pending`; 70 in 0.1.0).

### Limits of this release

- Not in the language yet: regular expression literals, identifiers outside ASCII, and
  everything after ES5 (`let` and `const`, arrow functions, classes, generators, promises,
  modules, and the rest). A script that uses one is refused with a `SyntaxError` that names
  the delivery slice that brings it.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- Most methods of the builtins are missing. Of `Array`, for example, `isArray`, `join`, `map`,
  `push` and `toString` exist; `Function.prototype.apply`, `JSON.parse`, `parseInt`, `Date`,
  `RegExp` and all of `Math` but `max` do not. The third delivery slice brings the core
  library.
- test262: 7,742 of the 44,812 cases the gate runs pass, 17.3 %. Its line reads
  `test262 total 53801 8982 7 44812 7742 37070 0`: 53,801 cases exist, 8,982 are excluded by
  recorded decision (`Temporal`, `Intl` and proposals at stage 3), 7 do not apply, 44,812
  run, 7,742 pass, 37,070 fail, none ends at a budget. The first two delivery slices pass all
  their cases in all their runs (`test262-slice 01 31 62 0 0 31 31 0 0`,
  `test262-slice 02 4259 7543 0 0 4259 4259 0 0`).
- What that number says: the second slice counted 5,267 cases before its close; 1,008 of them
  need a construct or a builtin of a later slice and are counted there now
  (`test262-moves: 1032 case(s) moved`, 24 of them cases of the third slice that need `**`).
  A refusal never counts as a pass, also when the script catches it: `test262-refused 46493`
  of the 85,556 runs meet one.
- A property is found by a scan of its object's property list, and the comparisons of that
  scan are not charged to the step budget: a script with very large objects runs longer than
  its step count says.
- The host interface: 80 of its 164 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit (`host-requirements: 164 requirement(s), 164 line(s): 80 test, 84 pending`);
  none is pending for the first two slices. 14 of the 58 scenarios of the interface's model
  pass on the engine, and the other 44 name what they wait for. The interface will change
  before 1.0.
- The engine's own tests: `62 test suites: 341 passed, 0 failed, 1 ignored`, the same tests of
  the engine crate in the mode that collects garbage at every safe point (`test-collect: 38
  test suites: 238 passed, 0 failed, 1 ignored`), and the test of an injected engine fault
  (`test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored`).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.2.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

## [0.1.0] - 2026-11-21

The first release. swell is an ECMAScript engine written from scratch in Rust, with no
third-party crate and no unsafe code. This release runs scripts of the ES5 level of the
language; it is a small part of the standard, and the limits below say how small.

### Added

- The library crate `swell` (`crates/swell`): a host creates an agent with limits and seeds
  (`Agent::new`), creates a realm (`create_realm`), parses a source (`parse_script`) and
  evaluates it (`evaluate_script`), calls and constructs functions (`Agent::call`,
  `Agent::construct`), and works with values inside a scope (`with_cx`).
- The language of this release: the ES5 grammar in strict and non-strict mode, with the early
  errors of strict code. A construct of a later part of the language is refused by the parser
  with a `SyntaxError` whose message names the delivery slice that brings it, for example
  `class is not supported before slice 8`.
- Bounded execution. A run has a step budget and a memory budget, and an interrupt handle ends
  it from another thread: the run ends with an outcome the host reads
  (`Terminated(StepBudget)`, `Terminated(MemoryBudget)`, `Terminated(Interrupted)`), and no
  `catch` or `finally` block of the script runs after it. Frame depth, re-entry depth and
  string length are limited by a `RangeError`, parser nesting by a `SyntaxError`.
  `Limits::min_stack_bytes()` states the thread stack an agent needs for its limits.
- No ambient authority: the engine reads no file, clock, random source or environment
  variable. What a script may observe of the outside comes through the host's hooks, and two
  runs with the same seeds and the same hook answers are the same.
- Misuse of the interface has an answer: a stale, released or foreign handle gets
  `Err(StaleHandle)` or `Err(ForeignHandle)`, and a broken invariant of the engine ends the
  run with `Terminated(EngineFault)` and leaves the agent faulted. A seeded generator of call
  sequences holds this in the gate.
- The builtins that the test262 harness files call: members of `Object`, `Function.prototype`,
  `Array`, `String`, `Math` and `JSON` (`JSON.stringify` among them), the Error constructors,
  and bound functions.
- The supporting crates: `swell-syntax` (lexer and parser), `swell-compiler` (bytecode),
  `swell-num` (number parsing and printing, the keyed hash) and `swell-unicode`.
- The conformance runner `swell-test262`: it runs every test262 case that is not excluded, as
  written and in strict mode, each run in a fresh agent and realm under a step and a memory
  budget, with `print` and `$262` from its host.
- The gate, `task check`, which pins test262 by commit and checksum and holds the counts below.

### Limits of this release

- Not in the language yet: `eval`, `with`, the `arguments` object, labels, automatic semicolon
  insertion (a statement needs its semicolon), regular expression literals, and everything
  after ES5 (`let` and `const`, arrow functions, classes, generators, promises, modules, and
  the rest). The global `Function` and most methods of the builtins are missing.
  [`docs/design/engine.md`](docs/design/engine.md), section 12, lists the nineteen delivery
  slices and what each brings.
- test262: 3,969 of the 44,812 cases the gate runs pass, 8.9 %. Its line reads
  `test262 total 53801 8982 7 44812 3969 40843 0`: 53,801 cases exist, 8,982 are excluded by
  recorded decision (`Temporal`, `Intl` and proposals at stage 3), 7 do not apply, 44,812
  run, 3,969 pass, 40,843 fail, none ends at a budget. The first delivery slice, the test262
  harness itself, passes its 31 cases in all 62 runs (`test262-slice 01 31 62 0 0 31 31 0 0`).
- What that number says: a construct the parser does not have yet is refused, and a refusal
  never counts as the `SyntaxError` a negative case asks for. 53,564 of the 85,556 runs end at
  such a refusal (`test262-refused 53564`). A case outside the first slice passes when it
  needs only the ES5 grammar and the builtins above; 269 negative cases pass on a generic
  syntax error, and 129 passing cases hold no assertion and pass by not throwing.
- The host interface: 70 of its 161 numbered requirements
  ([`docs/design/host-interface.md`](docs/design/host-interface.md)) are held by a test in
  this commit (`host-requirements: 161 requirement(s), 161 line(s): 70 test, 91 pending`);
  none is pending for the first slice. The interface will change before 1.0.
- The engine's own tests: `55 test suites: 268 passed, 0 failed, 1 ignored`, the same tests of
  the engine crate in the mode that collects garbage at every safe point (`test-collect: 31
  test suites: 194 passed, 0 failed, 1 ignored`), and the test of an injected engine fault
  (`test-faults: 1 test suites: 1 passed, 0 failed, 0 ignored`).
- No binary, no command line, and the crates are not on a package registry. To use the
  library, depend on the repository at the tag:
  `swell = { git = "https://github.com/beyond10x/swell", tag = "0.1.0" }`.
- Measured on x86_64 Linux with rustc 1.99.1 only.

[Unreleased]: https://github.com/beyond10x/swell/compare/0.12.0...HEAD
[0.12.0]: https://github.com/beyond10x/swell/releases/tag/0.12.0
[0.11.0]: https://github.com/beyond10x/swell/releases/tag/0.11.0
[0.10.0]: https://github.com/beyond10x/swell/releases/tag/0.10.0
[0.9.1]: https://github.com/beyond10x/swell/releases/tag/0.9.1
[0.9.0]: https://github.com/beyond10x/swell/releases/tag/0.9.0
[0.8.0]: https://github.com/beyond10x/swell/releases/tag/0.8.0
[0.7.0]: https://github.com/beyond10x/swell/releases/tag/0.7.0
[0.6.0]: https://github.com/beyond10x/swell/releases/tag/0.6.0
[0.5.0]: https://github.com/beyond10x/swell/releases/tag/0.5.0
[0.4.0]: https://github.com/beyond10x/swell/releases/tag/0.4.0
[0.3.0]: https://github.com/beyond10x/swell/releases/tag/0.3.0
[0.2.0]: https://github.com/beyond10x/swell/releases/tag/0.2.0
[0.1.0]: https://github.com/beyond10x/swell/releases/tag/0.1.0
