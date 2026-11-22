//! The bytecode of swell (`docs/design/engine.md`, decision 3): a stack machine whose every
//! ECMAScript frame lives on the engine's own frame stack.
//!
//! # The machine the instructions run on
//!
//! A frame has an instruction index, an operand stack, `locals` slots (each starts as
//! `undefined`; a slot of a `let` or `const` binding is uninitialized from the entry of its
//! scope to its declaration, [`Instruction::Uninitialize`], as are the slots of the records
//! [`Instruction::PushLexicalEnv`] makes), the running execution context's LexicalEnvironment
//! (an Environment Record, `#sec-environment-records`), its handler table and, for a script, a
//! completion value register that starts as `undefined`. The instructions of one template run
//! in that frame; a jump names the index of an instruction of the same template.
//!
//! # Operands
//!
//! Every operand is a `u32`: an index into the constants of the [`Code`] (`strings`, `numbers`,
//! `functions`), a slot, an environment hop count, a count of values or an instruction index.
//! [`crate::CompileError`] is the answer when a template would need more than `u32::MAX`
//! instructions, constants or slots.
//!
//! # Values on the operand stack
//!
//! Besides language values the stack holds five kinds of engine entries, each produced and
//! consumed by instructions of this set only: the reference of a global name
//! ([`Instruction::ResolveGlobal`]), the reference of a name looked up at run time
//! ([`Instruction::ResolveName`]), the enumerator of a for-in statement
//! ([`Instruction::ForInStart`]), the Iterator Record of a for-of statement or an array pattern
//! ([`Instruction::GetIterator`]) and the tag of a completion record ([`Instruction::Tag`]).
//!
//! # Completion records
//!
//! A `finally` block is entered with a completion record on the operand stack (`#sec-try-statement`):
//! two entries, a value and a tag on top. Tag [`TAG_NORMAL`] is a normal completion (the value is
//! `undefined`), [`TAG_THROW`] a throw completion of the value, [`TAG_RETURN`] a return of the value,
//! and every tag from [`TAG_FIRST_JUMP`] up a `break` or `continue` that leaves the try statement
//! (the value is `undefined`): which one, the code after the `finally` block says
//! ([`Instruction::JumpIfTag`]). The interpreter makes the throw record itself, when an exception
//! reaches a [`HandlerKind::Finally`] handler; the compiled code makes the others.
//!
//! # Step budget
//!
//! The interpreter charges one step per instruction and checks the budget at every
//! [`Instruction::LoopJump`], call and return (decision 3). Every jump whose target is not after
//! it is a `LoopJump`; every other jump goes forward. So a loop cannot run without passing a
//! check.

use swell_syntax::ast::{FunctionKind, Span};

/// Tag of a normal completion record.
pub const TAG_NORMAL: u32 = 0;
/// Tag of a throw completion record, pushed by the interpreter.
pub const TAG_THROW: u32 = 1;
/// Tag of a return completion record.
pub const TAG_RETURN: u32 = 2;
/// The first tag of a `break` or `continue` completion record.
pub const TAG_FIRST_JUMP: u32 = 3;

