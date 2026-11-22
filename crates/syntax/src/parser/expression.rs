//! Expressions (clause 13, `#sec-ecmascript-language-expressions`): primary expressions with
//! array and object literals and function expressions, member access, `new` and calls, update
//! and unary operators, the binary operators by precedence, the conditional operator, assignment
//! and the comma operator. Slice 2 adds `??` (`#sec-binary-logical-operators`), optional chains
//! (`#sec-optional-chains`), the logical assignments (`#sec-assignment-operators`), a trailing
//! comma in `Arguments` (`#sec-left-hand-side-expressions`), and plain methods and shorthand
//! properties in object literals (`#sec-object-initializer`, `#sec-method-definitions`). Slice 6
//! adds template literals without a tag (`#sec-template-literals`).
//!
//! Refused, each with its slice: `**` and `**=` (4), arrow functions, spread elements,
//! destructuring assignment, computed property names, a `CoverInitializedName` and a tagged
//! template (6), generator methods (7), `class`, `super`, `new.target` (8), `async` functions and
//! `await` (10), `import.meta` and dynamic `import` (11).
//!
//! The functions that stay on the Rust stack while the parser recurses (each one an entry of the
//! depth counter) are kept small: they look at tokens through helpers that return a small value,
//! and build nodes through helpers, so their own frames hold little more than ids and one span.

use super::{Parser, R, copy_name, copy_number, copy_regexp, copy_string, copy_template};
use crate::ast::{
    AssignOp, BinaryOp, Expr, ExprId, FunctionId, FunctionKind, LogicalOp, Property, PropertyKey,
    PropertyValue, Span, UnaryOp, UpdateOp,
};
use crate::error::SyntaxError;
use crate::ledger::HeapBytes;
use crate::token::{Keyword, Punct, TokenKind};

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
}

