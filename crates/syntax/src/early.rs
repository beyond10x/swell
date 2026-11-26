//! The early errors of a parsed script (`#sec-scripts-static-semantics-early-errors` and the
//! early errors of the productions in clauses 12 to 15 that slice 1 parses), strict ones
//! included, checked in one walk over the tree once every function's strictness is known:
//!
//! - `#sec-identifiers-static-semantics-early-errors`: in strict mode code `eval` and `arguments`
//!   are no `BindingIdentifier`, and `implements`, `interface`, `let`, `package`, `private`,
//!   `protected`, `public`, `static` and `yield` are no identifier;
//! - `#sec-static-semantics-assignmenttargettype` with `#sec-assignment-operators-static-semantics-early-errors`,
//!   `#sec-update-expressions-static-semantics-early-errors` and
//!   `#sec-for-in-and-for-of-statements-static-semantics-early-errors`: an assignment target is
//!   an identifier (not `eval` or `arguments` in strict mode code) or a property access. A call
//!   is not: swell does not take the normative optional "Runtime Errors for Function Call
//!   Assignment Targets";
//! - `#sec-delete-operator-static-semantics-early-errors`: `delete` of an identifier, in
//!   parentheses or not, in strict mode code;
//! - `#sec-parameter-lists-static-semantics-early-errors` through
//!   `#sec-function-definitions-static-semantics-early-errors`: duplicate parameters of a strict
//!   function or of a parameter list that is not simple, and the function's own name, its
//!   parameters and their initializers checked with its own strictness. A Use Strict Directive
//!   in the body of a function whose parameter list is not simple (for every kind of function,
//!   the arrows and methods included) is reported by the parser as it closes the function,
//!   where it knows the directive;
//! - `#sec-numeric-literals-early-errors` and `#sec-string-literals-early-errors`: legacy octal
//!   literals, `NonOctalDecimalIntegerLiteral`s, legacy octal escapes and `\8` and `\9` in strict
//!   mode code;
//! - `#sec-with-statement-static-semantics-early-errors`: `with` in strict mode code (the parser
//!   reports it, with [`WITH_IN_STRICT`], as it reads the statement);
//! - `#sec-break-statement-static-semantics-early-errors`,
//!   `#sec-continue-statement-static-semantics-early-errors` and
//!   `#sec-labelled-statements-static-semantics-early-errors`: `break` outside a loop or `switch`,
//!   `continue` outside a loop, a label that no enclosing statement has (ContainsUndefinedBreakTarget,
//!   ContainsUndefinedContinueTarget), a `continue` whose label is on no iteration statement, a
//!   label inside a statement with the same label (ContainsDuplicateLabels), none crossing a
//!   function; in strict mode code a label is no reserved word either;
//! - `#sec-object-initializer-static-semantics-early-errors`: `__proto__` twice as a
//!   `PropertyName : AssignmentExpression`, and a `CoverInitializedName` in an object literal
//!   that no pattern reads again;
//! - `#sec-destructuring-assignment-static-semantics-early-errors` and
//!   `#sec-destructuring-binding-patterns`: each target of an ObjectAssignmentPattern that is no
//!   nested pattern is a simple assignment target; the names a binding pattern binds are
//!   checked as those of its declaration, a parameter list or a `catch` parameter, and a pattern
//!   in a declaration has an initializer outside a for-in or for-of head;
//! - `#sec-method-definitions-static-semantics-early-errors`: the `UniqueFormalParameters` of a
//!   method, a generator, `async` or async generator method too, in any code;
//! - `#sec-arrow-function-definitions-static-semantics-early-errors`: the
//!   `ArrowFormalParameters` of an arrow function hold no name twice, in any code; a parameter
//!   that the body declares lexically, and `eval` and `arguments` as parameters in strict code,
//!   are checked as for every function;
//! - `#sec-generator-function-definitions-static-semantics-early-errors` and
//!   `#sec-arrow-function-definitions-static-semantics-early-errors`: the FormalParameters of a
//!   generator and the ArrowParameters of an arrow function contain no YieldExpression (Contains,
//!   which looks into no nested function); a generator's other early errors are those of every
//!   function above;
//! - `#sec-identifiers-static-semantics-early-errors`: `yield` is no BindingIdentifier,
//!   IdentifierReference or LabelIdentifier where the grammar's [Yield] is set (the parameters
//!   and the body of a generator and the parameters of an arrow function inside them, not the
//!   body of that arrow function), nor the name of a GeneratorExpression; the walk tracks
//!   [Yield] as it tracks strictness;
//! - `#sec-async-function-definitions-static-semantics-early-errors`,
//!   `#sec-async-arrow-function-definitions-static-semantics-early-errors` and
//!   `#sec-arrow-function-definitions-static-semantics-early-errors`: the FormalParameters of an
//!   `async` function and the ArrowParameters of an arrow function contain no AwaitExpression,
//!   and the head of an `async` arrow function no YieldExpression either; the other early errors
//!   of an `async` function or arrow function are those of every function or arrow function
//!   above;
//! - `#sec-identifiers-static-semantics-early-errors`: `await` is no BindingIdentifier,
//!   IdentifierReference or LabelIdentifier where the grammar's [Await] is set (the parameters
//!   and the body of an `async` function or arrow function, and the parameters of an arrow
//!   function inside them), nor the name of an AsyncFunctionExpression; the walk tracks [Await]
//!   beside [Yield];
//! - `#sec-async-generator-function-definitions-static-semantics-early-errors`: an async
//!   generator is both a generator and an `async` function, with [Yield] and [Await] set in its
//!   parameters and body and in the name of an AsyncGeneratorExpression, so the checks above
//!   apply to it with the messages of each: its FormalParameters contain no YieldExpression and
//!   no AwaitExpression, `yield` and `await` are no identifier inside it or as the name of the
//!   expression, and a Use Strict Directive with a list that is not simple, a parameter its body
//!   declares lexically, duplicate parameters and the redeclaration of an async generator
//!   declaration in a block or a `case` block are those of every function;
//! - `#sec-static-semantics-assignmenttargettype`: an AwaitExpression is no assignment target;
//! - `#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`:
//!   IsValidRegularExpressionLiteral, the flags and then ParsePattern of `swell-regexp` with
//!   them and the limit `parser_nesting`, charged to the meter of the parse ([`crate::regexp`]);
//!   an error of the pattern is an early error of the literal, and a part of the pattern grammar
//!   a later story of slice 5 brings is the refusal of the parse, the only error it answers
//!   (`docs/design/engine.md`, decision 7, "Refusals until the later stories of slice 5");
//! - `#sec-let-and-const-declarations-static-semantics-early-errors`: a `let` or `const`
//!   declaration binds no name `let` and no name twice, and a `const` one has an initializer
//!   outside a for-in or for-of head;
//! - the LexicallyDeclaredNames of a script, a function body, a block and a `case` block, and
//!   the names a `for`, for-in or for-of head declares with `let` or `const`, hold no duplicate
//!   and no name of the VarDeclaredNames around them (`#sec-scripts-static-semantics-early-errors`,
//!   `#sec-function-definitions-static-semantics-early-errors`,
//!   `#sec-block-static-semantics-early-errors`, `#sec-switch-statement-static-semantics-early-errors`,
//!   `#sec-for-statement-static-semantics-early-errors`,
//!   `#sec-for-in-and-for-of-statements-static-semantics-early-errors`); those of a function body
//!   no parameter name, those of a `catch` block not its parameter, and a `catch` parameter that
//!   is a pattern binds no name twice and none a `var` of its block binds
//!   (`#sec-try-statement-static-semantics-early-errors`).
//!
//! The lexical names in scope are a stack of frames, each made as the walk enters the statement
//! list (or the head) that declares them; a `var` or a top-level function declaration is checked
//! against the frames of its own function. The frame of a block, a `case` block and a `try`,
//! `catch` or `finally` block holds its function declarations beside its `let` and `const` ones
//! (LexicallyDeclaredNames, `#sec-block-static-semantics-early-errors`,
//! `#sec-switch-statement-static-semantics-early-errors`), in strict and non-strict code alike:
//! the duplicate declarations Annex B allows in non-strict code (B.3.2.4) wait for slice 18.
//! The errors are sorted into source order at the end.
//!
//! The parser reports the early errors it meets as it reads: a name written with an escape that
//! spells a reserved word ([`escaped_keyword`], `#sec-identifiers-static-semantics-early-errors`)
//! and `??` mixed with `&&` or `||` ([`COALESCE_MIXED`]).
//!
//! The walk does not recurse: its pending work is a stack on the heap, so the Rust stack it uses
//! is the same at any depth of the tree.

