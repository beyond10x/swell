#![forbid(unsafe_code)]
//! `swell-syntax`: the lexer, parser, early errors and scope analysis of swell (design,
//! decision 2).
//!
//! Slice 1 takes the ES5 grammar with strict mode (`docs/design/engine.md`, section 12): `var`
//! and function declarations, every ES5 statement but `with`, every ES5 expression form, the
//! directive prologue and the strict early errors. Slice 2 adds the language core around them:
//! automatic semicolon insertion, labelled statements, `??`, optional chains, the logical
//! assignments, optional catch bindings, trailing commas in calls and parameter lists, numeric
//! separators, a hashbang comment, `\u` escapes in ASCII identifiers, plain methods and
//! shorthand properties in object literals, the `with` statement in non-strict code, the
//! references to `arguments` that need an arguments object ([`Scope::arguments_object`]), and
//! the code compiled at run time: [`parse_eval`] for `eval` and [`parse_dynamic_function`] for
//! the `Function` constructor, beside [`parse_script`], with the direct `eval` calls scope
//! analysis marks ([`Scope::contains_direct_eval`]). Every construct of a later slice is refused
//! with a `SyntaxError` that names the slice: `<construct> is not supported before slice <n>`
//! ("A part before its slice"), which [`message_kind`] tells from a grammar or early error and
//! from the nesting limit by the message alone. Two of those refusals fall inside sections whose
//! cases are counted in slice 2 and are recorded as omissions in
//! `conformance/test262/sections.txt`: an identifier with a code point outside ASCII, written or
//! escaped (slice 5, `sec-names-and-keywords`), and a regular expression literal
//! ([`REGEXP_REFUSAL`]), which is lexed to its end and kept as a node, its pattern unparsed, so
//! that evaluating it throws a `SyntaxError` (`sec-literals-regular-expression-literals`).
//!
//! The parser's depth is bounded by [`ParseOptions::max_nesting`], the host's `parser_nesting`
//! (`docs/design/host-interface.md`), and the Rust stack a parse takes by
//! [`ParseOptions::stack_bytes`], through each of the three entries: they share one parser and
//! its one depth counter, and each parse of [`parse_dynamic_function`] stays within the bound on
//! its own. The parser is the only recursion: the walks after it (early
//! errors, scope analysis) keep their work on the heap, and the tree is arenas whose nodes name
//! their children by index, so dropping it does not recurse.
//!
//! Modules: [`error`], `token` and `lexer` (clause 12), [`ast`], `parser` (clauses 13 to 15),
//! `early` (the early errors), `scope` (directives, strictness, declared and referenced names)
//! and [`ledger`], the one trait every growth of the frontend is charged through before it is
//! made (`docs/design/engine.md`, decision 4, "Memory budget"). Each entry has a charged form
//! ([`parse_script_with`], [`parse_eval_with`], [`parse_dynamic_function_with`]) that takes a
//! [`Ledger`]; the entries without one are the charged forms with [`Unlimited`]. A charged
//! entry polls the ledger at least once per [`ledger::POLL_INTERVAL`] code units the lexer
//! advances and per as many nodes the early errors and the scope analysis visit, and keeps the
//! ledger in the parser's state, never in a recursive frame. The tree it answers holds what it
//! was charged ([`Script::charged`]); its owner credits that when it drops the tree. An entry
//! that answers errors or a stop has credited everything it charged.

pub mod ast;
mod early;
pub mod error;
pub mod ledger;
mod lexer;
mod parser;
mod scope;
mod token;

pub use ast::Scope;
pub use error::{MessageKind, REGEXP_REFUSAL, SyntaxError, message_kind};
pub use ledger::{Ledger, Stop, Unlimited};

use ast::{Ast, StmtId};
use ledger::{HeapBytes, Meter};

/// The options of a parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseOptions {
    /// The host's `parser_nesting`: how deep the parser may recurse and how deep a node of the
    /// tree may be. Passing it is the `SyntaxError` `nesting deeper than parser_nesting
    /// (<limit>)`; at 0 every non-empty source fails.
    pub max_nesting: u32,
}

/// The Rust stack, in bytes, one level of [`ParseOptions::max_nesting`] may take: the bound the
/// frames of every recursion of the parser hold, in an unoptimised build and an optimised one.
/// Measured on x86_64 with rustc 1.99.1 without incremental compilation: at most 172 bytes per
/// level in the debug profile (nested `if`, `switch`, function declarations and conditional
/// expressions), 115 in the release profile (function declarations). `tests/sources.rs` holds
/// every kind of nesting to it (`every_nesting_kind_fits_the_stack_bound`).
pub const STACK_BYTES_PER_LEVEL: usize = 224;

