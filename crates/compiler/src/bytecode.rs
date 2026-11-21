//! The bytecode of swell (`docs/design/engine.md`, decision 3): a stack machine whose every
//! ECMAScript frame lives on the engine's own frame stack.
//!
//! # The machine the instructions run on
//!
//! A frame has an instruction index, an operand stack, `locals` slots (each starts as
//! `undefined`), the running execution context's LexicalEnvironment (an Environment Record,
//! `#sec-environment-records`), its handler table and, for a script, a completion value register
//! that starts as `undefined`. The instructions of one template run in that frame; a jump names
//! the index of an instruction of the same template.
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
//! Besides language values the stack holds four kinds of engine entries, each produced and
//! consumed by instructions of this set only: the reference of a global name
//! ([`Instruction::ResolveGlobal`]), the reference of a name looked up at run time
//! ([`Instruction::ResolveName`]), the enumerator of a for-in statement
//! ([`Instruction::ForInStart`]) and the tag of a completion record ([`Instruction::Tag`]).
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
    /// bound by OrdinaryCallBindThis (`#sec-ordinarycallbindthis`).
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
    /// `[f, a1 .. an] -> [v]`: EvaluateNew (`#sec-evaluatenew`): a `TypeError` when `f` is not a
    /// constructor, then Construct(f, «a1 .. an»).
    New(u32),

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
            | Instruction::JumpIfTag { target: t, .. } => Some(t),
            Instruction::Undefined
            | Instruction::Null
            | Instruction::True
            | Instruction::False
            | Instruction::Number(_)
            | Instruction::String(_)
            | Instruction::This
            | Instruction::Closure(_)
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
            | Instruction::New(_)
            | Instruction::ForInStart
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
            | Instruction::StoreCompletion => None,
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
            Instruction::JumpIfTag { tag, .. } => Instruction::JumpIfTag { tag, target: t },
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
    /// function (whose parameter list is simple in this slice): every parameter then has a slot
    /// of the function environment, which the object's parameter map reads and writes. Otherwise
    /// CreateUnmappedArgumentsObject (`#sec-createunmappedargumentsobject`).
    pub mapped: bool,
    /// The slot of the binding `arguments`, which the call initializes to the object.
    pub slot: Slot,
}

/// The template of an ECMAScript function (`#sec-ecmascript-function-objects`): what
/// OrdinaryFunctionCreate and the call need, and its code.
///
/// A call binds `this` (OrdinaryCallBindThis), makes a frame of `locals` slots and, when
/// `captured` is not empty, a function environment of `captured.len()` slots named by `captured`
/// whose outer is the function's [[Environment]]; it stores the arguments object in its slot
/// when the template asks for one, and the arguments in `params` in order (a later parameter of
/// the same name overwrites an earlier one, `#sec-functiondeclarationinstantiation`, step 22), a
/// missing one as `undefined`, and runs `instructions`, which begin with the instantiation of
/// the function declarations (step 36).
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionTemplate {
    pub kind: FunctionKind,
    /// The value SetFunctionName gives: the declared name, the name NamedEvaluation gives an
    /// anonymous function (`#sec-runtime-semantics-namedevaluation`), `get <key>` or
    /// `set <key>` for an accessor, or empty.
    pub name: Vec<u16>,
    /// The `length` property: the number of parameters (ExpectedArgumentCount).
    pub length: u32,
    pub strict: bool,
    /// Where each parameter goes, in order.
    pub params: Vec<Slot>,
    pub locals: u32,
    /// The names of the slots of the function environment, as `strings` indices. A lookup by
    /// name reads them.
    pub captured: Vec<u32>,
    /// The arguments object a call makes, if any.
    pub arguments: Option<ArgumentsObject>,
    /// The names of the slots of each environment [`Instruction::PushNamedEnv`] makes, as
    /// `strings` indices.
    pub scopes: Vec<Vec<u32>>,
    /// A named function expression whose own name is referenced: [`Instruction::Closure`] gives
    /// it the environment holding that name.
    pub self_binding: bool,
    pub instructions: Vec<Instruction>,
    /// The source position of each instruction.
    pub positions: Vec<Span>,
    pub handlers: Vec<Handler>,
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
    /// The names of the slots of each environment [`Instruction::PushNamedEnv`] makes in the
    /// script's own code, as `strings` indices.
    pub scopes: Vec<Vec<u32>>,
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
}