use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::ast::{
    Ast, BindingTarget, Child, ClassElement, ClassKey, DeclKind, Expr, ExprId, ForInTarget,
    ForInit, Function, FunctionId, FunctionKind, Name, NumberForm, NumberLiteral, PatternKind,
    PatternTarget, PropertyKey, PropertyValue, RegExpLiteral, Span, Stmt, StmtId, StringLiteral,
    SwitchCase, UnaryOp, VarDeclarator,
};
use crate::error::{MessageKind, SyntaxError, message_kind};
use crate::ledger::{HeapBytes, Meter, map_entry_bytes};
use crate::regexp::PatternBudget;

/// The message of a `CoverInitializedName` (`a = 1` in an object literal) in a literal that is
/// not read again as a pattern (`#sec-object-initializer-static-semantics-early-errors`).
pub(crate) const COVER_INITIALIZED_NAME: &str = "invalid shorthand property initializer";

/// The message of a rest property `... target` of an object pattern that is not its last
/// property, or that a comma follows: the grammar of ObjectBindingPattern and
/// ObjectAssignmentPattern has a rest property only last and without a comma
/// (`#sec-destructuring-binding-patterns`, `#sec-destructuring-assignment`).
pub(crate) const REST_PROPERTY_NOT_LAST: &str =
    "a rest property must be the last property of its pattern";

/// The message of a rest property `... target = value` of an object pattern: a
/// BindingRestProperty and an AssignmentRestProperty have no Initializer.
pub(crate) const REST_PROPERTY_INITIALIZER: &str = "a rest property may not have an initializer";

/// The message of a rest property whose target is an object or array literal not in
/// parentheses: a BindingRestProperty is `... BindingIdentifier`, and an AssignmentRestProperty
/// whose DestructuringAssignmentTarget is an ArrayLiteral or an ObjectLiteral is a SyntaxError
/// (`#sec-destructuring-assignment-static-semantics-early-errors`).
pub(crate) const REST_PROPERTY_PATTERN: &str = "the target of a rest property may not be a pattern";

/// The message of a rest element `... target` of an array pattern that is not its last element,
/// or that a comma follows: ArrayBindingPattern and ArrayAssignmentPattern have a rest element
/// only last and without a comma after it (`#sec-destructuring-binding-patterns`,
/// `#sec-destructuring-assignment`).
pub(crate) const REST_ELEMENT_NOT_LAST: &str =
    "a rest element must be the last element of its pattern";

/// The message of a rest element `... target = value` of an array pattern: a BindingRestElement
/// and an AssignmentRestElement have no Initializer.
pub(crate) const REST_ELEMENT_INITIALIZER: &str = "a rest element may not have an initializer";

/// The message of a `with` statement in strict mode code.
pub(crate) const WITH_IN_STRICT: &str = "with statement in strict mode code";

/// The message of a `??` with an `&&` or `||` operand, or the other way round, outside
/// parentheses (`#sec-binary-logical-operators`).
pub(crate) const COALESCE_MIXED: &str = "?? mixed with && or || without parentheses";

/// The message of an `IdentifierName` written with an escape whose StringValue is the reserved
/// word `name`, used as an identifier or where the keyword stands
/// (`#sec-identifiers-static-semantics-early-errors`, `#sec-keywords-and-reserved-words`).
pub(crate) fn escaped_keyword(name: &str) -> String {
    format!("keyword {name} must not contain escaped characters")
}

/// The message of `yield` as a BindingIdentifier, IdentifierReference or LabelIdentifier where
/// [Yield] is set, or as the name of a GeneratorExpression
/// (`#sec-identifiers-static-semantics-early-errors`).
const YIELD_IDENTIFIER: &str = "yield may not be an identifier in a generator function";

/// The message of a YieldExpression in the FormalParameters of a generator
/// (`#sec-generator-function-definitions-static-semantics-early-errors`).
const YIELD_IN_GENERATOR_PARAMETERS: &str =
    "yield expression in the parameters of a generator function";

/// The message of a YieldExpression in the ArrowParameters of an arrow function
/// (`#sec-arrow-function-definitions-static-semantics-early-errors`).
const YIELD_IN_ARROW_PARAMETERS: &str = "yield expression in the parameters of an arrow function";

/// The message of `await` as a BindingIdentifier, IdentifierReference or LabelIdentifier where
/// [Await] is set, or as the name of an AsyncFunctionExpression
/// (`#sec-identifiers-static-semantics-early-errors`).
const AWAIT_IDENTIFIER: &str = "await may not be an identifier in an async function";

/// The message of an AwaitExpression in the FormalParameters of an `async` function
/// (`#sec-async-function-definitions-static-semantics-early-errors`). Its text does not say
/// "await expression", the text of the refusal of the base, so a search of the runs for that
/// refusal finds none of these early errors.
const AWAIT_IN_ASYNC_PARAMETERS: &str = "await in the parameters of an async function";

/// The message of an AwaitExpression in the ArrowParameters of an arrow function or the head of
/// an `async` arrow function (`#sec-arrow-function-definitions-static-semantics-early-errors`,
/// `#sec-async-arrow-function-definitions-static-semantics-early-errors`).
const AWAIT_IN_ARROW_PARAMETERS: &str = "await in the parameters of an arrow function";