/// A prefix operator.
#[derive(Clone, Copy)]
pub(crate) enum Prefix {
    Unary(UnaryOp),
    Update(UpdateOp),
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
    /// (`#sec-conditional-operator`).
    pub(super) fn assignment(&mut self, no_in: bool) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let left = self.binary(no_in)?;
        let e = match self.assign_tail(left)? {
            AssignTail::None => left,
            AssignTail::Conditional => self.conditional_rest(left, start, no_in)?,
            AssignTail::Assign(op) => self.assign_rest(op, left, start, no_in)?,
        };
        self.leave();
        Ok(e)
    }

    /// What follows the operand `left` of an assignment expression; refuses `=>`, `**=` and a
    /// destructuring assignment.
    fn assign_tail(&mut self, left: ExprId) -> R<AssignTail> {
        let op = match &self.tok.kind {
            TokenKind::Punct(p) => match p {
                Punct::Question => return Ok(AssignTail::Conditional),
                Punct::Arrow => return self.refuse("arrow function", 6),
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
            let at = self.expr_span(left);
            return self.refuse_at("destructuring pattern", 6, at);
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
            _ => return None,
        })
    }

    /// `UnaryExpression` and `UpdateExpression` (`#sec-unary-operators`,
    /// `#sec-update-expressions`).
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

    /// A template after a member expression, a call or a link of an optional chain: a tagged
    /// template (`MemberExpression TemplateLiteral`, `CallExpression TemplateLiteral`,
    /// `#sec-tagged-templates`) is refused; after an optional chain it is an early error
    /// (`#sec-left-hand-side-expressions-static-semantics-early-errors`), whatever is on the
    /// line before it: no semicolon is inserted there.
    #[inline(never)]
    fn template_after<T>(&mut self, chain: bool) -> R<T> {
        if chain {
            return self.fail_here(TEMPLATE_AFTER_OPTIONAL_CHAIN);
        }
        self.refuse("tagged template", 6)
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
                Suffix::Template => return self.template_after(chain),
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
                return self.template_after(true);
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
                Suffix::Template => return self.template_after(false),
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

    /// `Arguments` (`#sec-argument-lists`), with a trailing comma allowed; spread is refused.
    /// The arguments are appended to the call or `new` expression `id`.
    fn arguments(&mut self, id: ExprId) -> R<()> {
        self.enter()?;
        self.advance()?;
        let mut first = true;
        while self.argument_next(first)? {
            let e = self.assignment(false)?;
            self.append_expr(id, Some(e))?;
            first = false;
        }
        self.leave();
        Ok(())
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
        if self.is(Punct::Ellipsis) {
            return self.refuse("spread element", 6);
        }
        Ok(true)
    }

    // ---- primary expressions -------------------------------------------------------------------

    /// Whether `kind`, after `await` on the same line, starts an operand: `await` then is an
    /// `AwaitExpression` (slice 10), never the identifier `await` of a script.
    fn starts_await_operand(kind: &TokenKind) -> bool {
        matches!(
            kind,
            TokenKind::Name(_)
                | TokenKind::Number(_)
                | TokenKind::String(_)
                | TokenKind::Punct(Punct::Bang)
                | TokenKind::Punct(Punct::Tilde)
                | TokenKind::Keyword(
                    Keyword::This
                        | Keyword::Function
                        | Keyword::New
                        | Keyword::Typeof
                        | Keyword::Void
                        | Keyword::Delete
                        | Keyword::Null
                        | Keyword::True
                        | Keyword::False
                        | Keyword::Class
                        | Keyword::Super
                )
        )
    }

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
                if !self.tok.escaped && (n == "async" || n == "await") {
                    let is_async = n == "async";
                    let next = self.peek()?;
                    if !next.newline_before {
                        if is_async
                            && matches!(
                                next.kind,
                                TokenKind::Keyword(Keyword::Function) | TokenKind::Name(_)
                            )
                        {
                            return self.refuse("async function", 10);
                        }
                        if !is_async && Parser::starts_await_operand(&next.kind) {
                            return self.refuse("await expression", 10);
                        }
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

    /// `( Expression )` (`#sec-grouping-operator`). The arrow parameter lists it covers are
    /// refused.
    fn parenthesized(&mut self) -> R<ExprId> {
        self.enter()?;
        self.paren_open()?;
        let e = self.expression(false)?;
        self.expect(Punct::RParen)?;
        if let Some(node) = self.ast.exprs.get_mut(e.0 as usize) {
            node.parenthesized = true;
        }
        self.leave();
        Ok(e)
    }

    /// The `(` of a parenthesized expression; refuses `()` and `(...` before `=>`.
    fn paren_open(&mut self) -> R<()> {
        self.advance()?;
        if self.is(Punct::RParen) {
            if self.peek()?.kind == TokenKind::Punct(Punct::Arrow) {
                return self.refuse("arrow function", 6);
            }
            return self.unexpected();
        }
        if self.is(Punct::Ellipsis) {
            return self.refuse("arrow function", 6);
        }
        Ok(())
    }

    /// `ArrayLiteral` (`#sec-array-initializer`), with elisions; spread is refused.
    fn array(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let id = self.reserve_expr(Expr::Array(Vec::new()));
        while self.element_next(id)? {
            let e = self.assignment(false)?;
            self.append_expr(id, Some(e))?;
            self.element_end()?;
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
        if self.is(Punct::Ellipsis) {
            return self.refuse("spread element", 6);
        }
        Ok(true)
    }

    /// The `,` after an element, unless the `]` follows.
    fn element_end(&mut self) -> R<()> {
        if !self.is(Punct::RBracket) {
            self.expect(Punct::Comma)?;
        }
        Ok(())
    }

    /// `ObjectLiteral` (`#sec-object-initializer`): `PropertyName : AssignmentExpression` and the
    /// accessors `get` and `set`.
    fn object(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        self.advance()?;
        let id = self.reserve_expr(Expr::Object(Vec::new()));
        while !self.eat(Punct::RBrace)? {
            self.property(id)?;
            if !self.is(Punct::RBrace) {
                self.expect(Punct::Comma)?;
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
            TokenKind::Punct(Punct::LBracket) => return self.refuse("computed property name", 6),
            _ => return self.unexpected(),
        };
        let key = self.charged(key)?;
        self.advance()?;
        Ok(key)
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
            Some(kind) => self.accessor_rest(object, kind, start)?,
            None => self.value_property_rest(object, start)?,
        }
        self.leave();
        Ok(())
    }

    /// Whether the property at the current token is a getter or setter: `get` or `set`, written
    /// without an escape, followed by a property name. Refuses spread, computed names, generator
    /// and `async` methods.
    #[inline(never)]
    fn accessor_kind(&mut self) -> R<Option<FunctionKind>> {
        let plain = !self.tok.escaped;
        let word = match &self.tok.kind {
            TokenKind::Punct(Punct::Ellipsis) => return self.refuse("spread element", 6),
            TokenKind::Punct(Punct::LBracket) => return self.refuse("computed property name", 6),
            TokenKind::Punct(Punct::Star) => return self.refuse("generator function", 7),
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
                    return self.refuse("async function", 10);
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
        if computed {
            self.advance()?;
            return self.refuse("computed property name", 6);
        }
        if named {
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
        let id = self.accessor_head(object, kind, start)?;
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
    ) -> R<crate::ast::FunctionId> {
        let key = self.property_key()?;
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

    /// `PropertyName : AssignmentExpression`, a plain method `PropertyName ( params ) { body }`
    /// or a shorthand `IdentifierReference`, appended to `object`.
    fn value_property_rest(&mut self, object: ExprId, start: Span) -> R<()> {
        self.enter()?;
        match self.value_property_head(object, start)? {
            Head::Value => {
                let value = self.assignment(false)?;
                self.set_property_value(object, value);
            }
            Head::Method(id) => self.function_rest(id)?,
            Head::Done => {}
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

    /// The `PropertyName` of a value property and what follows it, appended to `object`: with
    /// `:` its value comes next, with `(` a method's parameters (`#sec-method-definitions`); a
    /// shorthand property is complete (`#sec-object-initializer`). A shorthand property is an
    /// `IdentifierReference`, so no reserved word, and a `CoverInitializedName` (`a = 1`) is
    /// refused (slice 6, with destructuring assignment, the only place it may stand).
    #[inline(never)]
    fn value_property_head(&mut self, object: ExprId, start: Span) -> R<Head> {
        let name_token = self.start();
        let escaped = self.tok.escaped;
        let keyword = match &self.tok.kind {
            TokenKind::Keyword(k) => Some(*k),
            _ => None,
        };
        let key = self.property_key()?;
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
            return self.refuse_at("CoverInitializedName", 6, start);
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

    /// Pushes `expr`, which started at `start` and ends at the previous token.
    #[inline(never)]
    pub(super) fn make(&mut self, expr: Expr, start: Span) -> R<ExprId> {
        let span = self.finish(start);
        self.push_expr(expr, span)
    }
}