/// One instruction. The stack effect is written `[before] -> [after]`, top of the stack last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Instruction {
    // ---- values --------------------------------------------------------------------------
    /// `[] -> [undefined]`.
    Undefined,
    /// `[] -> [null]`.
    Null,
    /// `[] -> [true]`.
    True,
    /// `[] -> [false]`.
    False,
    /// `[] -> [n]`: the Number constant `numbers[k]`.
    Number(u32),
    /// `[] -> [s]`: the String constant `strings[k]`.
    String(u32),
    /// `[] -> [this]`: ResolveThisBinding (`#sec-resolvethisbinding`); a function's `this` is
    /// bound by OrdinaryCallBindThis (`#sec-ordinarycallbindthis`); an arrow function's is the
    /// value its [`Instruction::ArrowClosure`] kept.
    This,
    /// `[] -> [f]`: a function object made from `functions[k]` with the LexicalEnvironment as
    /// its environment: InstantiateOrdinaryFunctionObject for a declaration
    /// (`#sec-runtime-semantics-instantiateordinaryfunctionobject`), MakeMethod with
    /// DefineMethodProperty's function for an accessor, and for an expression
    /// InstantiateOrdinaryFunctionExpression
    /// (`#sec-runtime-semantics-instantiateordinaryfunctionexpression`): when the template has
    /// [`FunctionTemplate::self_binding`], its environment is a new declarative environment of one
    /// immutable binding, slot 0, holding the function itself.
    Closure(u32),
    /// `[] -> [f]`: InstantiateArrowFunctionExpression
    /// (`#sec-runtime-semantics-instantiatearrowfunctionexpression`) of the arrow function
    /// `functions[k]`, with the LexicalEnvironment as its environment: [[ThisMode]] lexical, no
    /// [[Construct]], no `prototype`. The function object keeps the `this` value of the frame
    /// that runs this instruction, and a call of it puts that value in its own frame's `this`
    /// in place of OrdinaryCallBindThis (`docs/design/engine.md`, decision 3, "`this` and
    /// `arguments` in an arrow function", item 1); so [`Instruction::This`] in the arrow's code,
    /// in an arrow in it and in eval code in it reads that value.
    ArrowClosure(u32),

    // ---- the operand stack ------------------------------------------------------------------
    /// `[a] -> []`.
    Pop,
    /// `[a] -> [a, a]`.
    Dup,
    /// `[a, b] -> [a, b, a, b]`.
    Dup2,
    /// `[x1 .. xn, a] -> [a, x1 .. xn]`: the top value moves below the `n` values under it.
    Bury(u32),
    /// `[x1 .. xn, a] -> [a]`: the `n` values below the top are dropped.
    Nip(u32),

    // ---- bindings -------------------------------------------------------------------------
    /// `[] -> [v]`: local slot `n`.
    GetLocal(u32),
    /// `[v] -> [v]`: stores into local slot `n`.
    SetLocal(u32),
    /// `[] -> [v]`: slot `slot` of the environment `hops` outward from the LexicalEnvironment.
    GetEnv {
        hops: u32,
        slot: u32,
    },
    /// `[v] -> [v]`: stores into slot `slot` of the environment `hops` outward.
    SetEnv {
        hops: u32,
        slot: u32,
    },
    /// `[v] -> [v]`: SetMutableBinding of the immutable binding named `strings[k]`
    /// (`#sec-declarative-environment-records-setmutablebinding-n-v-s`, step 5): in strict code a
    /// `TypeError`, otherwise nothing.
    AssignImmutable(u32),
    /// `[] -> [v]`: ResolveBinding of `strings[k]` in the global environment and GetValue
    /// (`#sec-resolvebinding`, `#sec-getvalue`): a `ReferenceError` when it is unresolvable.
    GetGlobal(u32),
    /// `[] -> [ref]`: ResolveBinding of `strings[k]` in the global environment, a Reference
    /// Record whose [[Base]] is the global environment or unresolvable.
    ResolveGlobal(u32),
    /// `[ref] -> [ref, v]`: GetValue of the reference (`#sec-getvalue`).
    GetGlobalRef(u32),
    /// `[ref, v] -> [v]`: PutValue (`#sec-putvalue`): to an unresolvable reference a
    /// `ReferenceError` in strict code, a Set on the global object otherwise.
    PutGlobalRef(u32),
    /// `[] -> [s]`: `typeof` of `strings[k]` resolved in the global environment;
    /// `"undefined"` when it is unresolvable (`#sec-typeof-operator`).
    TypeofGlobal(u32),
    /// `[] -> [b]`: `delete` of `strings[k]` resolved in the global environment: `true` when it
    /// is unresolvable, DeleteBinding of the global environment otherwise
    /// (`#sec-delete-operator-runtime-semantics-evaluation`).
    DeleteGlobal(u32),

    // ---- names looked up at run time (decision 3, "Names") -------------------------------------
    /// `[] -> [v]`: ResolveBinding of `strings[k]` through the Environment Records from the
    /// LexicalEnvironment outward (`#sec-resolvebinding`, `#sec-getidentifierreference`), then
    /// GetValue (`#sec-getvalue`): a `ReferenceError` when it is unresolvable. A record whose slots
    /// have no names binds no name a lookup asks for (the compiler names every record a lookup
    /// can find a binding in); the lookup charges one step per record it visits.
    GetName(u32),
    /// `[] -> [ref]`: ResolveBinding of `strings[k]` as [`Instruction::GetName`] does: a Reference
    /// Record whose [[Base]] is the record that binds the name, or unresolvable.
    ResolveName(u32),
    /// `[ref] -> [ref, v]`: GetValue of the reference: GetBindingValue of its record
    /// (`#sec-object-environment-records-getbindingvalue-n-s` for an Object Environment Record).
    GetNameRef(u32),
    /// `[ref, v] -> [v]`: PutValue (`#sec-putvalue`): to an unresolvable reference a
    /// `ReferenceError` in strict code and a Set on the global object otherwise, to a resolved
    /// one SetMutableBinding of its record (an immutable binding: a `TypeError` in strict code).
    PutNameRef(u32),
    /// `[] -> [s]`: `typeof` of `strings[k]` looked up by name; `"undefined"` when it is
    /// unresolvable (`#sec-typeof-operator-runtime-semantics-evaluation`).
    TypeofName(u32),
    /// `[] -> [b]`: `delete` of `strings[k]` looked up by name: `true` when it is unresolvable,
    /// DeleteBinding of its record otherwise (`#sec-delete-operator-runtime-semantics-evaluation`).
    DeleteName(u32),
    /// `[] -> [this, f]`: the function of a call whose callee is `strings[k]` looked up by name
    /// and its this value (`#sec-evaluatecall`): WithBaseObject of the record that binds it, the
    /// binding object of an Object Environment Record of a `with` statement, otherwise
    /// `undefined`.
    GetNameForCall(u32),

    // ---- properties -----------------------------------------------------------------------
    /// `[o] -> [v]`: GetValue of the property reference `o.strings[k]`.
    GetNamed(u32),
    /// `[o, v] -> [v]`: PutValue of the property reference `o.strings[k]`; a failed [[Set]] is a
    /// `TypeError` in strict code.
    PutNamed(u32),
    /// `[o, key] -> [v]`: GetValue of the property reference `o[key]`: ToObject of `o`, then
    /// ToPropertyKey of `key` (`#sec-getvalue`).
    GetIndex,
    /// `[o, key, v] -> [v]`: PutValue of `o[key]`: ToObject of `o`, then ToPropertyKey of `key`
    /// (`#sec-putvalue`; `#sec-evaluate-property-access-with-expression-key`: the key is converted
    /// after the value is evaluated).
    PutIndex,
    /// `[o, key] -> [o, key']`: the first steps of GetValue of `o[key]` before the reference is
    /// read and written by a compound assignment or an update: a `TypeError` when `o` is
    /// `undefined` or `null` (ToObject), then `key'` is ToPropertyKey of `key`, so the key is
    /// converted once.
    PropertyKey,
    /// `[o] -> [b]`: `delete o.strings[k]` (`#sec-delete-operator-runtime-semantics-evaluation`);
    /// `false` from [[Delete]] is a `TypeError` in strict code.
    DeleteNamed(u32),
    /// `[o, key] -> [b]`: `delete o[key]`.
    DeleteIndex,

    // ---- literals -------------------------------------------------------------------------
    /// `[] -> [o]`: OrdinaryObjectCreate(%Object.prototype%).
    NewObject,
    /// `[] -> [a]`: ArrayCreate(0).
    NewArray,
    /// `[o, v] -> [o]`: CreateDataPropertyOrThrow(o, `strings[k]`, v)
    /// (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`).
    DefineField(u32),
    /// `[o, f] -> [o]`: the getter `f` as property `strings[k]`, enumerable and configurable
    /// (`#sec-runtime-semantics-methoddefinitionevaluation`).
    DefineGetter(u32),
    /// `[o, f] -> [o]`: the setter `f` as property `strings[k]`.
    DefineSetter(u32),
    /// `[o, f] -> [o]`: the plain method `f` (a closure of kind method) with `o` as its
    /// [[HomeObject]] (MakeMethod), then a data property `strings[k]` of `o` with value `f`,
    /// writable, enumerable and configurable (`#sec-runtime-semantics-definemethod`,
    /// `#sec-definemethodproperty`).
    DefineMethod(u32),
    /// `[o, v] -> [o]`: `__proto__: v` in an object literal: o.[[SetPrototypeOf]](v) when `v` is
    /// an Object or `null`, nothing otherwise.
    InitProto,
    /// `[a, v] -> [a]`: CreateDataPropertyOrThrow(a, ToString(n), v)
    /// (`#sec-runtime-semantics-arrayaccumulation`).
    DefineIndex(u32),
    /// `[a] -> [a]`: Set(a, "length", n, true), for a run of elisions.
    ArrayLength(u32),
    /// `[v] -> [k]`: ToPropertyKey (`#sec-topropertykey`) of a ComputedPropertyName, at once after
    /// its expression (`#sec-object-initializer-runtime-semantics-evaluation`,
    /// ComputedPropertyName): `k` is a String or a Symbol.
    ToPropertyKey,
    /// `[o, k, v] -> [o]`: CreateDataPropertyOrThrow(o, k, v) for `[key]: value`
    /// (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`).
    DefineComputedField,
    /// `[o, k, f] -> [o]`: SetFunctionName(f, k, "get"), then the getter `f` as property `k`,
    /// enumerable and configurable (`#sec-runtime-semantics-methoddefinitionevaluation`).
    DefineComputedGetter,
    /// `[o, k, f] -> [o]`: SetFunctionName(f, k, "set"), then the setter `f` as property `k`.
    DefineComputedSetter,
    /// `[o, k, f] -> [o]`: MakeMethod(f, o), SetFunctionName(f, k), then a data property `k` of
    /// `o` with value `f`, writable, enumerable and configurable
    /// (`#sec-runtime-semantics-definemethod`, `#sec-definemethodproperty`).
    DefineComputedMethod,
    /// `[k, f] -> [k, f]`: SetFunctionName(f, k) (`#sec-setfunctionname`) of the anonymous
    /// function definition `f`, the value of `[key]: value` (NamedEvaluation with the key,
    /// `#sec-object-initializer-runtime-semantics-propertydefinitionevaluation` step 5).
    SetFunctionName,
    /// `[o, v] -> [o]`: CopyDataProperties(o, v, an empty list) (`#sec-copydataproperties`) of
    /// `PropertyDefinition : ... AssignmentExpression`: nothing for `undefined` and `null`.
    CopyDataProperties,
    /// `[a, v] -> [a]`: ArrayAccumulation of a SpreadElement
    /// (`#sec-runtime-semantics-arrayaccumulation`): GetIterator(v, sync), then each value of
    /// IteratorStepValue until the iterator is done, CreateDataPropertyOrThrow(a, ToString(len),
    /// value) at the length `len` of `a`; one step per value, the record held on the kept stack
    /// while `next` runs, the iterator never closed (`docs/design/engine.md`, decision 3, "The
    /// iterator of `for`-`of`, of an array pattern and of a spread", item 5). The same builds
    /// the argument list of a call with a spread.
    SpreadArray,
    /// `[a, v] -> [a]`: CreateDataPropertyOrThrow(a, ToString(len), v) at the length `len` of
    /// `a`: an element after a spread element, whose index only the run knows.
    ArrayAppend,
    /// `[a] -> [a]`: Set(a, "length", len + n, true) of the length `len` of `a`: a run of `n`
    /// elisions after a spread element.
    ArrayGrow(u32),

    // ---- operators ------------------------------------------------------------------------
    /// `[v] -> [s]`: `typeof` (`#sec-typeof-operator`).
    Typeof,
    /// `[v] -> [n]`: unary `+`, ToNumber (`#sec-unary-plus-operator`).
    ToNumber,
    /// `[v] -> [n]`: unary `-` (`#sec-unary-minus-operator`).
    Negate,
    /// `[v] -> [n]`: `~` (`#sec-bitwise-not-operator`).
    BitNot,
    /// `[v] -> [b]`: `!` (`#sec-logical-not-operator`).
    Not,
    /// `[v] -> [n]`: ToNumeric (`#sec-tonumeric`).
    ToNumeric,
    /// `[n] -> [n + 1]`, of a numeric value (`#sec-postfix-increment-operator`).
    Increment,
    /// `[n] -> [n - 1]`, of a numeric value.
    Decrement,
    /// `[a, b] -> [a * b]`, ApplyStringOrNumericBinaryOperator
    /// (`#sec-applystringornumericbinaryoperator`); likewise `Div` to `UShr` and `BitAnd` to
    /// `BitOr`.
    Mul,
    Div,
    Rem,
    Add,
    Sub,
    Shl,
    Shr,
    UShr,
    /// `[a, b] -> [a < b]` (`#sec-relational-operators-runtime-semantics-evaluation`).
    Lt,
    Gt,
    LtEq,
    GtEq,
    /// `[a, b] -> [r]`: InstanceofOperator (`#sec-instanceofoperator`).
    InstanceOf,
    /// `[a, b] -> [r]`: `a in b`, a `TypeError` when `b` is not an Object.
    In,
    /// `[a, b] -> [r]`: IsLooselyEqual (`#sec-islooselyequal`).
    Eq,
    NotEq,
    /// `[a, b] -> [r]`: IsStrictlyEqual (`#sec-isstrictlyequal`).
    StrictEq,
    StrictNotEq,
    BitAnd,
    BitXor,
    BitOr,

    // ---- calls ----------------------------------------------------------------------------
    /// `[this, f, a1 .. an] -> [v]`: EvaluateCall (`#sec-evaluatecall`): a `TypeError` when `f`
    /// is not callable, then Call(f, this, «a1 .. an»).
    Call(u32),
    /// `[this, f, a1 .. an] -> [v]`: a call that scope analysis marked as a direct `eval`
    /// (`#sec-function-calls-runtime-semantics-evaluation`, step 6): when `f` is `%eval%` of the
    /// current realm (SameValue), PerformEval (`#sec-performeval`) of `a1`, or `undefined` with
    /// no argument, with the strictness of the calling code and `direct`, the eval code running
    /// in a frame of its own with the caller's `this`, LexicalEnvironment and
    /// VariableEnvironment; otherwise `Call(n)`.
    CallEval(u32),
    /// `[f, a1 .. an] -> [v]`: EvaluateNew (`#sec-evaluatenew`): a `TypeError` when `f` is not a
    /// constructor, then Construct(f, «a1 .. an»).
    New(u32),
    /// `[this, f, args] -> [v]`: `Call(n)` with the elements of the Array `args` as the
    /// arguments, the list ArgumentListEvaluation of `Arguments` with a spread element made
    /// (`#sec-runtime-semantics-argumentlistevaluation`), built by [`Instruction::NewArray`],
    /// [`Instruction::ArrayAppend`] and [`Instruction::SpreadArray`].
    CallSpread,
    /// `[this, f, args] -> [v]`: `CallEval(n)` with the elements of `args`: a direct `eval` of
    /// its first element, or `undefined` when it has none.
    CallEvalSpread,
    /// `[f, args] -> [v]`: `New(n)` with the elements of `args`.
    NewSpread,

    // ---- control --------------------------------------------------------------------------
    /// Continues at instruction `t`, which is after this one.
    Jump(u32),
    /// `[v] -> []`: continues at `t`, after this one, when ToBoolean(v) is true.
    JumpIfTrue(u32),
    /// `[v] -> []`: continues at `t`, after this one, when ToBoolean(v) is false.
    JumpIfFalse(u32),
    /// Continues at instruction `t`, which is this one or before it. The step budget and the
    /// interrupt are checked here (decision 3).
    LoopJump(u32),
    /// `[d, v] -> [d]` or `[]`: when IsStrictlyEqual(d, v) is true both are dropped and the run
    /// continues at `t`, after this one (`#sec-runtime-semantics-caseblockevaluation`).
    CaseJump(u32),
    /// `[v] -> [e]`: ForIn/OfHeadEvaluation for `enumerate` (`#sec-runtime-semantics-forinofheadevaluation`):
    /// for `undefined` or `null` an enumerator that yields nothing (the statement then completes
    /// like the break completion the clause returns), otherwise EnumerateObjectProperties of
    /// ToObject(v) (`#sec-enumerate-object-properties`).
    ForInStart,
    /// `[e] -> [e, key]`, or `[e]` and a jump to `t`, after this one, when the enumerator is done.
    ForInNext(u32),
    /// `[v] -> [i]`: ForIn/OfHeadEvaluation for `iterate`
    /// (`#sec-runtime-semantics-forinofheadevaluation`, step 7.b): the Iterator Record of
    /// GetIterator(v, sync) (`#sec-getiterator`), a `TypeError` when `v` is not iterable.
    GetIterator,
    /// `[i] -> [i, v]`, or `[i]` and a jump to `t`, after this one, when the iterator is done:
    /// IteratorStepValue (`#sec-iteratorstepvalue`), a call of [[NextMethod]]. A throw of it,
    /// a result that is not an object, and a throw while "done" or "value" is read set the
    /// record's [[Done]].
    IteratorStep(u32),
    /// `[i, v, tag] -> [i, v, tag]`: IteratorClose (`#sec-iteratorclose`) of the record under a
    /// completion record, unless its [[Done]] is set, which it then sets: with the throw
    /// completion for [`TAG_THROW`], whose throw then stands, and with a normal completion for
    /// every other tag, whose completion a throw of `return`, or a result of it that is no
    /// object, replaces.
    IteratorClose,
    /// `[v] -> `: throws `v`.
    Throw,
    /// Throws a `SyntaxError` of the current realm with the message `strings[k]`: evaluating a
    /// part of the language that its slice does not have yet (design, "A part before its slice").
    ThrowSyntaxError(u32),
    /// `[v] -> `: returns `v` from the frame.
    Return,
    /// A new declarative environment of `n` slots, whose outer is the LexicalEnvironment,
    /// becomes the LexicalEnvironment (`#sec-runtime-semantics-catchclauseevaluation`).
    PushEnv(u32),
    /// A new declarative environment whose slots are named by `scopes[t]` of the running code
    /// (a [`Code`] or [`FunctionTemplate`] table), one slot per name, whose outer is the
    /// LexicalEnvironment, becomes the LexicalEnvironment: the environment of a `catch` clause
    /// that encloses a `with` statement, whose binding a lookup by name can reach.
    PushNamedEnv(u32),
    /// `[v] -> []`: the start of a `with` statement
    /// (`#sec-with-statement-runtime-semantics-evaluation`): ToObject(v), a `TypeError` for
    /// `undefined` and `null`, then NewObjectEnvironment(obj, true, LexicalEnvironment)
    /// (`#sec-newobjectenvironment`) becomes the LexicalEnvironment. It counts as an environment
    /// of [`Instruction::PopEnv`] and of a handler's `envs`, as `PushEnv` does.
    PushWith,
    /// The outer environment of the LexicalEnvironment becomes the LexicalEnvironment.
    PopEnv,
    /// `[] -> [tag]`: the tag of a completion record.
    Tag(u32),
    /// `[v, tag] -> [v]` and a jump to `target`, after this one, when the tag is `tag`;
    /// otherwise nothing.
    JumpIfTag {
        tag: u32,
        target: u32,
    },
    /// `[v, tag] -> []`: the end of a `finally` block for a normal completion (continue with the
    /// next instruction) or a throw completion (throw `v`).
    EndFinally,
    /// `[] -> [v]`: the completion value register of a script.
    LoadCompletion,
    /// `[v] -> []`: stores into the completion value register of a script.
    StoreCompletion,

    // ---- parameters of a list that is not simple ----------------------------------------
    /// `[] -> [v]`: the argument the call kept at local slot `n`
    /// ([`FunctionTemplate::binds_parameters`]), `undefined` when the call passed none there
    /// (IteratorBindingInitialization of a `FormalParameter` with the list iterator of the
    /// arguments, `#sec-runtime-semantics-iteratorbindinginitialization`).
    Argument(u32),
    /// `[v] -> [v]` and a jump to `t`, after this one, when `v` is not `undefined`; otherwise
    /// `[v] -> []`: the test before the `Initializer` of a parameter.
    JumpIfNotUndefined(u32),
    /// `[] -> [a]`: a rest parameter, CreateArrayFromList (`#sec-createarrayfromlist`) of the
    /// arguments the call kept from local slot `n` on, a step per element and the Array charged
    /// to the memory budget.
    RestArguments(u32),
    /// A new declarative environment whose slots are named by `scopes[t]`, whose outer is the
    /// LexicalEnvironment, becomes the LexicalEnvironment and the VariableEnvironment: the
    /// record of the `var` names of the body of a function whose parameters have an
    /// initializer (`#sec-functiondeclarationinstantiation`, step 28), where a direct `eval` of
    /// the body adds its `var` names. It counts as an environment of a handler's `envs`.
    PushVarEnv(u32),

    // ---- lexical bindings (`let` and `const`) --------------------------------------------------
    /// Local slot `n` becomes uninitialized: the entry of the scope of a `let` or `const`
    /// binding that some access checks (`#sec-blockdeclarationinstantiation`).
    Uninitialize(u32),
    /// A new declarative environment of one uninitialized slot per name of `scopes[t]` of the
    /// running code, which names its slots, whose outer is the LexicalEnvironment, becomes the
    /// LexicalEnvironment: BlockDeclarationInstantiation of a block, a case block, a `for` or
    /// for-in head or a function body whose bindings a closure or a lookup by name reaches. It
    /// counts as an environment of [`Instruction::PopEnv`] and of a handler's `envs`.
    PushLexicalEnv(u32),
    /// CreatePerIterationEnvironment (`#sec-createperiterationenvironment`): a new declarative
    /// environment with the names of the LexicalEnvironment's slots and a copy of their values,
    /// whose outer is the LexicalEnvironment's outer, replaces the LexicalEnvironment. One step
    /// per slot copied.
    CopyEnv,
    /// `[] -> [v]`: local slot `slot`, a `ReferenceError` naming `strings[name]` when it is
    /// uninitialized (`#sec-declarative-environment-records-getbindingvalue-n-s`, step 2).
    GetLocalChecked {
        slot: u32,
        name: u32,
    },
    /// `[v] -> [v]`: stores into local slot `slot`, a `ReferenceError` naming `strings[name]`
    /// when it is uninitialized (`#sec-declarative-environment-records-setmutablebinding-n-v-s`,
    /// step 3).
    SetLocalChecked {
        slot: u32,
        name: u32,
    },
    /// `[] -> [v]`: slot `slot` of the environment `hops` outward, a `ReferenceError` naming the
    /// slot's name when it is uninitialized.
    GetEnvChecked {
        hops: u32,
        slot: u32,
    },
    /// `[v] -> [v]`: stores into slot `slot` of the environment `hops` outward, a
    /// `ReferenceError` naming the slot's name when it is uninitialized.
    SetEnvChecked {
        hops: u32,
        slot: u32,
    },
    /// `[v] -> [v]`: SetMutableBinding of the `const` binding in local slot `slot`, named
    /// `strings[name]`: a `ReferenceError` when it is uninitialized, otherwise a `TypeError`, in
    /// any code (`#sec-declarative-environment-records-setmutablebinding-n-v-s`, steps 3 and 5;
    /// CreateImmutableBinding(N, true)).
    AssignConstLocal {
        slot: u32,
        name: u32,
    },
    /// `[v] -> [v]`: the same for the `const` binding in slot `slot` of the environment `hops`
    /// outward, named by the slot's name.
    AssignConstEnv {
        hops: u32,
        slot: u32,
    },
    /// `[v] -> [v]`: InitializeBinding of the lexical binding `strings[k]` of the global
    /// environment's [[DeclarativeRecord]] (`#sec-declarative-environment-records-initializebinding-n-v`),
    /// which GlobalDeclarationInstantiation created (step 15): the evaluation of a `let` or
    /// `const` declaration of a script.
    InitGlobal(u32),

    // ---- object patterns ------------------------------------------------------------------
    /// `[v] -> [v]`: RequireObjectCoercible (`#sec-requireobjectcoercible`) of the value an
    /// object pattern destructures, a `TypeError` for `undefined` and `null`
    /// (`#sec-runtime-semantics-bindinginitialization`,
    /// `#sec-runtime-semantics-destructuringassignmentevaluation`).
    RequireObjectCoercible,
    /// `[v, r1..rn] -> [v, r1..rn, x]` with `depth` n: `x` is GetV(`v`, `strings[name]`)
    /// (`#sec-getv`), the read of one property of an object pattern, after the reference of its
    /// target (`#sec-runtime-semantics-keyedbindinginitialization`,
    /// `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`).
    PatternGet {
        depth: u32,
        name: u32,
    },
    /// `[v, k1..kj, k, r1..rn] -> [v, k1..kj, k, r1..rn, x]` with `depth` n and `kept` j: `x` is
    /// GetV(`v`, `k`), `k` a property key, the read of a property whose key is computed or whose
    /// pattern has a rest property; the keys stay. The keys of the properties before a rest
    /// property are its excluded names
    /// (`#sec-destructuring-binding-patterns-runtime-semantics-propertybindinginitialization`,
    /// `#sec-runtime-semantics-propertydestructuringassignmentevaluation`).
    PatternGetKey {
        depth: u32,
        kept: u32,
    },
    /// `[v, k1..kj, r1..rn] -> [v, k1..kj, r1..rn, rest]` with `depth` n and `kept` j: `rest` is
    /// a new ordinary object that CopyDataProperties(rest, v, the list k1..kj) filled, the value of
    /// a rest property `...target` after the reference of its target
    /// (`#sec-destructuring-binding-patterns-runtime-semantics-restbindinginitialization`,
    /// `#sec-runtime-semantics-restdestructuringassignmentevaluation`).
    ObjectRest {
        depth: u32,
        kept: u32,
    },

    // ---- array patterns -------------------------------------------------------------------
    /// `[i, r1..rn] -> [i, r1..rn, x]` with `depth` n: the next value of the Iterator Record
    /// `i` of an array pattern, after the reference of the element's target: IteratorStepValue
    /// (`#sec-iteratorstepvalue`) unless [[Done]] is set, `x` its value, or `undefined` once the
    /// iterator is done (`#sec-runtime-semantics-iteratorbindinginitialization`,
    /// `#sec-runtime-semantics-iteratordestructuringassignmentevaluation`). A throw of `next`,
    /// a result that is no object, done, and a throw while "done" or "value" is read set
    /// [[Done]].
    PatternStep(u32),
    /// `[i] -> [i]`: an elision of an array pattern, IteratorStep (`#sec-iteratorstep`) unless
    /// [[Done]] is set, its value not read.
    PatternSkip,
    /// `[i, r1..rn] -> [i, r1..rn, a]` with `depth` n: the rest element of an array pattern, a
    /// new Array of the values IteratorStepValue answers until the iterator is done, one step
    /// per value; [[Done]] is set after it.
    PatternRest(u32),

    // ---- templates ------------------------------------------------------------------------
    /// `[v] -> [s]`: ToString (`#sec-tostring`) of a substitution of a template literal
    /// (`#sec-template-literals-runtime-semantics-evaluation`): an object gives ToPrimitive with
    /// hint string, so its `toString` is asked first.
    ToString,
    /// `[] -> [t]`: GetTemplateObject (`#sec-gettemplateobject`) of tagged template site `k` of
    /// the code ([`Code::template_sites`]): the template object the record of the code holds
    /// for the site, made at its first evaluation, an argument of the tag's call
    /// (`#sec-tagged-templates-runtime-semantics-evaluation`).
    TemplateObject(u32),
}

