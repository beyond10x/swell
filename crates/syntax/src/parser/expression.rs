//! Expressions (clause 13, `#sec-ecmascript-language-expressions`): primary expressions with
//! array and object literals and function expressions, member access, `new` and calls, update
//! and unary operators, the binary operators by precedence, the conditional operator, assignment
//! and the comma operator. Slice 2 adds `??` (`#sec-binary-logical-operators`), optional chains
//! (`#sec-optional-chains`), the logical assignments (`#sec-assignment-operators`), a trailing
//! comma in `Arguments` (`#sec-left-hand-side-expressions`), and plain methods and shorthand
//! properties in object literals (`#sec-object-initializer`, `#sec-method-definitions`). Slice 6
//! adds template literals without a tag (`#sec-template-literals`) and arrow functions
//! (`#sec-arrow-function-definitions`): an `AssignmentExpression` whose operand is followed by
//! `=>` is an `ArrowFunction` when that operand is a `BindingIdentifier` or a
//! `CoverParenthesizedExpressionAndArrowParameterList` that reads again as
//! `ArrowFormalParameters` (`parser/function.rs`); a parenthesized expression takes `()` and a
//! trailing comma only before `=>`. An object literal on the left of `=` or in the head of a
//! for-in is read again as an ObjectAssignmentPattern (`#sec-destructuring-assignment`), and one
//! that stands as an arrow parameter as an ObjectBindingPattern
//! (`#sec-destructuring-binding-patterns`); a `CoverInitializedName` stands only there. An array
//! literal is read again the same way, as an ArrayAssignmentPattern or an ArrayBindingPattern,
//! its spread element last as the rest element; a spread element stands in an array literal and
//! among the arguments of a call or `new` (`#sec-argument-lists`). Slice 7 adds the
//! YieldExpression (`#sec-generator-function-definitions`) at the level of an
//! `AssignmentExpression` where [Yield] is set; elsewhere `yield` is an identifier; and the
//! generator methods of object literals (GeneratorMethod). Slice 10 adds the
//! AsyncFunctionExpression, the AsyncArrowFunction (`async` and an identifier before `=>`, or a
//! call of `async` read again as its parameters, `parser/function.rs`) and the AwaitExpression
//! (`#sec-async-function-definitions`), a prefix operator where [Await] is set; elsewhere
//! `await` is an identifier, and `async` is one wherever no `async` function or arrow function
//! starts; and the `async` methods of object literals (AsyncMethod). An `async function*` is an
//! AsyncGeneratorExpression (`#sec-async-generator-function-definitions`), `yield*` in it
//! included.
//!
//! Refused, each with its slice: `**` and `**=` (4), `class`, `super`, `new.target` (8),
//! `import.meta` and dynamic `import` (11).
//!
//! The functions that stay on the Rust stack while the parser recurses (each one an entry of the
//! depth counter) are kept small: they look at tokens through helpers that return a small value,
//! and build nodes through helpers, so their own frames hold little more than ids and one span.

use super::{Halt, Parser, R, copy_name, copy_number, copy_regexp, copy_string, copy_template};
use crate::ast::{
    AssignOp, BinaryOp, BoundName, Expr, ExprId, FunctionId, FunctionKind, LogicalOp, Name,
    Pattern, PatternElement, PatternKind, PatternProperty, PatternTarget, Property, PropertyKey,
    PropertyValue, Span, UnaryOp, UpdateOp,
};
use crate::error::SyntaxError;
use crate::ledger::HeapBytes;
use crate::token::{Keyword, Punct, TokenKind};

/// A property of an object literal read again as a binding pattern that is no
/// `BindingProperty`, or a target there that is no name and no nested pattern
/// (`#sec-destructuring-binding-patterns`); also a method, getter or setter in an
/// assignment pattern.
pub(super) const INVALID_DESTRUCTURING_TARGET: &str = "invalid destructuring target";

/// A target of an ObjectAssignmentPattern that is no simple assignment target and no nested
/// pattern (`#sec-destructuring-assignment-static-semantics-early-errors`).
const INVALID_ASSIGNMENT_PATTERN_TARGET: &str = "invalid destructuring assignment target";

/// A template string of a template without a tag holds a NotEscapeSequence, whose TV is
/// undefined (`#sec-static-semantics-template-early-errors`).
const NOT_ESCAPE_SEQUENCE: &str = "invalid escape sequence in a template literal without a tag";

/// `OptionalChain TemplateLiteral` or `?. TemplateLiteral`
/// (`#sec-left-hand-side-expressions-static-semantics-early-errors`).
const TEMPLATE_AFTER_OPTIONAL_CHAIN: &str = "template literal after an optional chain";

/// An operator of a binary expression waiting for its right operand.
pub(crate) struct Pending {
    op: Binary,
    precedence: u8,
    left: ExprId,
}

#[derive(Clone, Copy)]
enum Binary {
    Arithmetic(BinaryOp),
    Logical(LogicalOp),
}

/// What follows the operand of an `AssignmentExpression`.
#[derive(Clone, Copy)]
enum AssignTail {
    None,
    Conditional,
    Assign(AssignOp),
    /// `=>`: the operand is the parameters of an arrow function.
    Arrow,
}

/// What follows a member or call expression.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Suffix {
    None,
    Call,
    Dot,
    Index,
    /// `?.`
    Optional,
    /// A template token: a tagged template.
    Template,
}

/// What the `PropertyName` of a value property starts.
#[derive(Clone, Copy)]
enum Head {
    /// `PropertyName :`, its value to come.
    Value,
    /// A method, its parameters and body to come.
    Method(FunctionId),
    /// A shorthand property, complete.
    Done,
    /// A `CoverInitializedName` at its `=`, its initializer to come.
    Cover,
}

/// A prefix operator.
#[derive(Clone, Copy)]
pub(crate) enum Prefix {
    Unary(UnaryOp),
    Update(UpdateOp),
    /// The `await` of an AwaitExpression, `await UnaryExpression`.
    Await,
}

impl HeapBytes for Pending {
    fn heap_bytes(&self) -> usize {
        0
    }
}

impl HeapBytes for Prefix {
    fn heap_bytes(&self) -> usize {
        0
    }
}

type ExprParser<'a, 'm> = fn(&mut Parser<'a, 'm>) -> R<ExprId>;

/// A copy of the template string of the template token part `p` for a tagged template, charged
/// to `m`: no cooked string where the part holds a NotEscapeSequence.
fn copy_tagged(
    m: &crate::ledger::Meter<'_>,
    p: &crate::token::TemplatePart,
) -> Result<crate::ast::TaggedString, Halt> {
    let cooked = match p.not_escape {
        Some(_) => None,
        None => Some(m.copy(&p.string.cooked)?),
    };
    match m.copy(&p.string.raw) {
        Ok(raw) => Ok(crate::ast::TaggedString { cooked, raw }),
        Err(stop) => {
            m.release(cooked);
            Err(stop)
        }
    }
}

impl<'a, 'm> Parser<'a, 'm> {
    // ---- the comma operator and assignment -----------------------------------------------------