/// The identifiers strict mode code reserves (`#sec-identifiers-static-semantics-early-errors`).
const STRICT_RESERVED: [&str; 9] = [
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
];

/// The early errors of the script `body` of `ast`, strict when `strict`, in source order; the
/// pattern of a regular expression literal is parsed with the nesting limit `max_nesting`.
/// When a pattern is refused, the first refusal in source order is the only error.
///
/// Every structure of the walk is charged to `m` before it grows and credited when it is
/// dropped: the frames of lexical names (an entry each, [`crate::ledger::map_entry_bytes`], and
/// the bytes of each name), the labels, the stack of pending work, the tree of each pattern
/// (until its check ends) and the errors (a message is charged by its capacity once it is
/// formatted). The walk calls [`Meter::visit`] for every node and ends at the first stop; the
/// entry then answers the stop.
pub(crate) fn check(
    ast: &Ast,
    body: &[StmtId],
    strict: bool,
    max_nesting: u32,
    m: &Meter<'_>,
) -> Vec<SyntaxError> {
    let mut checker = Checker {
        ast,
        m,
        max_nesting,
        errors: Vec::new(),
        strict,
        yields: false,
        awaits: false,
        parameters: None,
        loops: 0,
        breakables: 0,
        labels: Vec::new(),
        label_base: 0,
        lexical: Vec::new(),
        lexical_base: 0,
    };
    let frame = checker.frame(Names::List(body), &[], "");
    checker.push_frame(frame);
    checker.walk(body);
    let Checker {
        errors: mut found,
        labels,
        lexical,
        ..
    } = checker;
    m.release(labels);
    m.release(lexical);
    // The stable sort may take a buffer as long as the list: charged for the sort.
    let scratch = found.len().saturating_mul(size_of::<SyntaxError>());
    if m.charge(scratch).is_ok() {
        found.sort_by_key(|e| (e.line, e.column));
        m.credit(scratch);
    }
    only_the_refusal(found, m)
}

/// `errors` as the parse answers them: a refusal stops a parse and is its only error (`lib.rs`,
/// "A part before its slice"), so when a pattern was refused the first refusal is kept and
/// every other error is dropped, its message credited.
fn only_the_refusal(mut errors: Vec<SyntaxError>, m: &Meter<'_>) -> Vec<SyntaxError> {
    let refused = |e: &SyntaxError| matches!(message_kind(&e.message), MessageKind::Refusal { .. });
    let Some(first) = errors.iter().position(refused) else {
        return errors;
    };
    let mut index = 0;
    errors.retain(|e| {
        let keep = index == first;
        index += 1;
        if !keep {
            m.credit(e.message.capacity());
        }
        keep
    });
    errors
}