impl Instruction {
    /// The instruction index a jump names; `None` for an instruction that does not jump.
    pub fn target(&self) -> Option<u32> {
        match *self {
            Instruction::Jump(t)
            | Instruction::JumpIfTrue(t)
            | Instruction::JumpIfFalse(t)
            | Instruction::LoopJump(t)
            | Instruction::CaseJump(t)
            | Instruction::ForInNext(t)
            | Instruction::IteratorStep(t)
            | Instruction::JumpIfTag { target: t, .. } => Some(t),
            Instruction::Undefined
            | Instruction::Null
            | Instruction::True
            | Instruction::False
            | Instruction::Number(_)
            | Instruction::String(_)
            | Instruction::This
            | Instruction::Closure(_)
            | Instruction::ArrowClosure(_)
            | Instruction::Pop
            | Instruction::Dup
            | Instruction::Dup2
            | Instruction::Bury(_)
            | Instruction::Nip(_)
            | Instruction::GetLocal(_)
            | Instruction::SetLocal(_)
            | Instruction::GetEnv { .. }
            | Instruction::SetEnv { .. }
            | Instruction::AssignImmutable(_)
            | Instruction::GetGlobal(_)
            | Instruction::ResolveGlobal(_)
            | Instruction::GetGlobalRef(_)
            | Instruction::PutGlobalRef(_)
            | Instruction::TypeofGlobal(_)
            | Instruction::DeleteGlobal(_)
            | Instruction::GetName(_)
            | Instruction::ResolveName(_)
            | Instruction::GetNameRef(_)
            | Instruction::PutNameRef(_)
            | Instruction::TypeofName(_)
            | Instruction::DeleteName(_)
            | Instruction::GetNameForCall(_)
            | Instruction::GetNamed(_)
            | Instruction::PutNamed(_)
            | Instruction::GetIndex
            | Instruction::PutIndex
            | Instruction::PropertyKey
            | Instruction::DeleteNamed(_)
            | Instruction::DeleteIndex
            | Instruction::NewObject
            | Instruction::NewArray
            | Instruction::DefineField(_)
            | Instruction::DefineGetter(_)
            | Instruction::DefineSetter(_)
            | Instruction::DefineMethod(_)
            | Instruction::InitProto
            | Instruction::DefineIndex(_)
            | Instruction::ArrayLength(_)
            | Instruction::Typeof
            | Instruction::ToNumber
            | Instruction::Negate
            | Instruction::BitNot
            | Instruction::Not
            | Instruction::ToNumeric
            | Instruction::Increment
            | Instruction::Decrement
            | Instruction::Mul
            | Instruction::Div
            | Instruction::Rem
            | Instruction::Add
            | Instruction::Sub
            | Instruction::Shl
            | Instruction::Shr
            | Instruction::UShr
            | Instruction::Lt
            | Instruction::Gt
            | Instruction::LtEq
            | Instruction::GtEq
            | Instruction::InstanceOf
            | Instruction::In
            | Instruction::Eq
            | Instruction::NotEq
            | Instruction::StrictEq
            | Instruction::StrictNotEq
            | Instruction::BitAnd
            | Instruction::BitXor
            | Instruction::BitOr
            | Instruction::Call(_)
            | Instruction::CallEval(_)
            | Instruction::New(_)
            | Instruction::ForInStart
            | Instruction::GetIterator
            | Instruction::IteratorClose
            | Instruction::Throw
            | Instruction::ThrowSyntaxError(_)
            | Instruction::Return
            | Instruction::PushEnv(_)
            | Instruction::PushNamedEnv(_)
            | Instruction::PushWith
            | Instruction::PopEnv
            | Instruction::Tag(_)
            | Instruction::EndFinally
            | Instruction::LoadCompletion
            | Instruction::StoreCompletion
            | Instruction::Uninitialize(_)
            | Instruction::PushLexicalEnv(_)
            | Instruction::CopyEnv
            | Instruction::GetLocalChecked { .. }
            | Instruction::SetLocalChecked { .. }
            | Instruction::GetEnvChecked { .. }
            | Instruction::SetEnvChecked { .. }
            | Instruction::AssignConstLocal { .. }
            | Instruction::AssignConstEnv { .. }
            | Instruction::InitGlobal(_) => None,
            Instruction::ToString => None,
            Instruction::TemplateObject(_) => None,
            Instruction::Argument(_)
            | Instruction::RestArguments(_)
            | Instruction::PushVarEnv(_) => None,
            Instruction::RequireObjectCoercible | Instruction::PatternGet { .. } => None,
            Instruction::ToPropertyKey
            | Instruction::DefineComputedField
            | Instruction::DefineComputedGetter
            | Instruction::DefineComputedSetter
            | Instruction::DefineComputedMethod
            | Instruction::SetFunctionName
            | Instruction::CopyDataProperties
            | Instruction::PatternGetKey { .. }
            | Instruction::ObjectRest { .. } => None,
            Instruction::SpreadArray
            | Instruction::ArrayAppend
            | Instruction::ArrayGrow(_)
            | Instruction::CallSpread
            | Instruction::CallEvalSpread
            | Instruction::NewSpread
            | Instruction::PatternStep(_)
            | Instruction::PatternSkip
            | Instruction::PatternRest(_) => None,
            Instruction::JumpIfNotUndefined(t) => Some(t),
        }
    }