/// The Rust stack, in bytes, a parse takes besides its levels, the thread's own start included.
/// Measured as above: about 16 KiB.
pub const STACK_BYTES_FIXED: usize = 32 * 1024;

impl ParseOptions {
    /// The Rust stack, in bytes, a thread must have for [`parse_script`], [`parse_eval`] or
    /// [`parse_dynamic_function`] with these options to end without a stack overflow, whatever
    /// the source: [`STACK_BYTES_FIXED`] plus
    /// `max_nesting` times [`STACK_BYTES_PER_LEVEL`]. The parser's part of the host's
    /// `Limits::min_stack_bytes()` (`docs/design/host-interface.md`, decision 2).
    pub const fn stack_bytes(&self) -> usize {
        STACK_BYTES_FIXED
            .saturating_add((self.max_nesting as usize).saturating_mul(STACK_BYTES_PER_LEVEL))
    }
}

/// A parsed `Script` (`#sec-scripts`) with its analysis.
#[derive(Clone, Debug, PartialEq)]
pub struct Script {
    /// The statements of the `ScriptBody`.
    pub body: Vec<StmtId>,
    /// Whether the script is strict mode code: its directive prologue holds a Use Strict
    /// Directive (`#sec-strict-mode-code`).
    pub strict: bool,
    /// The script's var declared names, function declarations to instantiate and references.
    pub scope: Scope,
    pub ast: Ast,
    /// The bytes the ledger of a charged entry holds for this tree: its arenas, node vectors,
    /// names and scopes by capacity. The owner of the tree credits them when it drops it; 0 from
    /// an entry without a ledger.
    pub charged: usize,
}

impl Script {
    /// The bytes the tree owns on the heap, by capacity, summed by a walk of its arenas, their
    /// nodes' vectors and strings, and the scopes: what a charged entry's ledger holds for it.
    pub fn capacity_bytes(&self) -> usize {
        self.body
            .heap_bytes()
            .saturating_add(self.scope.heap_bytes())
            .saturating_add(self.ast.heap_bytes())
    }
}

/// Why a charged entry answered no tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// Every `SyntaxError` the parse found, at least one.
    Errors(Vec<SyntaxError>),
    /// The ledger refused a charge or its poll read the interrupt; no error was made.
    Stopped(Stop),
}

impl Failure {
    /// The errors of an entry without a ledger, which never stops but for a refusal of the
    /// allocator: that one is answered as a `SyntaxError` too.
    fn into_errors(self) -> Vec<SyntaxError> {
        match self {
            Failure::Errors(errors) => errors,
            Failure::Stopped(_) => vec![SyntaxError::new("out of memory while parsing", 1, 1)],
        }
    }
}

/// Parses `source`, UTF-16 code units (a UTF-8 source is converted by the caller), as a
/// `Script`: the tree, or every `SyntaxError` the parse found, at least one. A grammar error or a
/// refusal stops the parse and is the only error; the early errors of a source that parses are
/// all reported, in source order.
pub fn parse_script(source: &[u16], options: ParseOptions) -> Result<Script, Vec<SyntaxError>> {
    parse_script_with(source, options, &mut Unlimited).map_err(Failure::into_errors)
}

/// [`parse_script`] charged to `ledger` (the module's documentation).
pub fn parse_script_with(
    source: &[u16],
    options: ParseOptions,
    ledger: &mut dyn Ledger,
) -> Result<Script, Failure> {
    let meter = Meter::new(ledger);
    let parsed = parser::parse(source, options.max_nesting, &meter);
    analysed(parsed, &meter)
}