/// Where the names of a frame of lexical names come from.
#[derive(Clone, Copy)]
enum Names<'a> {
    /// The `let` and `const` declarations of the top level of a script or a function body,
    /// where a function declaration is var scoped.
    List(&'a [StmtId]),
    /// The `let`, `const` and function declarations of a block, or of a `try`, `catch` or
    /// `finally` block (`#sec-block-static-semantics-lexicallydeclarednames`).
    Block(&'a [StmtId]),
    /// Those of the clauses of a `case` block, taken together.
    Cases(&'a [SwitchCase]),
    /// The declarators of a `for` head.
    Declarators(&'a [VarDeclarator]),
    /// The binding of a for-in or for-of head, or the names of a `catch` parameter that is a
    /// pattern.
    One(&'a BindingTarget),
}

/// One piece of the walk's pending work.
enum Step {
    Node(Child),
    /// Puts back the context a statement or a function changed, once its children are walked.
    Restore {
        strict: bool,
        yields: bool,
        awaits: bool,
        parameters: Option<Messages>,
        loops: u32,
        breakables: u32,
        label_base: usize,
        lexical_base: usize,
    },
    /// Sets the [Yield] and [Await] of what follows, and whether it is the parameters of a
    /// function, with the messages of a YieldExpression and an AwaitExpression there: between
    /// the parameters and the body of a function.
    Context {
        yields: bool,
        awaits: bool,
        parameters: Option<Messages>,
    },
    /// Drops the label of a labelled statement, once its body is walked.
    PopLabel,
    /// Enters the lexical names of a `catch` or `finally` block, made as the `try` was entered.
    Enter(BTreeSet<String>),
    /// Drops the innermost frame of lexical names, once the statements it covers are walked.
    PopLexical,
}

/// The messages of a YieldExpression and of an AwaitExpression in the parameters of a function.
type Messages = (&'static str, &'static str);

impl HeapBytes for Step {
    fn heap_bytes(&self) -> usize {
        match self {
            Step::Enter(frame) => frame.heap_bytes(),
            _ => 0,
        }
    }
}

struct Checker<'a, 'm> {
    ast: &'a Ast,
    m: &'a Meter<'m>,
    /// The host's `parser_nesting`, the nesting limit of a literal's pattern.
    max_nesting: u32,
    errors: Vec<SyntaxError>,
    strict: bool,
    /// The [Yield] parameter of the grammar at the current position.
    yields: bool,
    /// The [Await] parameter of the grammar at the current position.
    awaits: bool,
    /// Inside the parameters of a function (not of a function nested in them): the messages of
    /// a YieldExpression and an AwaitExpression there; `None` elsewhere.
    parameters: Option<Messages>,
    /// Loops around the current position inside the current function.
    loops: u32,
    /// Loops and `switch` statements around the current position inside the current function.
    breakables: u32,
    /// The labels around the current position, each with whether it labels an iteration
    /// statement (through other labels); those from `label_base` on are the current function's.
    labels: Vec<(String, bool)>,
    label_base: usize,
    /// The frames of lexical names around the current position, innermost last; those from
    /// `lexical_base` on are the current function's.
    lexical: Vec<BTreeSet<String>>,
    lexical_base: usize,
}

impl<'a, 'm> Checker<'a, 'm> {
    /// A frame of the lexical names `names`, in source order: a name twice is an error, and so
    /// is one of `others` (the parameters or the catch parameter, named by `what`).
    fn frame(
        &mut self,
        names: Names<'a>,
        others: &[BindingTarget],
        what: &str,
    ) -> BTreeSet<String> {
        let mut frame = BTreeSet::new();
        match names {
            Names::List(list) => {
                for &s in list {
                    self.frame_statement(&mut frame, s, others, what, false);
                }
            }
            Names::Block(list) => {
                for &s in list {
                    self.frame_statement(&mut frame, s, others, what, true);
                }
            }
            Names::Cases(cases) => {
                for c in cases {
                    for &s in &c.body {
                        self.frame_statement(&mut frame, s, others, what, true);
                    }
                }
            }
            Names::Declarators(declarators) => {
                for d in declarators {
                    for name in d.target.names() {
                        self.frame_name(&mut frame, name, others, what);
                    }
                }
            }
            Names::One(target) => {
                for name in target.names() {
                    self.frame_name(&mut frame, name, others, what);
                }
            }
        }
        frame
    }

    /// The names the statement `s` binds when it is a `let` or `const` declaration, or, with
    /// `functions`, a function declaration of a block or a `case` block, into `frame`.
    fn frame_statement(
        &mut self,
        frame: &mut BTreeSet<String>,
        s: StmtId,
        others: &[BindingTarget],
        what: &str,
        functions: bool,
    ) {
        let ast = self.ast;
        let statement = ast.stmt(s).map(|n| &n.stmt);
        // A class declaration binds its name as `let` does, at the top of a body too.
        if let Some(Stmt::Class(class)) = statement {
            if let Some(name) = &class.name {
                self.frame_name(frame, name, others, what);
            }
            return;
        }
        if let Some(Stmt::FunctionDeclaration(f)) = statement {
            if functions && let Some(name) = ast.function(*f).and_then(|x| x.name.as_ref()) {
                self.frame_name(frame, name, others, what);
            }
            return;
        }
        let Some(Stmt::Var(kind, declarators)) = statement else {
            return;
        };
        if !kind.is_lexical() {
            return;
        }
        for d in declarators {
            for name in d.target.names() {
                self.frame_name(frame, name, others, what);
            }
        }
    }

    /// The lexical name `n` into `frame`: its entry and a copy of the name charged first.
    fn frame_name(
        &mut self,
        frame: &mut BTreeSet<String>,
        n: &Name,
        others: &[BindingTarget],
        what: &str,
    ) {
        if others
            .iter()
            .flat_map(BindingTarget::names)
            .any(|o| o.name == n.name)
        {
            self.error(
                format!("lexical declaration of {} conflicts with {what}", n.name),
                n.span,
            );
        }
        if frame.contains(&n.name) {
            self.error(
                format!("duplicate lexical declaration of {}", n.name),
                n.span,
            );
            return;
        }
        if let Ok(key) = self.m.string(&n.name) {
            // A refusal stops the meter; the walk ends at its next node.
            let _ = self.m.insert(frame, key);
        }
    }

    /// `frame` as the innermost frame of lexical names, the growth of the list charged first.
    fn push_frame(&mut self, frame: BTreeSet<String>) {
        let _ = self.m.push(&mut self.lexical, frame);
    }

    /// `step` onto `stack`, the growth charged first.
    fn push_step(&self, stack: &mut Vec<Step>, step: Step) {
        if self.m.grow(stack).is_err() {
            self.m.release(step);
            return;
        }
        stack.push(step);
    }

    /// Pushes the statements `list` with the frame of lexical names around them, so that
    /// popping gives the frame's entry, the statements in source order and the frame's end.
    fn push_list(&self, stack: &mut Vec<Step>, list: &[StmtId], frame: BTreeSet<String>) {
        if self.m.grow_by(stack, list.len().saturating_add(2)).is_err() {
            self.m.release(frame);
            return;
        }
        stack.push(Step::PopLexical);
        stack.extend(list.iter().rev().map(|s| Step::Node(Child::Stmt(*s))));
        stack.push(Step::Enter(frame));
    }

    /// A name a `var` or a top-level function declaration binds: no lexical declaration of the
    /// current function around it may bind it too.
    fn var_name(&mut self, name: &Name) {
        let clash = self
            .lexical
            .get(self.lexical_base..)
            .unwrap_or(&[])
            .iter()
            .any(|f| f.contains(&name.name));
        if clash {
            self.error(
                format!(
                    "lexical declaration of {} conflicts with a var or function declaration",
                    name.name
                ),
                name.span,
            );
        }
    }

    /// Whether the statement being checked stands at the top level of the script or of the
    /// current function's body: the frame of that body is the current function's only one. Every
    /// block, `case` block, `try`, `catch` and `finally` block pushes a frame of its own, empty or
    /// not, before its statements are walked, and no other place admits a function declaration.
    fn at_top_level(&self) -> bool {
        self.lexical.len() == self.lexical_base.saturating_add(1)
    }

    /// A name a declaration of kind `kind` binds.
    fn declared(&mut self, kind: DeclKind, name: &Name) {
        if kind.is_lexical() && name.name == "let" {
            self.error("let is not a valid lexically bound name", name.span);
            return;
        }
        self.binding(name);
        if kind == DeclKind::Var {
            self.var_name(name);
        }
    }

    /// Enters the lexical names the statement `id` declares for its children: those of a block,
    /// a `case` block, or a `for`, for-in or for-of head. A `try` pushes its children itself, a
    /// frame around each of its blocks; then this answers true.
    fn lexical_scope(&mut self, id: StmtId, stack: &mut Vec<Step>) -> bool {
        let ast = self.ast;
        let Some(node) = ast.stmt(id) else {
            return false;
        };
        let names = match &node.stmt {
            Stmt::Block(body) => Names::Block(body),
            Stmt::Switch { cases, .. } => Names::Cases(cases),
            Stmt::For {
                init: Some(ForInit::Var(kind, declarators)),
                ..
            } if kind.is_lexical() => Names::Declarators(declarators),
            Stmt::ForIn {
                target: ForInTarget::Var(kind, target),
                ..
            } if kind.is_lexical() => Names::One(target),
            Stmt::Try {
                block,
                handler,
                finalizer,
            } => {
                let block_frame = self.frame(Names::Block(block), &[], "");
                if let Some(list) = finalizer {
                    let frame = self.frame(Names::Block(list), &[], "");
                    self.push_list(stack, list, frame);
                }
                if let Some(h) = handler {
                    // `#sec-try-statement-static-semantics-early-errors`: the names of a pattern
                    // are a frame of their own around the block, which no `var` of the block may
                    // bind (Annex B allows that only for a `BindingIdentifier`,
                    // `#sec-variablestatements-in-catch-blocks`), and which holds no name twice.
                    let pattern = h.param.as_ref().filter(|p| p.pattern().is_some());
                    if pattern.is_some() {
                        self.push_step(stack, Step::PopLexical);
                    }
                    let param: &[BindingTarget] = h.param.as_slice();
                    let frame = self.frame(Names::Block(&h.body), param, "the catch parameter");
                    self.push_list(stack, &h.body, frame);
                    if let Some(target) = pattern {
                        if let Some(e) = target.pattern() {
                            self.push_step(stack, Step::Node(Child::Expr(e)));
                        }
                        let frame = self.frame(Names::One(target), &[], "");
                        self.push_step(stack, Step::Enter(frame));
                    }
                }
                if self.m.grow_by(stack, block.len().saturating_add(1)).is_ok() {
                    stack.push(Step::PopLexical);
                    stack.extend(block.iter().rev().map(|s| Step::Node(Child::Stmt(*s))));
                }
                self.push_frame(block_frame);
                return true;
            }
            _ => return false,
        };
        let frame = self.frame(names, &[], "");
        self.push_step(stack, Step::PopLexical);
        self.push_frame(frame);
        false
    }

    /// An error at `span`: the growth of the list charged first, and the message by its
    /// capacity once it is formatted.
    fn error(&mut self, message: impl Into<String>, span: Span) {
        let message = message.into();
        if self.m.charge(message.capacity()).is_err() {
            return;
        }
        let _ = self.m.push(
            &mut self.errors,
            SyntaxError::new(message, span.line, span.column),
        );
    }

    /// A `BindingIdentifier` in code of the current strictness, [Yield] and [Await].
    fn binding(&mut self, name: &Name) {
        if self.yields && name.name == "yield" {
            self.error(YIELD_IDENTIFIER, name.span);
            return;
        }
        if self.awaits && name.name == "await" {
            self.error(AWAIT_IDENTIFIER, name.span);
            return;
        }
        if !self.strict {
            return;
        }
        if name.name == "eval" || name.name == "arguments" {
            self.error(
                format!("{} may not be bound in strict mode code", name.name),
                name.span,
            );
        } else if STRICT_RESERVED.contains(&name.name.as_str()) {
            self.reserved(&name.name, name.span);
        }
    }

    fn reserved(&mut self, name: &str, span: Span) {
        self.error(
            format!("{name} is a reserved word in strict mode code"),
            span,
        );
    }

    /// The target of an assignment, an update or a for-in or for-of head
    /// (`#sec-static-semantics-assignmenttargettype`).
    fn target(&mut self, e: ExprId) {
        let Some(node) = self.ast.expr(e) else {
            return;
        };
        match &node.expr {
            Expr::Identifier(n) => {
                if self.strict && (n == "eval" || n == "arguments") {
                    self.error(
                        format!("{n} may not be assigned in strict mode code"),
                        node.span,
                    );
                }
            }
            Expr::Member { .. } | Expr::Index { .. } | Expr::SuperMember { .. } => {}
            _ => self.error("invalid assignment target", node.span),
        }
    }

    fn number(&mut self, n: &NumberLiteral, span: Span) {
        if !self.strict {
            return;
        }
        match n.form {
            NumberForm::LegacyOctal => self.error("legacy octal literal in strict mode code", span),
            NumberForm::NonOctalDecimal => {
                self.error(
                    "decimal literal with a leading zero in strict mode code",
                    span,
                );
            }
            _ => {}
        }
    }

    fn string(&mut self, s: &StringLiteral, span: Span) {
        if !self.strict {
            return;
        }
        if s.legacy_octal_escape {
            self.error("legacy octal escape sequence in strict mode code", span);
        } else if s.non_octal_decimal_escape {
            self.error("\\8 or \\9 escape sequence in strict mode code", span);
        }
    }

    /// IsValidRegularExpressionLiteral
    /// (`#sec-primary-expression-regular-expression-literals-static-semantics-early-errors`):
    /// the flags are only `d g i m s u v y`, each once, and not `u` with `v`; then ParsePattern
    /// (`#sec-parsepattern`) of the body with the flags (the `u` and `v` modes, and the grammar
    /// of B.1.2 with neither) and the limit `parser_nesting`, charged to the meter of the parse
    /// (`docs/design/engine.md`, decision 7, "A regular expression literal: checked at parse
    /// time, compiled at each evaluation", rule 1). An error of the pattern is an early error at
    /// the literal's line and column with the message of `swell-regexp`: a grammar or early error,
    /// the nesting limit, or a refusal of slice 5, which [`only_the_refusal`] makes the only
    /// error. The tree is dropped and credited as soon as the check ends; a stop of the meter
    /// stops the walk at its next node.
    fn regexp(&mut self, r: &RegExpLiteral, span: Span) {
        let Ok(flags) = swell_regexp::Flags::parse(&r.flags) else {
            self.error("invalid regular expression flags", span);
            return;
        };
        let mut budget = PatternBudget::new(self.m);
        match swell_regexp::parse_pattern(&r.body, flags, self.max_nesting, &mut budget) {
            Ok(tree) => tree.release(&mut budget),
            Err(swell_regexp::ParseError::Pattern(error)) => self.error(error.message(), span),
            Err(swell_regexp::ParseError::Stopped(_)) => {
                if self.m.check().is_ok() {
                    // The crate stopped without a stop of the meter: the allocator refused a
                    // growth the crate had charged for. The meter records that as a refusal of
                    // its own: a reservation of 2^63 bytes, past what a vector may hold, which a
                    // ledger refuses or the allocator does, credited at once either way.
                    let _ = self
                        .m
                        .with_capacity::<u8>((isize::MAX as usize).saturating_add(1));
                }
            }
        }
    }

    fn property_key(&mut self, key: &PropertyKey, span: Span) {
        match key {
            PropertyKey::Identifier(_) => {}
            PropertyKey::String(s) => self.string(s, span),
            PropertyKey::Number(n) => self.number(n, span),
            PropertyKey::Computed(_) => {}
        }
    }

    fn is_proto(key: &PropertyKey) -> bool {
        match key {
            PropertyKey::Identifier(n) => n == "__proto__",
            PropertyKey::String(s) => s.value.iter().copied().eq("__proto__".encode_utf16()),
            PropertyKey::Number(_) | PropertyKey::Computed(_) => false,
        }
    }

    /// Walks the statements `body` and everything in them, each node before its children and the
    /// children in source order, so the errors come in source order.
    fn walk(&mut self, body: &[StmtId]) {
        let m = self.m;
        let Ok(mut stack) = m.with_capacity::<Step>(body.len()) else {
            return;
        };
        stack.extend(body.iter().rev().map(|s| Step::Node(Child::Stmt(*s))));
        while let Some(step) = stack.pop() {
            let node = match step {
                Step::Node(node) => node,
                Step::Restore {
                    strict,
                    yields,
                    awaits,
                    parameters,
                    loops,
                    breakables,
                    label_base,
                    lexical_base,
                } => {
                    (self.strict, self.loops, self.breakables) = (strict, loops, breakables);
                    (self.label_base, self.lexical_base) = (label_base, lexical_base);
                    (self.yields, self.awaits, self.parameters) = (yields, awaits, parameters);
                    continue;
                }
                Step::Context {
                    yields,
                    awaits,
                    parameters,
                } => {
                    (self.yields, self.awaits, self.parameters) = (yields, awaits, parameters);
                    continue;
                }
                Step::PopLabel => {
                    if let Some(label) = self.labels.pop() {
                        m.release(label);
                    }
                    continue;
                }
                Step::Enter(frame) => {
                    self.push_frame(frame);
                    continue;
                }
                Step::PopLexical => {
                    if let Some(frame) = self.lexical.pop() {
                        m.release(frame);
                    }
                    continue;
                }
            };
            if m.visit().is_err() {
                break;
            }
            let restore = Step::Restore {
                strict: self.strict,
                yields: self.yields,
                awaits: self.awaits,
                parameters: self.parameters,
                loops: self.loops,
                breakables: self.breakables,
                label_base: self.label_base,
                lexical_base: self.lexical_base,
            };
            // A class is strict mode code from `class` to its `}`, the walk of its children too.
            if self.class_early_errors(node) {
                let back = self.restore_step();
                self.push_step(&mut stack, back);
                self.strict = true;
            }
            match node {
                Child::Stmt(id) => {
                    let (loops, breakables) = self.check_stmt(id);
                    if loops + breakables > 0 {
                        self.push_step(&mut stack, restore);
                        self.loops += loops;
                        self.breakables += breakables;
                    }
                    if self.label(id) {
                        self.push_step(&mut stack, Step::PopLabel);
                    }
                    if self.lexical_scope(id, &mut stack) {
                        continue;
                    }
                }
                Child::Expr(id) => self.check_expr(id),
                // A function is walked with its own strictness, outside every loop, `switch` and
                // label, with the lexical names of its body as its only frame, and with the
                // [Yield] and [Await] of its parameters and then of its body.
                Child::Function(id) => {
                    let ast = self.ast;
                    let Some(f) = ast.function(id) else {
                        continue;
                    };
                    self.push_step(&mut stack, restore);
                    let outer = (self.yields, self.awaits);
                    self.strict = f.strict;
                    self.loops = 0;
                    self.breakables = 0;
                    self.label_base = self.labels.len();
                    self.check_function(id, outer);
                    self.lexical_base = self.lexical.len();
                    let frame = self.frame(Names::List(&f.body), &f.params, "a parameter");
                    self.push_step(&mut stack, Step::PopLexical);
                    self.push_frame(frame);
                    if self.push_function(f, outer, &mut stack).is_err() {
                        break;
                    }
                    continue;
                }
            }
            if self
                .ast
                .push_children_charged(node, &mut stack, Step::Node, m)
                .is_err()
            {
                break;
            }
        }
        m.release(stack);
    }

    /// Pushes the parameters and the body of the function `f`, inside code whose [Yield] and
    /// [Await] are `outer`, so that popping gives: the [Yield] and [Await] of its parameters
    /// ([`Checker::parameter_context`]) with the messages of a YieldExpression and an
    /// AwaitExpression there, the pattern and the initializer of each parameter in source order,
    /// the [Yield] and [Await] of its body (a generator's [Yield] and an `async` function's
    /// [Await]; in an arrow function's body [Yield] cleared and [Await] its own,
    /// `#sec-arrow-function-definitions`, `#sec-async-arrow-function-definitions`), then the
    /// statements of the body. The growth of the stack is charged first.
    fn push_function(
        &self,
        f: &'a Function,
        outer: (bool, bool),
        stack: &mut Vec<Step>,
    ) -> Result<(), crate::ledger::Stop> {
        let (yields, awaits) = Checker::parameter_context(f, outer);
        let message = if f.kind == FunctionKind::Arrow {
            (YIELD_IN_ARROW_PARAMETERS, AWAIT_IN_ARROW_PARAMETERS)
        } else {
            (YIELD_IN_GENERATOR_PARAMETERS, AWAIT_IN_ASYNC_PARAMETERS)
        };
        let count = f
            .initializers
            .iter()
            .flatten()
            .count()
            .saturating_add(f.params.iter().filter_map(BindingTarget::pattern).count())
            .saturating_add(f.body.len())
            .saturating_add(2);
        self.m.grow_by(stack, count)?;
        stack.extend(f.body.iter().rev().map(|s| Step::Node(Child::Stmt(*s))));
        stack.push(Step::Context {
            yields: f.generator,
            awaits: f.asynchronous,
            parameters: None,
        });
        for (i, p) in f.params.iter().enumerate().rev() {
            if let Some(e) = f.initializer(i) {
                stack.push(Step::Node(Child::Expr(e)));
            }
            if let Some(e) = p.pattern() {
                stack.push(Step::Node(Child::Expr(e)));
            }
        }
        stack.push(Step::Context {
            yields,
            awaits,
            parameters: Some(message),
        });
        Ok(())
    }

    /// The [Yield] and [Await] of the parameters of the function `f` inside code whose [Yield]
    /// and [Await] are `outer`: a generator's [Yield] and an `async` function's [Await], each
    /// cleared for every other function; for an arrow function those around it
    /// (`ArrowParameters[?Yield, ?Await]`), but [Await] set for an `async` one
    /// (`AsyncArrowBindingIdentifier[?Yield]` with `BindingIdentifier[?Yield, +Await]`,
    /// `CoverCallExpressionAndAsyncArrowHead[?Yield, ?Await]` refined as
    /// `ArrowFormalParameters[~Yield, +Await]`, whose YieldExpression is an early error).
    fn parameter_context(f: &Function, outer: (bool, bool)) -> (bool, bool) {
        if f.kind == FunctionKind::Arrow {
            (outer.0, outer.1 || f.asynchronous)
        } else {
            (f.generator, f.asynchronous)
        }
    }

    /// The early errors of the statement `id` itself; returns how many loops and how many
    /// breakable statements it opens for its children (the tests of a loop hold no `break` or
    /// `continue` that is not inside a function of their own).
    fn check_stmt(&mut self, id: StmtId) -> (u32, u32) {
        let Some(node) = self.ast.stmt(id) else {
            return (0, 0);
        };
        match &node.stmt {
            Stmt::Var(kind, declarators)
            | Stmt::For {
                init: Some(ForInit::Var(kind, declarators)),
                ..
            } => {
                for d in declarators {
                    for name in d.target.names() {
                        self.declared(*kind, name);
                    }
                    if d.init.is_some() {
                        continue;
                    }
                    // `#sec-destructuring-binding-patterns`: a `BindingPattern` in a
                    // declaration has an `Initializer` (the grammar of `LexicalBinding` and
                    // `VariableDeclaration`), but in the head of a for-in or a for-of.
                    if d.target.pattern().is_some() {
                        self.error(
                            "missing initializer in a destructuring declaration",
                            d.target.span(),
                        );
                    } else if *kind == DeclKind::Const {
                        self.error(
                            "missing initializer in a const declaration",
                            d.target.span(),
                        );
                    }
                }
            }
            Stmt::ForIn { target, .. } => match target {
                ForInTarget::Var(kind, target) => {
                    for name in target.names() {
                        self.declared(*kind, name);
                    }
                }
                // An assignment pattern is checked as the expression it is.
                ForInTarget::Expression(e) => {
                    if !self.is_pattern(*e) {
                        self.target(*e);
                    }
                }
            },
            // A function declaration at the top level of a body is var scoped; one in a block or
            // a `case` block is a name of that block's frame, checked as the frame was made.
            Stmt::FunctionDeclaration(f) => {
                if self.at_top_level()
                    && let Some(name) = self.ast.function(*f).and_then(|x| x.name.as_ref())
                {
                    self.var_name(name);
                }
            }
            Stmt::Continue(None) if self.loops == 0 => {
                self.error("continue outside of a loop", node.span);
            }
            Stmt::Break(None) if self.breakables == 0 => {
                self.error("break outside of a loop or switch", node.span);
            }
            Stmt::Continue(Some(label)) => match self.find_label(&label.name) {
                None => self.error(
                    format!("continue to undefined label {}", label.name),
                    label.span,
                ),
                Some(false) => self.error(
                    format!(
                        "continue to label {}, which labels no iteration statement",
                        label.name
                    ),
                    label.span,
                ),
                Some(true) => {}
            },
            Stmt::Break(Some(label)) => {
                if self.find_label(&label.name).is_none() {
                    self.error(
                        format!("break to undefined label {}", label.name),
                        label.span,
                    );
                }
            }
            Stmt::Try {
                handler: Some(h), ..
            } => {
                for name in h.param.iter().flat_map(BindingTarget::names) {
                    self.binding(name);
                }
            }
            _ => {}
        }
        match node.stmt {
            Stmt::DoWhile { .. } | Stmt::While { .. } | Stmt::For { .. } | Stmt::ForIn { .. } => {
                (1, 1)
            }
            Stmt::Switch { .. } => (0, 1),
            _ => (0, 0),
        }
    }

    /// Whether the label `name` is around the current position in the current function, and if
    /// so whether it labels an iteration statement.
    fn find_label(&self, name: &str) -> Option<bool> {
        self.labels
            .get(self.label_base..)
            .unwrap_or(&[])
            .iter()
            .rev()
            .find(|(l, _)| l == name)
            .map(|(_, is_loop)| *is_loop)
    }

    /// When `id` is a labelled statement: checks its label and pushes it for the statement's
    /// body; returns whether it pushed one.
    fn label(&mut self, id: StmtId) -> bool {
        let Some(Stmt::Labelled { label, body }) = self.ast.stmt(id).map(|n| &n.stmt) else {
            return false;
        };
        if self.yields && label.name == "yield" {
            self.error(YIELD_IDENTIFIER, label.span);
        } else if self.awaits && label.name == "await" {
            self.error(AWAIT_IDENTIFIER, label.span);
        } else if self.strict && STRICT_RESERVED.contains(&label.name.as_str()) {
            self.reserved(&label.name, label.span);
        }
        if self.find_label(&label.name).is_some() {
            self.error(format!("duplicate label {}", label.name), label.span);
        }
        let mut item = *body;
        while let Some(Stmt::Labelled { body, .. }) = self.ast.stmt(item).map(|n| &n.stmt) {
            item = *body;
        }
        let is_loop = matches!(
            self.ast.stmt(item).map(|n| &n.stmt),
            Some(Stmt::DoWhile { .. } | Stmt::While { .. } | Stmt::For { .. } | Stmt::ForIn { .. })
        );
        // A refusal stops the meter and the walk ends at its next node; the label is pushed or
        // nothing is, and true still names the step that pops it.
        if let Ok(name) = self.m.string(&label.name) {
            let _ = self.m.push(&mut self.labels, (name, is_loop));
        }
        true
    }

    /// The early errors of the expression `id` itself.
    fn check_expr(&mut self, id: ExprId) {
        let Some(node) = self.ast.expr(id) else {
            return;
        };
        let span = node.span;
        match &node.expr {
            Expr::Identifier(n) => {
                if self.yields && n == "yield" {
                    self.error(YIELD_IDENTIFIER, span);
                } else if self.awaits && n == "await" {
                    self.error(AWAIT_IDENTIFIER, span);
                } else if self.strict && STRICT_RESERVED.contains(&n.as_str()) {
                    self.reserved(n, span);
                }
            }
            // Contains YieldExpression and Contains AwaitExpression of the parameters around it,
            // which the walk does not look for inside a nested function.
            Expr::Yield { .. } => {
                if let Some((message, _)) = self.parameters {
                    self.error(message, span);
                }
            }
            Expr::Await(_) => {
                if let Some((_, message)) = self.parameters {
                    self.error(message, span);
                }
            }
            Expr::Number(n) => self.number(n, span),
            Expr::String(s) => self.string(s, span),
            Expr::RegExp(r) => self.regexp(r, span),
            Expr::Object(properties) => {
                let mut protos = 0;
                for p in properties {
                    self.property_key(&p.key, p.span);
                    if matches!(p.value, PropertyValue::Init(_)) && Checker::is_proto(&p.key) {
                        protos += 1;
                        if protos == 2 {
                            self.error("duplicate __proto__ property in an object literal", p.span);
                        }
                    }
                    // `PropertyDefinition : CoverInitializedName` in a literal that no pattern
                    // reads again.
                    if matches!(p.value, PropertyValue::CoverInitialized(..)) {
                        self.error(COVER_INITIALIZED_NAME, p.span);
                    }
                }
            }
            // `#sec-destructuring-assignment-static-semantics-early-errors`: each target of an
            // assignment pattern that is no nested pattern is a simple assignment target (a
            // binding pattern's names are checked with its declaration).
            Expr::ObjectPattern {
                properties, kind, ..
            } => {
                for p in properties {
                    self.property_key(&p.key, p.span);
                    if let (PatternKind::Assignment, PatternTarget::Expr(t)) = (kind, &p.target)
                        && !self.is_pattern(*t)
                    {
                        self.target(*t);
                    }
                }
            }
            Expr::ArrayPattern { elements, kind, .. } => {
                for p in elements.iter().flatten() {
                    if let (PatternKind::Assignment, PatternTarget::Expr(t)) = (kind, &p.target)
                        && !self.is_pattern(*t)
                    {
                        self.target(*t);
                    }
                }
            }
            Expr::Unary {
                op: UnaryOp::Delete,
                operand,
            } => {
                if self.strict
                    && matches!(
                        self.ast.expr(*operand).map(|n| &n.expr),
                        Some(Expr::Identifier(_))
                    )
                {
                    self.error(
                        "delete of an unqualified identifier in strict mode code",
                        span,
                    );
                }
            }
            Expr::Update { operand, .. } => self.target(*operand),
            // A pattern's targets are checked with the pattern.
            Expr::Assign { target, .. } if !self.is_pattern(*target) => {
                self.target(*target);
            }
            // A NotEscapeSequence of a tagged template is no early error: the rule of
            // `#sec-static-semantics-template-early-errors` reads a template without a tag only.
            Expr::TaggedTemplate { .. } => {}
            _ => {}
        }
    }

    /// Whether `e` is an object or array pattern.
    fn is_pattern(&self, e: ExprId) -> bool {
        matches!(
            self.ast.expr(e).map(|n| &n.expr),
            Some(Expr::ObjectPattern { .. } | Expr::ArrayPattern { .. })
        )
    }

    /// The function's name and parameters, checked with its own strictness, inside code whose
    /// [Yield] and [Await] are `outer`: the name of a declaration with `outer`
    /// (`BindingIdentifier[?Yield, ?Await]`), that of an expression with the function's own
    /// (`[+Yield]` for a GeneratorExpression, `[+Await]` for an AsyncFunctionExpression, both
    /// for an AsyncGeneratorExpression, each cleared for a FunctionExpression); the parameters
    /// with those of
    /// [`Checker::parameter_context`].
    fn check_function(&mut self, id: FunctionId, outer: (bool, bool)) {
        let Some(f) = self.ast.function(id) else {
            return;
        };
        if let Some(name) = &f.name {
            (self.yields, self.awaits) = if f.kind == FunctionKind::Expression {
                (f.generator, f.asynchronous)
            } else {
                outer
            };
            self.binding(name);
        }
        (self.yields, self.awaits) = Checker::parameter_context(f, outer);
        // The names seen, each entry charged when it is inserted and all credited at the end.
        let entry = map_entry_bytes::<&str, ()>();
        let mut seen = BTreeSet::new();
        for p in f.parameter_names() {
            self.binding(p);
            if !seen.contains(p.name.as_str()) {
                if self.m.charge(entry).is_err() {
                    break;
                }
                seen.insert(p.name.as_str());
                continue;
            }
            // `#sec-method-definitions-static-semantics-early-errors`,
            // `#sec-generator-function-definitions-static-semantics-early-errors` and
            // `#sec-async-function-definitions-static-semantics-early-errors`: the
            // UniqueFormalParameters of a method, a generator, `async` or async generator method
            // too, in any code; a generator method's other early errors are those of every
            // generator, and an `async` method's (AsyncMethod: no AwaitExpression in its
            // parameters, no Use Strict Directive with a list that is not simple, no parameter
            // its body declares lexically) those of every `async` function. An async generator
            // method's (AsyncGeneratorMethod,
            // `#sec-async-generator-function-definitions-static-semantics-early-errors`: no
            // YieldExpression and no AwaitExpression in its parameters, no Use Strict Directive
            // with a list that is not simple, no parameter its body declares lexically) are those
            // of every async generator, which has both flags set. HasDirectSuper is a SuperCall,
            // which the parser answers as an early error where it reads it, in a method as
            // anywhere before derived classes. A class's constructor is a method
            // (`#sec-class-definitions`, ClassElement : MethodDefinition).
            if matches!(
                f.kind,
                FunctionKind::Method | FunctionKind::ClassConstructor
            ) {
                self.error(
                    format!("duplicate parameter name {} in a method", p.name),
                    p.span,
                );
            } else if f.kind == FunctionKind::Arrow {
                self.error(
                    format!("duplicate parameter name {} in an arrow function", p.name),
                    p.span,
                );
            } else if !f.is_simple() {
                // `#sec-function-definitions-static-semantics-early-errors`: a list that is not
                // simple binds each name once, in any code.
                self.error(
                    format!(
                        "duplicate parameter name {} in a parameter list that is not simple",
                        p.name
                    ),
                    p.span,
                );
            } else if self.strict {
                self.error(
                    format!("duplicate parameter name {} in strict mode code", p.name),
                    p.span,
                );
            }
        }
        self.m.credit(seen.len().saturating_mul(entry));
    }

    /// A step that puts the current context back, once the children pushed after it are walked.
    fn restore_step(&self) -> Step {
        Step::Restore {
            strict: self.strict,
            yields: self.yields,
            awaits: self.awaits,
            parameters: self.parameters,
            loops: self.loops,
            breakables: self.breakables,
            label_base: self.label_base,
            lexical_base: self.lexical_base,
        }
    }

    /// The early errors of the class `node` itself, a ClassDeclaration or a ClassExpression
    /// (`#sec-class-definitions-static-semantics-early-errors`); whether it is one, so that the
    /// walk reads its children as strict mode code (`#sec-strict-mode-code`: all parts of a
    /// class are). Its name is a BindingIdentifier of strict code with the [Yield] and [Await]
    /// around the class (`#sec-identifiers-static-semantics-early-errors`); a literal name of an
    /// element is read as strict code; an element that is not `static` and whose name is the
    /// literal `constructor` is a second constructor when it is a plain method (the parser made
    /// the first one the class's constructor) and a special method otherwise; and an element
    /// that is `static` may not be named `prototype` by a literal (a computed one is a
    /// `TypeError` at run time). Its methods, the constructor included, have the early errors of
    /// every function, with the strictness the parser gave them ([`Checker::check_function`]),
    /// and the name of a declaration is a lexical name of the list it stands in
    /// ([`Checker::frame_statement`]).
    fn class_early_errors(&mut self, node: Child) -> bool {
        let ast = self.ast;
        let class = match node {
            Child::Stmt(id) => match ast.stmt(id).map(|n| &n.stmt) {
                Some(Stmt::Class(class)) => class,
                _ => return false,
            },
            Child::Expr(id) => match ast.expr(id).map(|n| &n.expr) {
                Some(Expr::Class(class)) => class,
                _ => return false,
            },
            Child::Function(_) => return false,
        };
        let outer = self.strict;
        self.strict = true;
        if let Some(name) = &class.name {
            self.binding(name);
        }
        for element in &class.elements {
            let ClassElement::Method {
                is_static,
                key: ClassKey::Literal(key),
                kind,
                function,
                span,
            } = element
            else {
                continue;
            };
            self.property_key(key, *span);
            if *is_static {
                if Checker::is_named(key, "prototype") {
                    self.error("a static class method may not be named prototype", *span);
                }
                continue;
            }
            if !Checker::is_named(key, "constructor") {
                continue;
            }
            let (generator, asynchronous) = ast
                .function(*function)
                .map_or((false, false), |f| (f.generator, f.asynchronous));
            let what = match (kind, generator, asynchronous) {
                (FunctionKind::Getter, ..) => "a getter",
                (FunctionKind::Setter, ..) => "a setter",
                (_, true, true) => "an async generator method",
                (_, true, false) => "a generator method",
                (_, false, true) => "an async method",
                (_, false, false) => {
                    self.error("a class may have only one constructor", *span);
                    continue;
                }
            };
            self.error(format!("a class constructor may not be {what}"), *span);
        }
        self.strict = outer;
        true
    }

    /// Whether `key` is the literal PropertyName whose StringValue is `name`.
    fn is_named(key: &PropertyKey, name: &str) -> bool {
        match key {
            PropertyKey::Identifier(n) => n == name,
            PropertyKey::String(s) => s.value.iter().copied().eq(name.encode_utf16()),
            PropertyKey::Number(_) | PropertyKey::Computed(_) => false,
        }
    }
}