    /// The same instruction with its jump target replaced; unchanged when it does not jump.
    pub(crate) fn with_target(self, t: u32) -> Instruction {
        match self {
            Instruction::Jump(_) => Instruction::Jump(t),
            Instruction::JumpIfTrue(_) => Instruction::JumpIfTrue(t),
            Instruction::JumpIfFalse(_) => Instruction::JumpIfFalse(t),
            Instruction::LoopJump(_) => Instruction::LoopJump(t),
            Instruction::CaseJump(_) => Instruction::CaseJump(t),
            Instruction::ForInNext(_) => Instruction::ForInNext(t),
            Instruction::IteratorStep(_) => Instruction::IteratorStep(t),
            Instruction::JumpIfTag { tag, .. } => Instruction::JumpIfTag { tag, target: t },
            Instruction::JumpIfNotUndefined(_) => Instruction::JumpIfNotUndefined(t),
            other => other,
        }
    }
}

/// Whether a handler catches or runs a `finally` block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandlerKind {
    /// The exception is pushed and the run continues at the target, the `catch` block.
    Catch,
    /// The throw completion record `[exception, TAG_THROW]` is pushed and the run continues at
    /// the target, the `finally` block.
    Finally,
    /// The handler of an array pattern, which an expression holds: the operand stack is dropped
    /// down to the topmost Iterator Record at `items` or above, the pattern's own, the one
    /// handler a temporary of an expression may stand under (the compiler counts the entries of
    /// statements alone); then as [`HandlerKind::Finally`]. Its target closes the record, drops
    /// it and throws again (`docs/design/engine.md`, decision 3, item 5 of "The iterator of
    /// `for`-`of`, of an array pattern and of a spread").
    Pattern,
}