    /// `Expression`: assignment expressions joined by the comma operator (`#sec-comma-operator`).
    pub(super) fn expression(&mut self, no_in: bool) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let mut e = self.assignment(no_in)?;
        if self.is(Punct::Comma) {
            e = self.sequence_rest(e, start, no_in)?;
        }
        self.leave();
        Ok(e)
    }

    fn sequence_rest(&mut self, first: ExprId, start: Span, no_in: bool) -> R<ExprId> {
        self.enter()?;
        let id = self.reserve_sequence(first)?;
        while self.eat(Punct::Comma)? {
            let e = self.assignment(no_in)?;
            self.append_expr(id, Some(e))?;
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// A comma expression whose first operand is `first`, reserved.
    #[inline(never)]
    fn reserve_sequence(&mut self, first: ExprId) -> R<ExprId> {
        let id = self.reserve_expr(Expr::Sequence(Vec::new()));
        self.append_expr(id, Some(first))?;
        Ok(id)
    }

    /// Appends `e` to the list of the expression `id`: the elements of an array literal (`None`
    /// an elision), the arguments of a call or `new`, the operands of a comma expression. The
    /// growth of the list is charged first.
    #[inline(never)]
    fn append_expr(&mut self, id: ExprId, e: Option<ExprId>) -> R<()> {
        let m = self.m;
        let pushed = match (self.expr_mut(id), e) {
            (Some(Expr::Array(list)), e) => m.push(list, e),
            (
                Some(
                    Expr::Sequence(list)
                    | Expr::Call {
                        arguments: list, ..
                    }
                    | Expr::OptionalCall {
                        arguments: list, ..
                    }
                    | Expr::New {
                        arguments: Some(list),
                        ..
                    },
                ),
                Some(e),
            ) => m.push(list, e),
            _ => Ok(()),
        };
        self.charged(pushed)
    }

    /// Whether `e` is an object or array literal not in parentheses: as an assignment target it
    /// would be a destructuring pattern.
    pub(super) fn is_literal_pattern(&self, e: ExprId) -> bool {
        self.expr_node(e)
            .is_some_and(|n| !n.parenthesized && matches!(n.expr, Expr::Object(_) | Expr::Array(_)))
    }

    /// `AssignmentExpression` (`#sec-assignment-operators`), with the conditional operator
    /// (`#sec-conditional-operator`); where [Yield] is set, a `yield` written without an escape
    /// is a YieldExpression ([`Parser::yield_expression`]). `async` and an identifier after it on
    /// the same line start an AsyncArrowFunction ([`Parser::async_arrow_parameter`]).
    pub(super) fn assignment(&mut self, no_in: bool) -> R<ExprId> {
        if self.at_yield() {
            return self.yield_expression(no_in);
        }
        self.enter()?;
        let start = self.start();
        let left = if self.at_async_arrow()? {
            self.async_arrow_parameter()?
        } else {
            self.binary(no_in)?
        };
        let e = match self.assign_tail(left)? {
            AssignTail::None => left,
            AssignTail::Conditional => self.conditional_rest(left, start, no_in)?,
            AssignTail::Assign(op) => self.assign_rest(op, left, start, no_in)?,
            AssignTail::Arrow => self.arrow_function(left, start, no_in)?,
        };
        self.leave();
        Ok(e)
    }

    /// Whether the current token begins `async AsyncArrowBindingIdentifier =>`
    /// (`#sec-async-arrow-function-definitions`): `async` written without an escape, then a name
    /// with no line terminator before it. No other expression has a name after `async`, so the
    /// tokens after it decide. Not inlined, so that the look at the next token stays out of the
    /// frame of [`Parser::assignment`].
    #[inline(never)]
    fn at_async_arrow(&mut self) -> R<bool> {
        if !self.is_name("async") {
            return Ok(false);
        }
        let next = self.peek()?;
        Ok(!next.newline_before && matches!(next.kind, TokenKind::Name(_)))
    }

    /// The `async` at the current token and the `BindingIdentifier` after it, read as an
    /// identifier: the parameter of an `async` arrow function, whose `=>` must follow. The
    /// operand so starts after the start of its `AssignmentExpression`, which tells
    /// [`Parser::arrow_function`] that the arrow function is `async`.
    #[inline(never)]
    fn async_arrow_parameter(&mut self) -> R<ExprId> {
        self.advance()?;
        let parameter = self.leaf()?;
        if !self.is(Punct::Arrow) {
            return self.unexpected();
        }
        Ok(parameter)
    }

    /// Whether the current token begins a YieldExpression: `yield` written without an escape
    /// where [Yield] is set. Not inlined, so that the comparison of the name stays out of the
    /// frame of [`Parser::assignment`], which every nesting of an expression holds.
    #[inline(never)]
    fn at_yield(&self) -> bool {
        self.yields && self.is_name("yield")
    }

    /// `YieldExpression[In, Await]` (`#sec-generator-function-definitions`) at the current
    /// `yield`: `yield` alone when a line terminator follows it or the next token cannot begin
    /// an `AssignmentExpression`, `yield AssignmentExpression[?In, +Yield, ?Await]`, or, with no
    /// line terminator before the `*`, `yield * AssignmentExpression[?In, +Yield, ?Await]`.
    ///
    /// [`Parser::assignment`] hands the `yield` over before its own entry of the depth counter,
    /// so that the test stays out of its frame, which every nesting of an expression holds; this
    /// function enters the counter twice, once for the `AssignmentExpression` it stands as and
    /// once for itself, and so a nesting of `yield` takes two levels, as the frames of the two
    /// functions on its path do.
    #[inline(never)]
    fn yield_expression(&mut self, no_in: bool) -> R<ExprId> {
        self.enter()?;
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let delegate = self.yield_delegate()?;
        let argument = if delegate || self.starts_yield_operand() {
            Some(self.assignment(no_in)?)
        } else {
            None
        };
        let e = self.make(Expr::Yield { argument, delegate }, start)?;
        self.leave();
        self.leave();
        Ok(e)
    }

    /// Whether the token after `yield` is the `*` of `yield *`, on the same line; consumes it.
    #[inline(never)]
    fn yield_delegate(&mut self) -> R<bool> {
        if !self.tok.newline_before && self.is(Punct::Star) {
            self.advance()?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Whether the current token, after `yield`, is on the same line and can begin an
    /// `AssignmentExpression`: then it is the operand of the YieldExpression. A `/` or `/=` there
    /// begins a regular expression literal ([`Parser::primary`] reads it again).
    fn starts_yield_operand(&self) -> bool {
        if self.tok.newline_before {
            return false;
        }
        match &self.tok.kind {
            TokenKind::Name(_)
            | TokenKind::Number(_)
            | TokenKind::String(_)
            | TokenKind::RegExp(_)
            | TokenKind::NoSubstitutionTemplate(_)
            | TokenKind::TemplateHead(_) => true,
            TokenKind::Keyword(k) => matches!(
                k,
                Keyword::This
                    | Keyword::Function
                    | Keyword::Class
                    | Keyword::New
                    | Keyword::Typeof
                    | Keyword::Void
                    | Keyword::Delete
                    | Keyword::Null
                    | Keyword::True
                    | Keyword::False
                    | Keyword::Super
                    | Keyword::Import
            ),
            TokenKind::Punct(p) => matches!(
                p,
                Punct::LParen
                    | Punct::LBracket
                    | Punct::LBrace
                    | Punct::Plus
                    | Punct::Minus
                    | Punct::Bang
                    | Punct::Tilde
                    | Punct::PlusPlus
                    | Punct::MinusMinus
                    | Punct::Slash
                    | Punct::SlashEq
            ),
            TokenKind::Eof | TokenKind::TemplateMiddle(_) | TokenKind::TemplateTail(_) => false,
        }
    }

    /// What follows the operand `left` of an assignment expression; refuses `**=`. An object or
    /// array literal before `=` is read again as an assignment pattern.
    fn assign_tail(&mut self, left: ExprId) -> R<AssignTail> {
        let op = match &self.tok.kind {
            TokenKind::Punct(p) => match p {
                Punct::Question => return Ok(AssignTail::Conditional),
                Punct::Arrow => return Ok(AssignTail::Arrow),
                Punct::Eq => AssignOp::Assign,
                Punct::PlusEq => AssignOp::Compound(BinaryOp::Add),
                Punct::MinusEq => AssignOp::Compound(BinaryOp::Sub),
                Punct::StarEq => AssignOp::Compound(BinaryOp::Mul),
                Punct::SlashEq => AssignOp::Compound(BinaryOp::Div),
                Punct::PercentEq => AssignOp::Compound(BinaryOp::Rem),
                Punct::ShlEq => AssignOp::Compound(BinaryOp::Shl),
                Punct::ShrEq => AssignOp::Compound(BinaryOp::Shr),
                Punct::UShrEq => AssignOp::Compound(BinaryOp::UShr),
                Punct::AmpEq => AssignOp::Compound(BinaryOp::BitAnd),
                Punct::PipeEq => AssignOp::Compound(BinaryOp::BitOr),
                Punct::CaretEq => AssignOp::Compound(BinaryOp::BitXor),
                Punct::StarStarEq => return self.refuse("exponentiation operator", 4),
                Punct::AmpAmpEq => AssignOp::Logical(LogicalOp::And),
                Punct::PipePipeEq => AssignOp::Logical(LogicalOp::Or),
                Punct::QuestionQuestionEq => AssignOp::Logical(LogicalOp::Coalesce),
                _ => return Ok(AssignTail::None),
            },
            _ => return Ok(AssignTail::None),
        };
        if op == AssignOp::Assign && self.is_literal_pattern(left) {
            self.assignment_pattern(left)?;
        }
        self.advance()?;
        Ok(AssignTail::Assign(op))
    }

    fn assign_rest(&mut self, op: AssignOp, target: ExprId, start: Span, no_in: bool) -> R<ExprId> {
        self.enter()?;
        let value = self.assignment(no_in)?;
        let e = self.make(Expr::Assign { op, target, value }, start)?;
        self.leave();
        Ok(e)
    }

    /// `? AssignmentExpression : AssignmentExpression` after the test.
    fn conditional_rest(&mut self, test: ExprId, start: Span, no_in: bool) -> R<ExprId> {
        self.enter()?;
        self.advance()?;
        let consequent = self.assignment(false)?;
        self.expect(Punct::Colon)?;
        let alternate = self.assignment(no_in)?;
        let e = self.make(
            Expr::Conditional {
                test,
                consequent,
                alternate,
            },
            start,
        )?;
        self.leave();
        Ok(e)
    }

    // ---- binary operators ----------------------------------------------------------------------

    /// The binary operator at the current token with its precedence, `in` excepted under
    /// `no_in`; `**` is refused. `??` takes the precedence of `||`: the grammar lets neither
    /// stand as an operand of the other, which [`Parser::reduce`] checks.
    #[inline(never)]
    fn binary_operator(&mut self, no_in: bool) -> R<Option<(Binary, u8)>> {
        use BinaryOp::*;
        let op = match &self.tok.kind {
            TokenKind::Punct(p) => match p {
                Punct::QuestionQuestion => return self.logical(LogicalOp::Coalesce, 1),
                Punct::PipePipe => return self.logical(LogicalOp::Or, 1),
                Punct::AmpAmp => return self.logical(LogicalOp::And, 2),
                Punct::Pipe => BitOr,
                Punct::Caret => BitXor,
                Punct::Amp => BitAnd,
                Punct::EqEq => Eq,
                Punct::NotEq => NotEq,
                Punct::EqEqEq => StrictEq,
                Punct::NotEqEq => StrictNotEq,
                Punct::Lt => Lt,
                Punct::Gt => Gt,
                Punct::LtEq => LtEq,
                Punct::GtEq => GtEq,
                Punct::Shl => Shl,
                Punct::Shr => Shr,
                Punct::UShr => UShr,
                Punct::Plus => Add,
                Punct::Minus => Sub,
                Punct::Star => Mul,
                Punct::Slash => Div,
                Punct::Percent => Rem,
                Punct::StarStar => return self.refuse("exponentiation operator", 4),
                _ => return Ok(None),
            },
            TokenKind::Keyword(Keyword::Instanceof) => Instanceof,
            TokenKind::Keyword(Keyword::In) if !no_in => In,
            _ => return Ok(None),
        };
        let precedence = match op {
            BitOr => 3,
            BitXor => 4,
            BitAnd => 5,
            Eq | NotEq | StrictEq | StrictNotEq => 6,
            Lt | Gt | LtEq | GtEq | Instanceof | In => 7,
            Shl | Shr | UShr => 8,
            Add | Sub => 9,
            Mul | Div | Rem => 10,
        };
        self.advance()?;
        Ok(Some((Binary::Arithmetic(op), precedence)))
    }

    /// `||`, `&&` or `??` at the current token, consumed.
    fn logical(&mut self, op: LogicalOp, precedence: u8) -> R<Option<(Binary, u8)>> {
        self.advance()?;
        Ok(Some((Binary::Logical(op), precedence)))
    }

    /// The binary operators from `||` down to the multiplicative ones (13.6 to 13.13), all left
    /// associative, by precedence climbing over an operator stack: a chain of operators adds no
    /// recursion, and its depth in the tree is bounded by the node depth check.
    fn binary(&mut self, no_in: bool) -> R<ExprId> {
        self.enter()?;
        let base = self.operators.len();
        let mut left = self.unary()?;
        while self.binary_step(base, left, no_in)? {
            left = self.unary()?;
        }
        let e = self.reduce(base, 0, left)?;
        self.leave();
        Ok(e)
    }

    /// Reads the operator after `left`, if any: reduces the pending operators that bind at
    /// least as tightly and pushes it with `left`. False when no operator follows.
    #[inline(never)]
    fn binary_step(&mut self, base: usize, left: ExprId, no_in: bool) -> R<bool> {
        let Some((op, precedence)) = self.binary_operator(no_in)? else {
            return Ok(false);
        };
        let left = self.reduce(base, precedence, left)?;
        let m = self.m;
        let pushed = m.push(
            &mut self.operators,
            Pending {
                op,
                precedence,
                left,
            },
        );
        self.charged(pushed)?;
        Ok(true)
    }

    /// Combines the pending operators above `base` whose precedence is at least `precedence`
    /// with `right`, innermost first. A `??` with an `&&` or `||` operand, or the other way
    /// round, outside parentheses is an error: `CoalesceExpressionHead` is a `CoalesceExpression`
    /// or a `BitwiseORExpression`, and the operands of `&&` and `||` hold no `??`
    /// (`#sec-binary-logical-operators`). As `&&` and `||` bind more loosely than every other
    /// binary operator, two of them always meet as operator and operand.
    #[inline(never)]
    fn reduce(&mut self, base: usize, precedence: u8, mut right: ExprId) -> R<ExprId> {
        while self.operators.len() > base
            && self
                .operators
                .last()
                .is_some_and(|p| p.precedence >= precedence)
        {
            let Some(p) = self.operators.pop() else { break };
            let mut span = self.expr_span(p.left);
            span.end = self.expr_span(right).end;
            let left = p.left;
            let expr = match p.op {
                Binary::Arithmetic(op) => Expr::Binary { op, left, right },
                Binary::Logical(op) => {
                    let coalesce = op == LogicalOp::Coalesce;
                    if self.mixes_coalesce(left, coalesce) || self.mixes_coalesce(right, coalesce) {
                        return self.fail_at(crate::early::COALESCE_MIXED, span);
                    }
                    Expr::Logical { op, left, right }
                }
            };
            right = self.push_expr(expr, span)?;
        }
        Ok(right)
    }

    /// Whether `operand`, an operand of `??` (`coalesce`) or of `&&` or `||`, is an unparenthesized
    /// logical expression of the other kind.
    fn mixes_coalesce(&self, operand: ExprId, coalesce: bool) -> bool {
        self.expr_node(operand).is_some_and(|n| {
            !n.parenthesized
                && matches!(n.expr, Expr::Logical { op, .. } if (op == LogicalOp::Coalesce) != coalesce)
        })
    }

    // ---- unary, update, call and member expressions --------------------------------------------

    fn prefix(&self) -> Option<Prefix> {
        Some(match &self.tok.kind {
            TokenKind::Keyword(Keyword::Delete) => Prefix::Unary(UnaryOp::Delete),
            TokenKind::Keyword(Keyword::Void) => Prefix::Unary(UnaryOp::Void),
            TokenKind::Keyword(Keyword::Typeof) => Prefix::Unary(UnaryOp::Typeof),
            TokenKind::Punct(Punct::Plus) => Prefix::Unary(UnaryOp::Plus),
            TokenKind::Punct(Punct::Minus) => Prefix::Unary(UnaryOp::Minus),
            TokenKind::Punct(Punct::Tilde) => Prefix::Unary(UnaryOp::BitNot),
            TokenKind::Punct(Punct::Bang) => Prefix::Unary(UnaryOp::Not),
            TokenKind::Punct(Punct::PlusPlus) => Prefix::Update(UpdateOp::Increment),
            TokenKind::Punct(Punct::MinusMinus) => Prefix::Update(UpdateOp::Decrement),
            TokenKind::Name(n) if self.awaits && !self.tok.escaped && n == "await" => Prefix::Await,
            _ => return None,
        })
    }

    /// `UnaryExpression` and `UpdateExpression` (`#sec-unary-operators`,
    /// `#sec-update-expressions`), with the AwaitExpression where [Await] is set
    /// (`#sec-async-function-definitions`: `await UnaryExpression`, an operand it always has).
    /// A chain of prefix operators adds no recursion: they wait on the shared stack
    /// `Parser::prefixes` for their operand, and the chain's depth in the tree is bounded by the
    /// node depth check, as a chain of binary operators is.
    fn unary(&mut self) -> R<ExprId> {
        self.enter()?;
        let base = self.prefixes.len();
        self.prefix_operators()?;
        let start = self.start();
        let operand = self.left_hand_side()?;
        let e = self.postfix(operand, start)?;
        let e = self.apply_prefixes(base, e)?;
        self.leave();
        Ok(e)
    }

    /// Pushes the prefix operators at the current token, each with where it starts.
    #[inline(never)]
    fn prefix_operators(&mut self) -> R<()> {
        while let Some(prefix) = self.prefix() {
            let start = self.start();
            let m = self.m;
            let pushed = m.push(&mut self.prefixes, (prefix, start));
            self.charged(pushed)?;
            self.advance()?;
        }
        Ok(())
    }

    /// Applies the prefix operators above `base` to `operand`, innermost first.
    #[inline(never)]
    fn apply_prefixes(&mut self, base: usize, mut operand: ExprId) -> R<ExprId> {
        while self.prefixes.len() > base {
            let Some((prefix, start)) = self.prefixes.pop() else {
                break;
            };
            operand = self.make_prefix(prefix, operand, start)?;
        }
        Ok(operand)
    }

    #[inline(never)]
    fn make_prefix(&mut self, prefix: Prefix, operand: ExprId, start: Span) -> R<ExprId> {
        let expr = match prefix {
            Prefix::Unary(op) => Expr::Unary { op, operand },
            Prefix::Update(op) => Expr::Update {
                op,
                prefix: true,
                operand,
            },
            Prefix::Await => Expr::Await(operand),
        };
        self.make(expr, start)
    }

    /// A postfix `++` or `--` after `operand`, on the same line (a restricted production).
    #[inline(never)]
    fn postfix(&mut self, operand: ExprId, start: Span) -> R<ExprId> {
        let op = match &self.tok.kind {
            TokenKind::Punct(Punct::PlusPlus) if !self.tok.newline_before => UpdateOp::Increment,
            TokenKind::Punct(Punct::MinusMinus) if !self.tok.newline_before => UpdateOp::Decrement,
            _ => return Ok(operand),
        };
        self.advance()?;
        self.make(
            Expr::Update {
                op,
                prefix: false,
                operand,
            },
            start,
        )
    }

    /// What follows a member expression: a call (when `calls`), `.name`, `[index]`, `?.` or
    /// nothing.
    #[inline(never)]
    fn suffix(&mut self, calls: bool) -> R<Suffix> {
        Ok(match &self.tok.kind {
            TokenKind::Punct(Punct::LParen) if calls => Suffix::Call,
            TokenKind::Punct(Punct::Dot) => Suffix::Dot,
            TokenKind::Punct(Punct::LBracket) => Suffix::Index,
            TokenKind::Punct(Punct::QuestionDot) => Suffix::Optional,
            TokenKind::NoSubstitutionTemplate(_) | TokenKind::TemplateHead(_) => Suffix::Template,
            _ => Suffix::None,
        })
    }

    /// A template after a link of an optional chain: an early error
    /// (`#sec-left-hand-side-expressions-static-semantics-early-errors`), whatever is on the
    /// line before it: no semicolon is inserted there.
    #[inline(never)]
    fn template_after<T>(&mut self) -> R<T> {
        self.fail_here(TEMPLATE_AFTER_OPTIONAL_CHAIN)
    }

    /// A template after the member expression or call `tag`: a tagged template
    /// (`MemberExpression TemplateLiteral`, `CallExpression TemplateLiteral`,
    /// `#sec-tagged-templates`), its template strings and, after each `TemplateHead` and
    /// `TemplateMiddle`, an `Expression[+In]` that ends at the `}` the lexer reads again as the
    /// next template token; after an optional chain (`chain`) the early error of
    /// [`Parser::template_after`]. Each substitution is one more entry of the depth counter than
    /// its template, as in a template without a tag. Not inlined, so that the frames of the
    /// loops that call it stay as they were.
    #[inline(never)]
    fn tagged_rest(&mut self, tag: ExprId, start: Span, chain: bool) -> R<ExprId> {
        if chain {
            return self.template_after();
        }
        self.enter()?;
        let id = self.reserve_tagged(tag);
        while self.tagged_string(id)? {
            let e = self.expression(false)?;
            self.tagged_substitution(id, e)?;
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Reserves the node of a tagged template with tag `tag`.
    #[inline(never)]
    fn reserve_tagged(&mut self, tag: ExprId) -> ExprId {
        self.reserve_expr(Expr::TaggedTemplate {
            tag,
            strings: Vec::new(),
            substitutions: Vec::new(),
        })
    }

    /// Appends the template string of the current template token to the tagged template `id`
    /// and consumes the token; whether a substitution follows it. A part with a
    /// NotEscapeSequence keeps no cooked string: its TV is undefined (`#sec-static-semantics-tv`).
    #[inline(never)]
    fn tagged_string(&mut self, id: ExprId) -> R<bool> {
        let m = self.m;
        let (string, substitution) = match &self.tok.kind {
            TokenKind::NoSubstitutionTemplate(p) | TokenKind::TemplateTail(p) => {
                (copy_tagged(m, p), false)
            }
            TokenKind::TemplateHead(p) | TokenKind::TemplateMiddle(p) => (copy_tagged(m, p), true),
            _ => return self.unexpected(),
        };
        let string = self.charged(string)?;
        let pushed = match self.expr_mut(id) {
            Some(Expr::TaggedTemplate { strings, .. }) => m.push(strings, string),
            _ => {
                m.release(string);
                Ok(())
            }
        };
        self.charged(pushed)?;
        self.advance()?;
        Ok(substitution)
    }

    /// The substitution `e` of the tagged template `id`, which the `}` at the current token
    /// ends; the `}` is lexed again as a `TemplateMiddle` or `TemplateTail`.
    #[inline(never)]
    fn tagged_substitution(&mut self, id: ExprId, e: ExprId) -> R<()> {
        if !self.is(Punct::RBrace) {
            return self.unexpected();
        }
        let m = self.m;
        let pushed = match self.expr_mut(id) {
            Some(Expr::TaggedTemplate { substitutions, .. }) => m.push(substitutions, e),
            _ => Ok(()),
        };
        self.charged(pushed)?;
        self.rescan_template()
    }

    /// `LeftHandSideExpression`: a member or `new` expression followed by calls and property
    /// accesses (`#sec-left-hand-side-expressions`), or an `OptionalExpression`
    /// (`#sec-optional-chains`), which a first `?.` starts: the expression is then wrapped in
    /// [`Expr::OptionalChain`], so the chain ends where this expression ends (a parenthesized
    /// chain is a chain of its own).
    fn left_hand_side(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let mut e = self.member_or_new()?;
        let mut chain = false;
        loop {
            e = match self.suffix(true)? {
                Suffix::None => break,
                Suffix::Call => self.call_rest(e, start, false)?,
                Suffix::Dot => self.dot_rest(e, start)?,
                Suffix::Index => self.index_rest(e, start, false)?,
                Suffix::Optional => {
                    chain = true;
                    self.optional_rest(e, start)?
                }
                Suffix::Template => self.tagged_rest(e, start, chain)?,
            };
        }
        if chain {
            e = self.make(Expr::OptionalChain(e), start)?;
        }
        self.leave();
        Ok(e)
    }

    /// `?.` and the link after it: `?. IdentifierName`, `?. [ Expression ]` or `?. Arguments`.
    fn optional_rest(&mut self, object: ExprId, start: Span) -> R<ExprId> {
        self.enter()?;
        let e = match self.optional_link(object)? {
            Suffix::Call => self.call_rest(object, start, true)?,
            Suffix::Index => self.index_rest(object, start, true)?,
            _ => self.member_name(object, start, true)?,
        };
        self.leave();
        Ok(e)
    }

    /// Consumes the `?.` and tells which link follows. A `new` without `Arguments` may not be the
    /// start of an optional chain: `new a?.b` is no `OptionalExpression`
    /// (`#sec-optional-chains`).
    #[inline(never)]
    fn optional_link(&mut self, object: ExprId) -> R<Suffix> {
        let bare_new = self.expr_node(object).is_some_and(|n| {
            !n.parenthesized
                && matches!(
                    n.expr,
                    Expr::New {
                        arguments: None,
                        ..
                    }
                )
        });
        if bare_new {
            return self.unexpected();
        }
        self.advance()?;
        Ok(match &self.tok.kind {
            TokenKind::Punct(Punct::LParen) => Suffix::Call,
            TokenKind::Punct(Punct::LBracket) => Suffix::Index,
            TokenKind::Name(_) | TokenKind::Keyword(_) => Suffix::Dot,
            TokenKind::NoSubstitutionTemplate(_) | TokenKind::TemplateHead(_) => {
                return self.template_after();
            }
            _ => return self.unexpected(),
        })
    }

    /// `MemberExpression` and `new MemberExpression Arguments?` (`#sec-new-operator`).
    fn member_or_new(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let mut e = if self.is_keyword(Keyword::New) {
            self.new_rest(start)?
        } else {
            self.primary()?
        };
        loop {
            e = match self.suffix(false)? {
                Suffix::Dot => self.dot_rest(e, start)?,
                Suffix::Index => self.index_rest(e, start, false)?,
                Suffix::Template => self.tagged_rest(e, start, false)?,
                Suffix::None | Suffix::Call | Suffix::Optional => break,
            };
        }
        self.leave();
        Ok(e)
    }

    /// `new` at the current token: its callee and its `Arguments`, if any.
    fn new_rest(&mut self, start: Span) -> R<ExprId> {
        self.enter()?;
        self.advance()?;
        if self.is(Punct::Dot) {
            return self.refuse_at("new.target", 8, start);
        }
        let callee = self.member_or_new()?;
        let id = self.reserve_new(callee);
        if self.is(Punct::LParen) {
            self.arguments(id)?;
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Reserves the node of a `new` expression with callee `callee`, with arguments when an
    /// `Arguments` follows. A leaf, so the node is not built in [`Parser::new_rest`]'s frame.
    #[inline(never)]
    fn reserve_new(&mut self, callee: ExprId) -> ExprId {
        let with_arguments = self.is(Punct::LParen);
        self.reserve_expr(Expr::New {
            callee,
            arguments: with_arguments.then(Vec::new),
        })
    }

    /// `Arguments` after `callee`: a call, or the optional call `callee?.(...)` when `optional`.
    fn call_rest(&mut self, callee: ExprId, start: Span, optional: bool) -> R<ExprId> {
        self.enter()?;
        let id = self.reserve_call(callee, optional);
        self.arguments(id)?;
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    #[inline(never)]
    fn reserve_call(&mut self, callee: ExprId, optional: bool) -> ExprId {
        let arguments = Vec::new();
        self.reserve_expr(if optional {
            Expr::OptionalCall { callee, arguments }
        } else {
            Expr::Call { callee, arguments }
        })
    }

    /// `.IdentifierName` after `object`.
    fn dot_rest(&mut self, object: ExprId, start: Span) -> R<ExprId> {
        self.advance()?;
        self.member_name(object, start, false)
    }

    /// The `IdentifierName` of a property access on `object`, at the current token.
    #[inline(never)]
    fn member_name(&mut self, object: ExprId, start: Span, optional: bool) -> R<ExprId> {
        let m = self.m;
        let property = match &self.tok.kind {
            TokenKind::Name(n) => copy_name(m, n),
            TokenKind::Keyword(k) => copy_name(m, k.as_str()),
            _ => return self.unexpected(),
        };
        let property = self.charged(property)?;
        self.advance()?;
        let expr = if optional {
            Expr::OptionalMember { object, property }
        } else {
            Expr::Member { object, property }
        };
        self.make(expr, start)
    }

    /// `[ Expression ]` after `object`, or after its `?.` when `optional`.
    fn index_rest(&mut self, object: ExprId, start: Span, optional: bool) -> R<ExprId> {
        self.enter()?;
        self.advance()?;
        let index = self.expression(false)?;
        self.expect(Punct::RBracket)?;
        let e = self.make_index(object, index, start, optional)?;
        self.leave();
        Ok(e)
    }

    #[inline(never)]
    fn make_index(
        &mut self,
        object: ExprId,
        index: ExprId,
        start: Span,
        optional: bool,
    ) -> R<ExprId> {
        let expr = if optional {
            Expr::OptionalIndex { object, index }
        } else {
            Expr::Index { object, index }
        };
        self.make(expr, start)
    }

    /// `Arguments` (`#sec-argument-lists`), with a trailing comma allowed and spread elements.
    /// The arguments are appended to the call or `new` expression `id`.
    fn arguments(&mut self, id: ExprId) -> R<()> {
        self.enter()?;
        self.advance()?;
        let mut first = true;
        while self.argument_next(first)? {
            let e = self.element_or_spread()?;
            self.append_expr(id, Some(e))?;
            first = false;
        }
        self.leave();
        Ok(())
    }

    /// An `AssignmentExpression`, or at `...` a spread element `... AssignmentExpression`
    /// ([`Expr::Spread`]): an element of an array literal or an argument.
    fn element_or_spread(&mut self) -> R<ExprId> {
        if !self.is(Punct::Ellipsis) {
            return self.assignment(false);
        }
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let e = self.assignment(false)?;
        let e = self.make(Expr::Spread(e), start)?;
        self.leave();
        Ok(e)
    }

    /// Whether another argument follows; consumes the `,` before it, or the closing `)`.
    fn argument_next(&mut self, first: bool) -> R<bool> {
        if self.eat(Punct::RParen)? {
            return Ok(false);
        }
        if !first {
            self.expect(Punct::Comma)?;
            if self.eat(Punct::RParen)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // ---- primary expressions -------------------------------------------------------------------

    /// `PrimaryExpression` (`#sec-primary-expression`).
    fn primary(&mut self) -> R<ExprId> {
        self.enter()?;
        let parse = self.primary_kind()?;
        let e = parse(self)?;
        self.leave();
        Ok(e)
    }

    /// Which parser takes the primary expression at the current token; refuses the forms of
    /// later slices.
    #[inline(never)]
    fn primary_kind(&mut self) -> R<ExprParser<'a, 'm>> {
        Ok(match &self.tok.kind {
            TokenKind::Keyword(k) => match *k {
                Keyword::This | Keyword::Null | Keyword::True | Keyword::False => Parser::leaf,
                Keyword::Function => Parser::function_expression_node,
                Keyword::Class => return self.refuse("class", 8),
                Keyword::Super => return self.refuse("super", 8),
                Keyword::Import => {
                    let next = match &self.peek()?.kind {
                        TokenKind::Punct(p) => Some(*p),
                        _ => None,
                    };
                    return match next {
                        Some(Punct::Dot) => self.refuse("import.meta", 11),
                        Some(Punct::LParen) => self.refuse("dynamic import", 11),
                        _ => self.unexpected(),
                    };
                }
                _ => return self.unexpected(),
            },
            TokenKind::Name(n) => {
                // `async [no LineTerminator here] function`: an AsyncFunctionExpression, or with
                // a `*` an AsyncGeneratorExpression.
                if !self.tok.escaped && n == "async" {
                    let next = self.peek()?;
                    if !next.newline_before && next.kind == TokenKind::Keyword(Keyword::Function) {
                        return Ok(Parser::function_expression_node);
                    }
                }
                Parser::leaf
            }
            TokenKind::Number(_) | TokenKind::String(_) => Parser::leaf,
            TokenKind::Punct(Punct::Slash) | TokenKind::Punct(Punct::SlashEq) => {
                self.rescan_regexp()?;
                Parser::leaf
            }
            TokenKind::Punct(Punct::LBracket) => Parser::array,
            TokenKind::Punct(Punct::LBrace) => Parser::object,
            TokenKind::Punct(Punct::LParen) => Parser::parenthesized,
            TokenKind::NoSubstitutionTemplate(_) | TokenKind::TemplateHead(_) => Parser::template,
            _ => return self.unexpected(),
        })
    }

    /// `TemplateLiteral[~Tagged]` (`#sec-template-literals`): its template strings and, after
    /// each `TemplateHead` and `TemplateMiddle`, an `Expression[+In]` that ends at the `}` the
    /// lexer reads again as the next template token. Each substitution is one more entry of the
    /// depth counter than its template, as a parenthesized expression is.
    fn template(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let id = self.reserve_expr(Expr::Template {
            strings: Vec::new(),
            substitutions: Vec::new(),
        });
        while self.template_string(id)? {
            let e = self.expression(false)?;
            self.template_substitution(id, e)?;
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Appends the template string of the current template token to the template `id` and
    /// consumes the token; whether a substitution follows it. A NotEscapeSequence is an early
    /// error of a template without a tag (`#sec-static-semantics-template-early-errors`).
    #[inline(never)]
    fn template_string(&mut self, id: ExprId) -> R<bool> {
        let (not_escape, substitution) = match &self.tok.kind {
            TokenKind::NoSubstitutionTemplate(p) | TokenKind::TemplateTail(p) => {
                (p.not_escape, false)
            }
            TokenKind::TemplateHead(p) | TokenKind::TemplateMiddle(p) => (p.not_escape, true),
            _ => return self.unexpected(),
        };
        if let Some((line, column)) = not_escape {
            return self.fail(SyntaxError::new(NOT_ESCAPE_SEQUENCE, line, column));
        }
        let m = self.m;
        let string = match &self.tok.kind {
            TokenKind::NoSubstitutionTemplate(p)
            | TokenKind::TemplateTail(p)
            | TokenKind::TemplateHead(p)
            | TokenKind::TemplateMiddle(p) => copy_template(m, &p.string),
            _ => return self.unexpected(),
        };
        let string = self.charged(string)?;
        let pushed = match self.expr_mut(id) {
            Some(Expr::Template { strings, .. }) => m.push(strings, string),
            _ => {
                m.release(string);
                Ok(())
            }
        };
        self.charged(pushed)?;
        self.advance()?;
        Ok(substitution)
    }

    /// The substitution `e` of the template `id`, which the `}` at the current token ends; the
    /// `}` is lexed again as a `TemplateMiddle` or `TemplateTail`.
    #[inline(never)]
    fn template_substitution(&mut self, id: ExprId, e: ExprId) -> R<()> {
        if !self.is(Punct::RBrace) {
            return self.unexpected();
        }
        let m = self.m;
        let pushed = match self.expr_mut(id) {
            Some(Expr::Template { substitutions, .. }) => m.push(substitutions, e),
            _ => Ok(()),
        };
        self.charged(pushed)?;
        self.rescan_template()
    }

    /// A primary expression of one token: `this`, a literal or an identifier reference, which may
    /// not spell a reserved word with an escape.
    #[inline(never)]
    fn leaf(&mut self) -> R<ExprId> {
        self.check_escaped_keyword()?;
        let start = self.start();
        let m = self.m;
        let expr = match &self.tok.kind {
            TokenKind::Keyword(Keyword::This) => Ok(Expr::This),
            TokenKind::Keyword(Keyword::Null) => Ok(Expr::Null),
            TokenKind::Keyword(Keyword::True) => Ok(Expr::Boolean(true)),
            TokenKind::Keyword(Keyword::False) => Ok(Expr::Boolean(false)),
            TokenKind::Name(n) => copy_name(m, n).map(Expr::Identifier),
            TokenKind::Number(n) => copy_number(m, n).map(Expr::Number),
            TokenKind::String(s) => copy_string(m, s).map(Expr::String),
            TokenKind::RegExp(r) => copy_regexp(m, r).map(Expr::RegExp),
            _ => return self.unexpected(),
        };
        let expr = self.charged(expr)?;
        self.advance()?;
        self.make(expr, start)
    }

    fn function_expression_node(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let id = self.function_expression()?;
        let e = self.make(Expr::Function(id), start)?;
        self.leave();
        Ok(e)
    }

    /// `CoverParenthesizedExpressionAndArrowParameterList` (`#sec-grouping-operator`,
    /// `#sec-arrow-function-definitions`): `( Expression )`, and before `=>` also `()` and a
    /// trailing comma and a rest element last ([`Parser::cover_rest`]). The expression is marked
    /// parenthesized; an arrow function reads it again as its parameters
    /// ([`Parser::arrow_function`]).
    fn parenthesized(&mut self) -> R<ExprId> {
        self.enter()?;
        let e = match self.paren_open()? {
            Some(empty) => empty,
            None => self.paren_list()?,
        };
        self.expect(Punct::RParen)?;
        self.paren_close(e)?;
        self.leave();
        Ok(e)
    }

    /// The `(` of a parenthesized expression. `()` stands only before `=>`: it answers an empty
    /// comma expression, the empty parameter list the arrow function reads; `(...` answers the
    /// rest element alone ([`Parser::cover_rest`]).
    #[inline(never)]
    fn paren_open(&mut self) -> R<Option<ExprId>> {
        let start = self.start();
        self.advance()?;
        if self.is(Punct::RParen) {
            if self.peek()?.kind == TokenKind::Punct(Punct::Arrow) {
                let empty = self.push_expr(Expr::Sequence(Vec::new()), start)?;
                return Ok(Some(empty));
            }
            return self.unexpected();
        }
        if self.is(Punct::Ellipsis) {
            return self.cover_rest().map(Some);
        }
        Ok(None)
    }

    /// The rest element `... BindingIdentifier` or `... BindingPattern` of a parenthesized list
    /// at its `...`, which only `ArrowFormalParameters` take: the `)` and a `=>` must follow it
    /// (`#sec-arrow-function-definitions`). It is held as the identifier, or as a spread of the
    /// literal the pattern covers, its span starting at the `...`, which marks it for the arrow
    /// function that reads the list again ([`Parser::arrow_function`]).
    #[inline(never)]
    fn cover_rest(&mut self) -> R<ExprId> {
        let start = self.start();
        self.advance()?;
        let pattern = self.pattern_literal()?;
        let name = match pattern {
            Some(_) => None,
            None => Some(self.binding_identifier()?),
        };
        if !self.is(Punct::RParen) {
            return self.fail_here(super::function::REST_NOT_LAST);
        }
        if self.peek()?.kind != TokenKind::Punct(Punct::Arrow) {
            return self.fail_here(super::function::REST_WITHOUT_ARROW);
        }
        let span = self.finish(start);
        match (pattern, name) {
            (Some(pattern), _) => self.push_expr(Expr::Spread(pattern), span),
            (None, Some(name)) => self.push_expr(Expr::Identifier(name.name), span),
            (None, None) => self.unexpected(),
        }
    }

    /// The `Expression` inside the parentheses, with a trailing comma allowed before `) =>`.
    fn paren_list(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let mut e = self.assignment(false)?;
        if self.is(Punct::Comma) {
            e = self.paren_sequence(e, start)?;
        }
        self.leave();
        Ok(e)
    }

    /// The comma expression inside parentheses whose first operand is `first`.
    fn paren_sequence(&mut self, first: ExprId, start: Span) -> R<ExprId> {
        self.enter()?;
        let id = self.reserve_sequence(first)?;
        while self.paren_comma()? {
            let e = if self.is(Punct::Ellipsis) {
                self.cover_rest()?
            } else {
                self.assignment(false)?
            };
            self.append_expr(id, Some(e))?;
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Whether another operand follows inside parentheses: consumes its `,`. A `,` before the
    /// `)` is a trailing comma, which only `ArrowFormalParameters` take, so `=>` must follow the
    /// `)`. A `...` after the `,` starts the rest element ([`Parser::cover_rest`]).
    #[inline(never)]
    fn paren_comma(&mut self) -> R<bool> {
        if !self.eat(Punct::Comma)? {
            return Ok(false);
        }
        if self.is(Punct::RParen) {
            if self.peek()?.kind == TokenKind::Punct(Punct::Arrow) {
                return Ok(false);
            }
            return self.unexpected();
        }
        Ok(true)
    }

    /// Marks `e` parenthesized after its `)`. An expression in two pairs of parentheses is no
    /// `ArrowFormalParameters`: before `=>` it is an error here, so that an arrow function finds
    /// a parenthesized operand in exactly one pair.
    #[inline(never)]
    fn paren_close(&mut self, e: ExprId) -> R<()> {
        let twice = self.expr_node(e).is_some_and(|n| n.parenthesized);
        if twice && self.is(Punct::Arrow) {
            return self.unexpected();
        }
        if let Some(node) = self.ast.exprs.get_mut(e.0 as usize) {
            node.parenthesized = true;
        }
        Ok(())
    }

    /// `ArrayLiteral` (`#sec-array-initializer`), with elisions and spread elements.
    pub(super) fn array(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let id = self.reserve_expr(Expr::Array(Vec::new()));
        while self.element_next(id)? {
            let e = self.element_or_spread()?;
            self.append_expr(id, Some(e))?;
            self.element_end(e)?;
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Consumes elisions, appended to the array `id`, and the closing `]`; whether an element
    /// follows.
    fn element_next(&mut self, id: ExprId) -> R<bool> {
        loop {
            if self.eat(Punct::RBracket)? {
                return Ok(false);
            }
            if !self.eat(Punct::Comma)? {
                break;
            }
            self.append_expr(id, None)?;
        }
        Ok(true)
    }

    /// The `,` after the element `e`, unless the `]` follows; a spread element's span is
    /// extended over it (see [`Expr::Spread`]).
    #[inline(never)]
    fn element_end(&mut self, e: ExprId) -> R<()> {
        if !self.is(Punct::RBracket) {
            self.expect(Punct::Comma)?;
            let end = self.prev_end;
            if let Some(node) = self.ast.exprs.get_mut(e.0 as usize)
                && matches!(node.expr, Expr::Spread(_))
            {
                node.span.end = end;
            }
        }
        Ok(())
    }

    /// `ObjectLiteral` (`#sec-object-initializer`): `PropertyName : AssignmentExpression` and the
    /// accessors `get` and `set`.
    pub(super) fn object(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let id = self.reserve_expr(Expr::Object(Vec::new()));
        while !self.eat(Punct::RBrace)? {
            self.property(id)?;
            if !self.is(Punct::RBrace) {
                self.expect(Punct::Comma)?;
                self.spread_comma(id);
            }
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Whether `kind` can be a `PropertyName` that is not computed.
    fn is_property_name(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Name(_)
                | TokenKind::Keyword(_)
                | TokenKind::String(_)
                | TokenKind::Number(_)
        )
    }

    /// The `PropertyName` at the current token, consumed.
    #[inline(never)]
    fn property_key(&mut self) -> R<PropertyKey> {
        let m = self.m;
        let key = match &self.tok.kind {
            TokenKind::Name(n) => copy_name(m, n).map(PropertyKey::Identifier),
            TokenKind::Keyword(k) => copy_name(m, k.as_str()).map(PropertyKey::Identifier),
            TokenKind::String(s) => copy_string(m, s).map(PropertyKey::String),
            TokenKind::Number(n) => copy_number(m, n).map(PropertyKey::Number),
            _ => return self.unexpected(),
        };
        let key = self.charged(key)?;
        self.advance()?;
        Ok(key)
    }

    /// The key of a property: the ComputedPropertyName `computed` read before, or the
    /// `PropertyName` at the current token, consumed.
    fn key_or(&mut self, computed: Option<ExprId>) -> R<PropertyKey> {
        match computed {
            Some(e) => Ok(PropertyKey::Computed(e)),
            None => self.property_key(),
        }
    }

    /// A ComputedPropertyName `[ AssignmentExpression ]` at the current token, consumed, or
    /// `None` at any other token (`#sec-object-initializer`). One entry of the depth counter whose
    /// frame holds the id alone.
    fn computed_name(&mut self) -> R<Option<ExprId>> {
        if !self.is(Punct::LBracket) {
            return Ok(None);
        }
        self.enter()?;
        self.advance()?;
        let key = self.assignment(false)?;
        self.expect(Punct::RBracket)?;
        self.leave();
        Ok(Some(key))
    }

    /// `PropertyDefinition : ... AssignmentExpression` at the `...`, appended to `object`.
    fn spread_property(&mut self, object: ExprId, start: Span) -> R<()> {
        self.enter()?;
        self.advance()?;
        let value = self.assignment(false)?;
        self.push_spread(object, value, start)?;
        self.property_end(object);
        self.leave();
        Ok(())
    }

    /// Appends the spread property of `value` to `object`; its key, empty, takes no memory.
    #[inline(never)]
    fn push_spread(&mut self, object: ExprId, value: ExprId, start: Span) -> R<()> {
        let property = Property {
            key: PropertyKey::Identifier(String::new()),
            value: PropertyValue::Spread(value),
            span: start,
        };
        self.push_property(object, property)
    }

    /// Extends a spread property last in `object` over the `,` read after it (see
    /// [`PropertyValue::Spread`]).
    #[inline(never)]
    fn spread_comma(&mut self, object: ExprId) {
        let end = self.prev_end;
        if let Some(p) = self.properties(object).and_then(|l| l.last_mut())
            && matches!(p.value, PropertyValue::Spread(_))
        {
            p.span.end = end;
        }
    }

    /// Append `property` to the object literal `object`, the growth of its list charged first.
    #[inline(never)]
    fn push_property(&mut self, object: ExprId, property: Property) -> R<()> {
        let m = self.m;
        let pushed = match self.properties(object) {
            Some(list) => m.push(list, property),
            None => {
                m.release(property);
                Ok(())
            }
        };
        self.charged(pushed)
    }

    /// One `PropertyDefinition`, appended to the object literal `object`.
    fn property(&mut self, object: ExprId) -> R<()> {
        self.enter()?;
        let start = self.start();
        match self.accessor_kind()? {
            Some(FunctionKind::Method) => self.async_method(object, start)?,
            Some(kind) => self.accessor_rest(object, kind, start)?,
            None if self.is(Punct::Star) => self.generator_method(object, start)?,
            None if self.is(Punct::Ellipsis) => self.spread_property(object, start)?,
            None => self.value_property_rest(object, start)?,
        }
        self.leave();
        Ok(())
    }

    /// Whether the property at the current token is a getter or setter: `get` or `set`, written
    /// without an escape, followed by a property name or a computed one; or an `async` method
    /// (`FunctionKind::Method`, AsyncMethod, `#sec-async-function-definitions`): `async`, written
    /// without an escape, followed with no line terminator before it by a property name or a
    /// computed one; the `async` is consumed. `async *` is an async generator method
    /// (AsyncGeneratorMethod, `#sec-async-generator-function-definitions`), whose `*`
    /// [`Parser::async_method`] reads.
    #[inline(never)]
    fn accessor_kind(&mut self) -> R<Option<FunctionKind>> {
        let plain = !self.tok.escaped;
        let word = match &self.tok.kind {
            TokenKind::Name(n) if plain && n == "get" => FunctionKind::Getter,
            TokenKind::Name(n) if plain && n == "set" => FunctionKind::Setter,
            TokenKind::Name(n) if plain && n == "async" => {
                let next = self.peek()?;
                if !next.newline_before
                    && (Parser::is_property_name(&next.kind)
                        || matches!(
                            next.kind,
                            TokenKind::Punct(Punct::Star) | TokenKind::Punct(Punct::LBracket)
                        ))
                {
                    self.advance()?;
                    return Ok(Some(FunctionKind::Method));
                }
                return Ok(None);
            }
            _ => return Ok(None),
        };
        let next = &self.peek()?.kind;
        let (computed, named) = (
            *next == TokenKind::Punct(Punct::LBracket),
            Parser::is_property_name(next),
        );
        if computed || named {
            self.advance()?;
            return Ok(Some(word));
        }
        Ok(None)
    }

    fn properties(&mut self, object: ExprId) -> Option<&mut Vec<Property>> {
        match self.expr_mut(object) {
            Some(Expr::Object(list)) => Some(list),
            _ => None,
        }
    }

    /// Sets the end of the last property of `object` at the previous token.
    fn property_end(&mut self, object: ExprId) {
        let end = self.prev_end;
        if let Some(p) = self.properties(object).and_then(|l| l.last_mut()) {
            p.span.end = end.max(p.span.start);
        }
    }

    /// The name, parameters and body of a getter or setter after `get` or `set`, appended to
    /// `object`.
    fn accessor_rest(&mut self, object: ExprId, kind: FunctionKind, start: Span) -> R<()> {
        self.enter()?;
        let computed = self.computed_name()?;
        let id = self.accessor_head(object, kind, start, computed)?;
        self.function_rest(id)?;
        self.property_end(object);
        self.leave();
        Ok(())
    }

    /// The `PropertyName` of an accessor: numbers its function and appends the property.
    #[inline(never)]
    fn accessor_head(
        &mut self,
        object: ExprId,
        kind: FunctionKind,
        start: Span,
        computed: Option<ExprId>,
    ) -> R<crate::ast::FunctionId> {
        let key = self.key_or(computed)?;
        let id = self.begin_function(kind, start, None);
        let value = match kind {
            FunctionKind::Getter => PropertyValue::Get(id),
            _ => PropertyValue::Set(id),
        };
        self.push_property(
            object,
            Property {
                key,
                value,
                span: start,
            },
        )?;
        Ok(id)
    }

    /// `MethodDefinition : GeneratorMethod`, `* ClassElementName ( UniqueFormalParameters[+Yield,
    /// ~Await] ) { GeneratorBody }` (`#sec-generator-function-definitions`), at its `*`,
    /// appended to `object`. The name is read with the [Yield] of the code around the literal
    /// (`ClassElementName[?Yield, ?Await]`), so in a generator a `yield` in a computed name is a
    /// YieldExpression of that generator; the parameters and the body are those of a method
    /// whose `generator` is set, which [`Parser::function_rest`] reads with [Yield] set.
    fn generator_method(&mut self, object: ExprId, start: Span) -> R<()> {
        self.enter()?;
        self.advance()?;
        let computed = self.computed_name()?;
        let id = self.method_head(object, start, computed, true, false)?;
        self.function_rest(id)?;
        self.property_end(object);
        self.leave();
        Ok(())
    }

    /// `MethodDefinition : AsyncMethod`, `async ClassElementName ( UniqueFormalParameters[~Yield,
    /// +Await] ) { AsyncFunctionBody }` (`#sec-async-function-definitions`), after its `async`,
    /// appended to `object`; its function's span starts at the `async` (`start`). The name is
    /// read with the [Yield] and [Await] of the code around the literal
    /// (`ClassElementName[?Yield, ?Await]`), so in an `async` function an `await` in a computed
    /// name is an AwaitExpression of that function; the parameters and the body are those of a
    /// method whose `asynchronous` is set, which [`Parser::function_rest`] reads with [Await]
    /// set and [Yield] cleared. A `*` after the `async` makes it `MethodDefinition :
    /// AsyncGeneratorMethod`, `async * ClassElementName ( UniqueFormalParameters[+Yield, +Await] )
    /// { AsyncGeneratorBody }` (`#sec-async-generator-function-definitions`): the `*` is
    /// consumed, its name is read as above, and its function has both `generator` and
    /// `asynchronous` set, so its parameters and body are read with [Yield] and [Await] set.
    fn async_method(&mut self, object: ExprId, start: Span) -> R<()> {
        self.enter()?;
        let generator = self.is(Punct::Star);
        if generator {
            self.advance()?;
        }
        let computed = self.computed_name()?;
        let id = self.method_head(object, start, computed, generator, true)?;
        self.function_rest(id)?;
        self.property_end(object);
        self.leave();
        Ok(())
    }

    /// The `ClassElementName` of a generator method (`generator`), an `async` method
    /// (`asynchronous`) or an async generator method (both), a `PropertyName` (a
    /// `PrivateIdentifier` stands only in a class): numbers its function, a method with those
    /// flags, and appends the property.
    #[inline(never)]
    fn method_head(
        &mut self,
        object: ExprId,
        start: Span,
        computed: Option<ExprId>,
        generator: bool,
        asynchronous: bool,
    ) -> R<FunctionId> {
        let key = self.key_or(computed)?;
        let id = self.begin_function(FunctionKind::Method, start, None);
        if let Some(f) = self.ast.functions.get_mut(id.0 as usize) {
            f.generator = generator;
            f.asynchronous = asynchronous;
        }
        self.push_property(
            object,
            Property {
                key,
                value: PropertyValue::Method(id),
                span: start,
            },
        )?;
        Ok(id)
    }

    /// `PropertyName : AssignmentExpression`, a plain method `PropertyName ( params ) { body }`
    /// or a shorthand `IdentifierReference`, appended to `object`.
    fn value_property_rest(&mut self, object: ExprId, start: Span) -> R<()> {
        self.enter()?;
        let computed = self.computed_name()?;
        match self.value_property_head(object, start, computed)? {
            Head::Value => {
                let value = self.assignment(false)?;
                self.set_property_value(object, value);
            }
            Head::Method(id) => self.function_rest(id)?,
            Head::Done => {}
            Head::Cover => {
                self.advance()?;
                let init = self.assignment(false)?;
                self.set_cover_initializer(object, init);
            }
        }
        self.property_end(object);
        self.leave();
        Ok(())
    }

    #[inline(never)]
    fn set_property_value(&mut self, object: ExprId, value: ExprId) {
        if let Some(p) = self.properties(object).and_then(|l| l.last_mut()) {
            p.value = PropertyValue::Init(value);
        }
    }

    /// Sets the initializer of the `CoverInitializedName` last in `object`.
    #[inline(never)]
    fn set_cover_initializer(&mut self, object: ExprId, init: ExprId) {
        if let Some(p) = self.properties(object).and_then(|l| l.last_mut())
            && let PropertyValue::CoverInitialized(_, value) = &mut p.value
        {
            *value = init;
        }
    }

    /// The `PropertyName` of a value property and what follows it, appended to `object`: with
    /// `:` its value comes next, with `(` a method's parameters (`#sec-method-definitions`); a
    /// shorthand property is complete (`#sec-object-initializer`). A shorthand property is an
    /// `IdentifierReference`, so no reserved word; so is the name of a `CoverInitializedName`
    /// (`a = 1`), whose initializer comes next: it stands only where the literal is read again
    /// as a pattern, and anywhere else it is an early error (`src/early.rs`).
    #[inline(never)]
    fn value_property_head(
        &mut self,
        object: ExprId,
        start: Span,
        computed: Option<ExprId>,
    ) -> R<Head> {
        let name_token = self.start();
        let escaped = self.tok.escaped;
        let keyword = match &self.tok.kind {
            TokenKind::Keyword(k) => Some(*k),
            _ => None,
        };
        let key = self.key_or(computed)?;
        let (head, value) = if self.eat(Punct::Colon)? {
            (Head::Value, PropertyValue::Init(ExprId(u32::MAX)))
        } else if self.is(Punct::LParen) {
            let id = self.begin_function(FunctionKind::Method, start, None);
            (Head::Method(id), PropertyValue::Method(id))
        } else if self.is(Punct::Comma) || self.is(Punct::RBrace) {
            let e = self.shorthand(&key, keyword, escaped, name_token)?;
            (Head::Done, PropertyValue::Shorthand(e))
        } else if self.is(Punct::Eq)
            && keyword.is_none()
            && matches!(key, PropertyKey::Identifier(_))
        {
            let e = self.shorthand(&key, keyword, escaped, name_token)?;
            (
                Head::Cover,
                PropertyValue::CoverInitialized(e, ExprId(u32::MAX)),
            )
        } else {
            return self.unexpected();
        };
        self.push_property(
            object,
            Property {
                key,
                value,
                span: start,
            },
        )?;
        Ok(head)
    }

    /// The `IdentifierReference` of a shorthand property with key `key`, read from the token at
    /// `at` (a `keyword` token, or a name `escaped` or not).
    #[inline(never)]
    fn shorthand(
        &mut self,
        key: &PropertyKey,
        keyword: Option<Keyword>,
        escaped: bool,
        at: Span,
    ) -> R<ExprId> {
        if let Some(k) = keyword {
            return self.fail_at(
                format!("reserved word {} as a shorthand property", k.as_str()),
                at,
            );
        }
        let PropertyKey::Identifier(name) = key else {
            return self.unexpected();
        };
        if escaped && Keyword::from_name(name).is_some() {
            return self.fail_at(crate::early::escaped_keyword(name), at);
        }
        let span = self.finish(at);
        let name = copy_name(self.m, name);
        let name = self.charged(name)?;
        self.push_expr(Expr::Identifier(name), span)
    }

    // ---- object patterns -----------------------------------------------------------------------

    /// The literal `left` before `=` or in a for-in head, read again as an
    /// ObjectAssignmentPattern or an ArrayAssignmentPattern (`#sec-destructuring-assignment`).
    #[inline(never)]
    pub(super) fn assignment_pattern(&mut self, left: ExprId) -> R<()> {
        self.reread_as_pattern(left, PatternKind::Assignment)
    }

    /// Reads the object literal `root` again as an object pattern of `kind`, in place, with each
    /// object literal that stands as a target in it; an ObjectAssignmentPattern read before (a
    /// target with a default, `{a: {b} = c}`) takes `kind` too. Each `PropertyDefinition` must
    /// be one of a pattern: `PropertyName : AssignmentElement` (its value an assignment target
    /// with an optional default), a shorthand or a `CoverInitializedName`; a binding pattern's
    /// targets are names not in parentheses and nested patterns
    /// (`#sec-destructuring-binding-patterns`). The nodes of the first reading that no longer
    /// stand in the tree (an `=` that became a default) stay in the arena; the name a binding
    /// takes moves out of its identifier node. The work list is charged as it grows, and the
    /// list of each node is made anew, charged first, the old one credited.
    #[inline(never)]
    pub(super) fn reread_as_pattern(&mut self, root: ExprId, kind: PatternKind) -> R<()> {
        let made = self.m.with_capacity(4);
        let mut work: Vec<ExprId> = self.charged(made)?;
        work.push(root);
        let mut done = Ok(());
        while let Some(node) = work.pop() {
            done = self.pattern_node(node, kind, &mut work);
            if done.is_err() {
                break;
            }
        }
        self.m.release(work);
        done
    }

    /// One node of [`Parser::reread_as_pattern`]: its properties read as pattern properties; the
    /// nested patterns onto `work`.
    #[inline(never)]
    fn pattern_node(&mut self, node: ExprId, kind: PatternKind, work: &mut Vec<ExprId>) -> R<()> {
        let m = self.m;
        let mut rest = false;
        let (mut literal, mut read) = match self.expr_mut(node) {
            Some(Expr::Object(list)) => (std::mem::take(list), Vec::new()),
            Some(Expr::ObjectPattern {
                properties,
                kind: k,
                rest: r,
            }) => {
                if *k == kind {
                    return Ok(());
                }
                rest = *r;
                (Vec::new(), std::mem::take(properties))
            }
            Some(Expr::Array(_) | Expr::ArrayPattern { .. }) => {
                return self.array_pattern_node(node, kind, work);
            }
            _ => return self.unexpected(),
        };
        let mut properties: Vec<PatternProperty> =
            match m.with_capacity(literal.len().max(read.len())) {
                Ok(list) => list,
                Err(stop) => {
                    m.release(literal);
                    m.release(read);
                    return self.charged(Err(stop));
                }
            };
        // Taken from the end of each list, reversed first, so that what is left is released.
        literal.reverse();
        read.reverse();
        let mut done = Ok(());
        while let Some(p) = literal.pop() {
            let (target, default) = match p.value {
                PropertyValue::Init(e) => self.split_default(e),
                PropertyValue::Shorthand(e) => (e, None),
                PropertyValue::CoverInitialized(e, init) => (e, Some(init)),
                PropertyValue::Spread(e) => match self.rest_target(e, p.span, literal.is_empty()) {
                    Ok(()) => {
                        rest = true;
                        (e, None)
                    }
                    Err(stop) => {
                        m.release(p);
                        done = Err(stop);
                        break;
                    }
                },
                PropertyValue::Get(_) | PropertyValue::Set(_) | PropertyValue::Method(_) => {
                    let span = p.span;
                    m.release(p);
                    done = self.fail_at(INVALID_DESTRUCTURING_TARGET, span);
                    break;
                }
            };
            match self.pattern_target(target, kind, work) {
                Ok(target) => properties.push(PatternProperty {
                    key: p.key,
                    target,
                    default,
                    span: p.span,
                }),
                Err(stop) => {
                    m.release(p);
                    done = Err(stop);
                    break;
                }
            }
        }
        while done.is_ok()
            && let Some(p) = read.pop()
        {
            let target = match p.target {
                PatternTarget::Expr(e) => self.pattern_target(e, kind, work),
                binding @ PatternTarget::Binding(_) => Ok(binding),
            };
            match target {
                Ok(target) => properties.push(PatternProperty { target, ..p }),
                Err(stop) => {
                    m.release(p.key);
                    done = Err(stop);
                }
            }
        }
        m.release(literal);
        m.release(read);
        if done.is_err() {
            m.release(properties);
            return done;
        }
        match self.expr_mut(node) {
            Some(expr) => {
                *expr = Expr::ObjectPattern {
                    properties,
                    kind,
                    rest,
                }
            }
            None => m.release(properties),
        }
        Ok(())
    }

    /// One array literal or array pattern `node` of [`Parser::reread_as_pattern`]: its elements
    /// read as the elements of an array pattern of `kind`, an elision kept, a spread element last
    /// as the rest element; the nested patterns onto `work`. The list is made anew, charged
    /// first, the old one credited.
    #[inline(never)]
    fn array_pattern_node(
        &mut self,
        node: ExprId,
        kind: PatternKind,
        work: &mut Vec<ExprId>,
    ) -> R<()> {
        let m = self.m;
        let mut rest = false;
        let (mut literal, mut read) = match self.expr_mut(node) {
            Some(Expr::Array(list)) => (std::mem::take(list), Vec::new()),
            Some(Expr::ArrayPattern {
                elements,
                kind: k,
                rest: r,
            }) => {
                if *k == kind {
                    return Ok(());
                }
                rest = *r;
                (Vec::new(), std::mem::take(elements))
            }
            _ => return self.unexpected(),
        };
        let mut elements: Vec<Option<PatternElement>> =
            match m.with_capacity(literal.len().max(read.len())) {
                Ok(list) => list,
                Err(stop) => {
                    m.release(literal);
                    m.release(read);
                    return self.charged(Err(stop));
                }
            };
        literal.reverse();
        read.reverse();
        let mut done = Ok(());
        while let Some(element) = literal.pop() {
            let Some(e) = element else {
                elements.push(None);
                continue;
            };
            let span = self.expr_span(e);
            let (target, default) = match self.expr_node(e).map(|n| &n.expr) {
                Some(Expr::Spread(inner)) => {
                    let inner = *inner;
                    match self.rest_element(inner, span, literal.is_empty()) {
                        Ok(()) => {
                            rest = true;
                            (inner, None)
                        }
                        Err(stop) => {
                            done = Err(stop);
                            break;
                        }
                    }
                }
                _ => self.split_default(e),
            };
            match self.pattern_target(target, kind, work) {
                Ok(target) => elements.push(Some(PatternElement {
                    target,
                    default,
                    span,
                })),
                Err(stop) => {
                    done = Err(stop);
                    break;
                }
            }
        }
        while done.is_ok()
            && let Some(element) = read.pop()
        {
            let Some(p) = element else {
                elements.push(None);
                continue;
            };
            let target = match p.target {
                PatternTarget::Expr(e) => self.pattern_target(e, kind, work),
                binding @ PatternTarget::Binding(_) => Ok(binding),
            };
            match target {
                Ok(target) => elements.push(Some(PatternElement { target, ..p })),
                Err(stop) => done = Err(stop),
            }
        }
        m.release(literal);
        m.release(read);
        if done.is_err() {
            m.release(elements);
            return done;
        }
        match self.expr_mut(node) {
            Some(expr) => {
                *expr = Expr::ArrayPattern {
                    elements,
                    kind,
                    rest,
                }
            }
            None => m.release(elements),
        }
        Ok(())
    }

    /// The spread element `... e` at `span`, `last` when no element follows it, read again as
    /// the rest element of an array pattern: last and without a comma after it, its target no
    /// assignment outside parentheses (the constants `REST_ELEMENT_*` of `src/early.rs`); the
    /// target itself, which may be a nested pattern, is checked as any other.
    #[inline(never)]
    fn rest_element(&mut self, e: ExprId, span: Span, last: bool) -> R<()> {
        let comma = span
            .end
            .checked_sub(1)
            .and_then(|at| self.src.get(at as usize))
            == Some(&u16::from(b','));
        if !last || comma {
            return self.fail_at(crate::early::REST_ELEMENT_NOT_LAST, span);
        }
        match self.expr_node(e) {
            Some(n) if !n.parenthesized && matches!(n.expr, Expr::Assign { .. }) => {
                let at = n.span;
                self.fail_at(crate::early::REST_ELEMENT_INITIALIZER, at)
            }
            Some(_) => Ok(()),
            None => self.unexpected(),
        }
    }

    /// The spread property `... e` at `span`, `last` when no property follows it, read again as
    /// the rest property of a pattern: last and without a comma after it, its target no
    /// assignment and no object or array literal outside parentheses (the constants
    /// `REST_PROPERTY_*` of `src/early.rs`); the target itself is checked as any other.
    #[inline(never)]
    fn rest_target(&mut self, e: ExprId, span: Span, last: bool) -> R<()> {
        let comma = span
            .end
            .checked_sub(1)
            .and_then(|at| self.src.get(at as usize))
            == Some(&u16::from(b','));
        if !last || comma {
            return self.fail_at(crate::early::REST_PROPERTY_NOT_LAST, span);
        }
        let Some(n) = self.expr_node(e) else {
            return self.unexpected();
        };
        let at = n.span;
        let message = match n.expr {
            _ if n.parenthesized => return Ok(()),
            Expr::Assign { .. } => crate::early::REST_PROPERTY_INITIALIZER,
            Expr::Object(_) | Expr::ObjectPattern { .. } | Expr::Array(_) => {
                crate::early::REST_PROPERTY_PATTERN
            }
            _ => return Ok(()),
        };
        self.fail_at(message, at)
    }

    /// The target and the default of the value `e` of `PropertyName : AssignmentExpression`
    /// read as an `AssignmentElement` or a `BindingElement`: an `=` not in parentheses splits
    /// into both.
    fn split_default(&self, e: ExprId) -> (ExprId, Option<ExprId>) {
        match self.expr_node(e) {
            Some(n) if !n.parenthesized => match n.expr {
                Expr::Assign {
                    op: AssignOp::Assign,
                    target,
                    value,
                } => (target, Some(value)),
                _ => (e, None),
            },
            _ => (e, None),
        }
    }

    /// The expression `t` as the target of a property or element of a pattern of `kind`: a
    /// nested object or array literal or pattern not in parentheses (pushed onto `work`), a
    /// name, or for an assignment pattern a property access; a name in a
    /// binding pattern moves into the target. Anything else is a `SyntaxError`
    /// (`#sec-destructuring-assignment-static-semantics-early-errors`,
    /// `#sec-destructuring-binding-patterns`).
    #[inline(never)]
    fn pattern_target(
        &mut self,
        t: ExprId,
        kind: PatternKind,
        work: &mut Vec<ExprId>,
    ) -> R<PatternTarget> {
        let Some(n) = self.expr_node(t) else {
            return self.unexpected();
        };
        let (span, parenthesized) = (n.span, n.parenthesized);
        let assignment = kind == PatternKind::Assignment;
        match &n.expr {
            Expr::Object(_)
            | Expr::ObjectPattern { .. }
            | Expr::Array(_)
            | Expr::ArrayPattern { .. }
                if !parenthesized =>
            {
                let pushed = self.m.push(work, t);
                self.charged(pushed)?;
                Ok(PatternTarget::Expr(t))
            }
            Expr::Identifier(_) | Expr::Member { .. } | Expr::Index { .. } if assignment => {
                Ok(PatternTarget::Expr(t))
            }
            Expr::Identifier(_) if !parenthesized => {
                let name = match self.expr_mut(t) {
                    Some(Expr::Identifier(name)) => std::mem::take(name),
                    _ => String::new(),
                };
                Ok(PatternTarget::Binding(Name { name, span }))
            }
            _ if assignment => self.fail_at(INVALID_ASSIGNMENT_PATTERN_TARGET, span),
            _ => self.fail_at(INVALID_DESTRUCTURING_TARGET, span),
        }
    }

    /// The binding pattern `root`, read by [`Parser::reread_as_pattern`], with its BoundNames in source
    /// order and whether it holds a default; each name a copy charged first, the work list
    /// charged as it grows.
    #[inline(never)]
    pub(super) fn bound_names(&mut self, root: ExprId) -> R<Pattern> {
        let m = self.m;
        let made = m.with_capacity(4);
        let mut work: Vec<(ExprId, usize)> = self.charged(made)?;
        work.push((root, 0));
        let mut names: Vec<BoundName> = Vec::new();
        let mut expressions = false;
        let mut done: Result<(), Halt> = Ok(());
        while let Some((node, next)) = work.pop() {
            // The target of the property or element `next` of `node`; `None` for an elision.
            let target = match self.ast.expr(node).map(|n| &n.expr) {
                Some(Expr::ObjectPattern { properties, .. }) => match properties.get(next) {
                    Some(p) => {
                        expressions |=
                            p.default.is_some() || matches!(p.key, PropertyKey::Computed(_));
                        Some(&p.target)
                    }
                    None => continue,
                },
                Some(Expr::ArrayPattern { elements, .. }) => match elements.get(next) {
                    Some(element) => element.as_ref().map(|p| {
                        expressions |= p.default.is_some();
                        &p.target
                    }),
                    None => continue,
                },
                _ => continue,
            };
            done = m.grow(&mut work).map(|()| work.push((node, next + 1)));
            if done.is_err() {
                break;
            }
            let Some(target) = target else {
                continue;
            };
            match target {
                PatternTarget::Binding(name) => {
                    let property = u32::try_from(next).unwrap_or(u32::MAX);
                    done = copy_name(m, &name.name).and_then(|copy| {
                        m.push(
                            &mut names,
                            BoundName {
                                name: Name {
                                    name: copy,
                                    span: name.span,
                                },
                                pattern: node,
                                property,
                            },
                        )
                    });
                }
                PatternTarget::Expr(e) => done = m.grow(&mut work).map(|()| work.push((*e, 0))),
            }
            if done.is_err() {
                break;
            }
        }
        m.release(work);
        if let Err(stop) = done {
            m.release(names);
            return self.charged(Err(stop));
        }
        let span = self.expr_span(root);
        Ok(Pattern {
            pattern: root,
            names,
            expressions,
            span,
        })
    }

    /// Pushes `expr`, which started at `start` and ends at the previous token.
    #[inline(never)]
    pub(super) fn make(&mut self, expr: Expr, start: Span) -> R<ExprId> {
        let span = self.finish(start);
        self.push_expr(expr, span)
    }
}