/// Parses `source` as the eval code of PerformEval (`#sec-performeval`, steps 9 to 11):
/// `ParseText(x, Script)`, strict when the caller is (`strict_caller`, IsStrict of the direct
/// call; false for an indirect `eval`) or when the source's directive prologue holds a Use Strict
/// Directive, with the early errors of a script of that strictness. The answer is a [`Script`]
/// as [`parse_script`] gives it: its tree, [`Script::strict`] (strictEval), and in
/// [`Script::scope`] the VarDeclaredNames and the function declarations to hoist that
/// EvalDeclarationInstantiation takes (`#sec-evaldeclarationinstantiation`). An empty source is
/// a script with no statement.
///
/// The checks of steps 6 and 10 on code inside a function, a method or a class field
/// initializer (`new.target`, `super`, `arguments` in a field initializer) need syntax the
/// parser refuses anyway: `new.target` and `super` before slice 8, class fields before slice 8;
/// so this entry has nothing more to check.
pub fn parse_eval(
    source: &[u16],
    strict_caller: bool,
    options: ParseOptions,
) -> Result<Script, Vec<SyntaxError>> {
    parse_eval_with(source, strict_caller, options, &mut Unlimited).map_err(Failure::into_errors)
}

/// [`parse_eval`] charged to `ledger` (the module's documentation).
pub fn parse_eval_with(
    source: &[u16],
    strict_caller: bool,
    options: ParseOptions,
    ledger: &mut dyn Ledger,
) -> Result<Script, Failure> {
    let meter = Meter::new(ledger);
    let parsed = parser::parse_eval(source, options.max_nesting, strict_caller, &meter);
    analysed(parsed, &meter)
}

/// Parses the parts of CreateDynamicFunction (`#sec-createdynamicfunction`) for kind `normal`
/// (the `Function` constructor): `parameters`, the parameter text P (the parameter arguments
/// joined with ","), alone as `FormalParameters[~Yield, ~Await]`; `body`, the body text, with a
/// line feed before and after, alone as `FunctionBody[~Yield, ~Await]`; then the whole source
/// text `function anonymous(<P>\n) {\n<body>\n}` as a `FunctionExpression` with its early
/// errors: so a parameter text that closes the list early (`a) {`), or a comment that spans the
/// two texts, is a SyntaxError although the whole text would parse, and a body with a Use
/// Strict Directive makes duplicate parameters an error
/// (`#sec-function-definitions-static-semantics-early-errors`). Each parse has the nesting limit
/// of `options` and the parser's one depth counter.
///
/// The answer is a [`Script`] of one expression statement whose expression is the function
/// expression, `FunctionId(0)`, the only function at the top of the tree; its own name is taken
/// off, since the name "anonymous" is given by SetFunctionName (step 35) and not bound inside
/// the function. The kinds `generator` (slice 7), `async` (slice 10) and `async generator`
/// (slice 10) come with their constructors and have no entry before then.
pub fn parse_dynamic_function(
    parameters: &[u16],
    body: &[u16],
    options: ParseOptions,
) -> Result<Script, Vec<SyntaxError>> {
    parse_dynamic_function_with(parameters, body, options, &mut Unlimited)
        .map_err(Failure::into_errors)
}

/// [`parse_dynamic_function`] charged to `ledger` (the module's documentation).
pub fn parse_dynamic_function_with(
    parameters: &[u16],
    body: &[u16],
    options: ParseOptions,
    ledger: &mut dyn Ledger,
) -> Result<Script, Failure> {
    let meter = Meter::new(ledger);
    let parsed = parser::parse_dynamic_function(parameters, body, options.max_nesting, &meter);
    analysed(parsed, &meter)
}

/// The early errors and the scope analysis of what a parse produced. A stop of the meter wins
/// over every other answer; on errors or a stop everything the meter holds is credited.
fn analysed(
    parsed: Result<parser::Parsed, SyntaxError>,
    meter: &Meter<'_>,
) -> Result<Script, Failure> {
    let answer = match parsed {
        Ok(parsed) => analyse_parsed(parsed, meter),
        Err(error) => Err(Failure::Errors(vec![error])),
    };
    let answer = match (answer, meter.stopped()) {
        (_, Some(stop)) => Err(Failure::Stopped(stop)),
        (answer, None) => answer,
    };
    match answer {
        Ok(mut script) => {
            script.charged = meter.held();
            Ok(script)
        }
        Err(failure) => {
            meter.credit_all();
            Err(failure)
        }
    }
}

fn analyse_parsed(parsed: parser::Parsed, meter: &Meter<'_>) -> Result<Script, Failure> {
    let mut ast = parsed.ast;
    let errors = early::check(&ast, &parsed.body, parsed.strict, meter);
    if !errors.is_empty() {
        return Err(Failure::Errors(errors));
    }
    let scope = scope::analyse(&mut ast, &parsed.body, meter);
    Ok(Script {
        body: parsed.body,
        strict: parsed.strict,
        scope,
        ast,
        charged: 0,
    })
}