/// An entry of a handler table: a throw from an instruction `i` with `start <= i < end` continues
/// at `target` (an empty try block gives an empty range). Entries are in the order they are tried: an inner try statement's before an outer
/// one's, and of one statement the catch before the finally. A termination
/// (`Terminated(...)`) enters no handler (decision 3).
///
/// Before it continues at `target` the interpreter drops the operand stack down to `items`
/// entries and the environments down to `envs` [`Instruction::PushEnv`]s made in this frame,
/// the state of the frame at the try statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Handler {
    pub kind: HandlerKind,
    pub start: u32,
    pub end: u32,
    pub target: u32,
    pub items: u32,
    pub envs: u32,
}

/// The name of a slot of an environment the code makes ([`Instruction::PushNamedEnv`],
/// [`Instruction::PushLexicalEnv`], the record of eval code): a `strings` index, and whether the
/// binding is a `let` or `const` one, and `const`. Four bytes, as the index alone was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotName(u32);

impl SlotName {
    const LEXICAL: u32 = 1 << 31;
    const CONSTANT: u32 = 1 << 30;
    const INDEX: u32 = SlotName::CONSTANT - 1;

    /// The slot name of a `var` binding or a `catch` parameter named `strings[name]`; `None`
    /// when the index needs the bits of the marks.
    pub fn plain(name: u32) -> Option<SlotName> {
        (name <= SlotName::INDEX).then_some(SlotName(name))
    }

    /// The slot name of a `let` binding, or with `constant` a `const` one.
    pub fn lexical(name: u32, constant: bool) -> Option<SlotName> {
        let marks = if constant {
            SlotName::LEXICAL | SlotName::CONSTANT
        } else {
            SlotName::LEXICAL
        };
        (name <= SlotName::INDEX).then_some(SlotName(name | marks))
    }

    /// The `strings` index of the name.
    pub fn name(self) -> u32 {
        self.0 & SlotName::INDEX
    }

    /// Whether the binding is a `let` or `const` one.
    pub fn is_lexical(self) -> bool {
        self.0 & SlotName::LEXICAL != 0
    }

    /// Whether the binding is a `const` one: immutable and strict.
    pub fn is_constant(self) -> bool {
        self.0 & SlotName::CONSTANT != 0
    }
}

/// A `let` or `const` name of the top level of a script or of eval code
/// ([`Code::lexical`]): its `strings` index, whether it is `const`, and, of eval code, where it
/// lives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LexicalName {
    pub name: u32,
    pub constant: bool,
    /// `None` for a script, whose lexical names are bindings of the global environment's
    /// [[DeclarativeRecord]]; for eval code a local slot or a slot of its record.
    pub slot: Option<Slot>,
}

/// Where a parameter lives: a local slot of the frame, or a slot of the function environment
/// the call creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Local(u32),
    Env(u32),
}

/// The arguments object a call of a function makes (`#sec-functiondeclarationinstantiation`,
/// steps 15 to 22): only for a function whose own code references `arguments` and that has no
/// parameter and declares no function of that name (`docs/design/engine.md`, decision 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArgumentsObject {
    /// CreateMappedArgumentsObject (`#sec-createmappedargumentsobject`) for a non-strict
    /// function whose parameter list is simple: every parameter then has a slot of the function
    /// environment, which the object's parameter map reads and writes. Otherwise, in strict code
    /// or for a list that is not simple (`#sec-functiondeclarationinstantiation`, step 22),
    /// CreateUnmappedArgumentsObject (`#sec-createunmappedargumentsobject`).
    pub mapped: bool,
    /// The slot of the binding `arguments`, which the call initializes to the object.
    pub slot: Slot,
}

/// The template of an ECMAScript function (`#sec-ecmascript-function-objects`): what
/// OrdinaryFunctionCreate and the call need, and its code.
///
/// A call binds `this` (OrdinaryCallBindThis; an arrow function takes the `this` its
/// [`Instruction::ArrowClosure`] kept instead), makes a frame of `locals` slots and, when
/// `captured` is not empty or `eval_environment` is set, a function environment of
/// `captured.len()` slots named by `captured` whose outer is the function's [[Environment]];
/// it stores the arguments object in its slot
/// when the template asks for one, and the arguments in `params` in order (a later parameter of
/// the same name overwrites an earlier one, `#sec-functiondeclarationinstantiation`, step 22), a
/// missing one as `undefined`, and runs `instructions`, which begin with the instantiation of
/// the function declarations (step 36).
///
/// For a parameter list that is not simple (`binds_parameters`) the call stores no argument:
/// each slot of `params` starts uninitialized, the call keeps its arguments in the frame, in
/// the local slots after `locals`, and `instructions` begin with the binding of each parameter
/// in order ([`Instruction::Argument`], [`Instruction::RestArguments`]), then, when a parameter
/// has an initializer, the record of the body's `var` names ([`Instruction::PushVarEnv`]),
/// then the function declarations.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionTemplate {
    pub kind: FunctionKind,
    /// The value SetFunctionName gives: the declared name, the name NamedEvaluation gives an
    /// anonymous function (`#sec-runtime-semantics-namedevaluation`), `get <key>` or
    /// `set <key>` for an accessor, or empty.
    pub name: Vec<u16>,
    /// The `length` property: ExpectedArgumentCount
    /// (`#sec-static-semantics-expectedargumentcount`), the parameters before the first one with
    /// an initializer or the rest parameter.
    pub length: u32,
    pub strict: bool,
    /// Where each parameter goes, in order; for a parameter that is a pattern, where each name it
    /// binds goes, in source order (a list with a pattern is not simple: the call sets them
    /// uninitialized, [`FunctionTemplate::binds_parameters`]).
    pub params: Vec<Slot>,
    /// Whether the code binds the parameters, from the arguments the call keeps: a list that is
    /// not simple (`#sec-static-semantics-issimpleparameterlist`).
    pub binds_parameters: bool,
    /// Whether the function environment's outer is a record of its own, empty, which is the
    /// VariableEnvironment while the parameters are bound: non-strict code with an initializer
    /// that calls `eval` directly, whose `var` names go there, outside the parameters'
    /// environment (`#sec-functiondeclarationinstantiation`, steps 19 and 20).
    pub parameter_environment: bool,
    pub locals: u32,
    /// The names of the slots of the function environment, as `strings` indices. A lookup by
    /// name reads them.
    pub captured: Vec<u32>,
    /// The arguments object a call makes, if any.
    pub arguments: Option<ArgumentsObject>,
    /// The names of the slots of each environment [`Instruction::PushNamedEnv`] and
    /// [`Instruction::PushLexicalEnv`] make.
    pub scopes: Vec<Vec<SlotName>>,
    /// A named function expression whose own name is referenced: [`Instruction::Closure`] gives
    /// it the environment holding that name.
    pub self_binding: bool,
    /// Whether a call makes a function environment though `captured` is empty: an arrow
    /// function whose own non-strict code holds a direct `eval` call and that binds no name.
    /// The eval code adds its `var` and function names to the VariableEnvironment of its
    /// caller (`#sec-evaldeclarationinstantiation`), which must be the arrow's own and not the
    /// environment around it. Every other function with such a call has a binding (its
    /// arguments object at least), so `captured` is not empty and the flag is false.
    pub eval_environment: bool,
    pub instructions: Vec<Instruction>,
    /// The source position of each instruction.
    pub positions: Vec<Span>,
    pub handlers: Vec<Handler>,
    /// The span of the function's source text, in code units of the source the parser read:
    /// "the source text matched by" its FunctionDeclaration, FunctionExpression or
    /// MethodDefinition (`#sec-runtime-semantics-instantiateordinaryfunctionobject`,
    /// `#sec-runtime-semantics-definemethod`), from its first token (`function`; `get` or `set`
    /// of an accessor; the property name of a plain method) to its closing brace. An engine that
    /// keeps the source with the code reads the function's [[SourceText]] out of it
    /// (`#sec-function.prototype.tostring`).
    pub source_text: Span,
}

/// The compiled script: its own code and every function template in it.
///
/// ScriptEvaluation (`#sec-runtime-semantics-scriptevaluation`) runs
/// GlobalDeclarationInstantiation (`#sec-globaldeclarationinstantiation`) with `var_names` and
/// `function_declarations`, then `instructions` in a frame of `locals` slots with the global
/// environment as LexicalEnvironment; the script ends with the [`Instruction::Return`] of its
/// completion value.
#[derive(Clone, Debug, PartialEq)]
pub struct Code {
    pub strict: bool,
    pub instructions: Vec<Instruction>,
    pub positions: Vec<Span>,
    pub handlers: Vec<Handler>,
    pub locals: u32,
    /// The names of the slots of each environment [`Instruction::PushNamedEnv`] and
    /// [`Instruction::PushLexicalEnv`] make in the script's own code, and of the record of eval
    /// code ([`Code::eval_scope`]).
    pub scopes: Vec<Vec<SlotName>>,
    /// String constants as UTF-16 code units, each once.
    pub strings: Vec<Vec<u16>>,
    /// Number constants, each bit pattern once.
    pub numbers: Vec<f64>,
    /// Every function of the script, numbered as the parser numbers them (in source order).
    pub functions: Vec<FunctionTemplate>,
    /// The `declaredVarNames` of GlobalDeclarationInstantiation, step 9: the names the script's
    /// `var` declarations bind that no function declaration of the script binds, in source order,
    /// as `strings` indices.
    pub var_names: Vec<u32>,
    /// The `functionsToInitialize` of GlobalDeclarationInstantiation, step 8: the last function
    /// declaration of each name, in source order, as `functions` indices.
    pub function_declarations: Vec<u32>,
    /// The LexicallyDeclaredNames of the top level, in source order: for a script the bindings
    /// GlobalDeclarationInstantiation checks (steps 3 and 4) and creates uninitialized (step 15);
    /// for eval code those of EvalDeclarationInstantiation (step 16), whose local slots start
    /// uninitialized.
    pub lexical: Vec<LexicalName>,
    /// Of eval code: the `scopes` entry that names the slots of the declarative record
    /// PerformEval makes for its lexical bindings (`#sec-performeval`, steps 17 to 19), when one
    /// of them lives there; `None` otherwise.
    pub eval_scope: Option<u32>,
    /// The site table: each tagged template of the code, its functions' included, numbered as
    /// the compile step reaches it ([`Instruction::TemplateObject`] names the number), with its
    /// template strings (`docs/design/engine.md`, decision 4, "Template objects", item 1).
    pub template_sites: Vec<TemplateSite>,
    /// The bytes the compile step's ledger charged for the code's vectors, by capacity, and
    /// still holds: the vectors above, the inner ones and those of every template. The owner of
    /// the code takes them over (the engine's script record owns them) or credits them when it
    /// drops the code.
    pub charged: usize,
}

/// A tagged template site of a [`Code`]: per template string in source order its TV as a
/// `strings` index, `None` where it is undefined (a NotEscapeSequence), and its TRV as a
/// `strings` index (`#sec-static-semantics-templatestrings`).
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateSite {
    pub cooked: Vec<Option<u32>>,
    pub raw: Vec<u32>,
}

/// The bytes of a vector's buffer by its capacity.
fn buffer<T>(v: &Vec<T>) -> usize {
    v.capacity().saturating_mul(size_of::<T>())
}

/// The bytes of the names tables of `scopes` by capacity.
fn scopes_bytes(scopes: &Vec<Vec<SlotName>>) -> usize {
    scopes.iter().fold(buffer(scopes), |sum, names| {
        sum.saturating_add(buffer(names))
    })
}

impl FunctionTemplate {
    /// The bytes the template's vectors own, by capacity.
    pub fn capacity_bytes(&self) -> usize {
        [
            buffer(&self.name),
            buffer(&self.params),
            buffer(&self.captured),
            scopes_bytes(&self.scopes),
            buffer(&self.instructions),
            buffer(&self.positions),
            buffer(&self.handlers),
        ]
        .iter()
        .fold(0usize, |sum, bytes| sum.saturating_add(*bytes))
    }
}

impl Code {
    /// The bytes the code's vectors own, by capacity: its own, the constants' and every
    /// template's. What [`Code::charged`] holds for code from a charged entry.
    pub fn capacity_bytes(&self) -> usize {
        let strings = self.strings.iter().fold(buffer(&self.strings), |sum, s| {
            sum.saturating_add(buffer(s))
        });
        let functions = self
            .functions
            .iter()
            .fold(buffer(&self.functions), |sum, f| {
                sum.saturating_add(f.capacity_bytes())
            });
        [
            buffer(&self.instructions),
            buffer(&self.positions),
            buffer(&self.handlers),
            scopes_bytes(&self.scopes),
            strings,
            buffer(&self.numbers),
            functions,
            buffer(&self.var_names),
            buffer(&self.function_declarations),
            buffer(&self.lexical),
            self.template_sites
                .iter()
                .fold(buffer(&self.template_sites), |sum, site| {
                    sum.saturating_add(buffer(&site.cooked))
                        .saturating_add(buffer(&site.raw))
                }),
        ]
        .iter()
        .fold(0usize, |sum, bytes| sum.saturating_add(*bytes))
    }
}
