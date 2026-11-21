//! Expressions (clause 13, `#sec-ecmascript-language-expressions`): primary expressions with
//! array and object literals and function expressions, member access, `new` and calls, update
//! and unary operators, the binary operators by precedence, the conditional operator, assignment
//! and the comma operator.
//!
//! Refused, each with its slice: arrow functions, spread elements, destructuring assignment,
//! computed and shorthand properties and methods (6), generator methods (7), `class`, `super`,
//! `new.target` (8), `async` functions and `await` (10), `import.meta` and dynamic `import` (11),
//! and `**`, `??`, `?.`, logical assignment and trailing commas in argument lists (2, the slice of
//! their directories: the design names no feature rule for them).
//!
//! The functions that stay on the Rust stack while the parser recurses (each one an entry of the
//! depth counter) are kept small: they look at tokens through helpers that return a small value,
//! and build nodes through helpers, so their own frames hold little more than ids and one span.

use super::{Parser, R};
use crate::ast::{
    AssignOp, BinaryOp, Expr, ExprId, FunctionKind, LogicalOp, Property, PropertyKey,
    PropertyValue, Span, UnaryOp, UpdateOp,
};
use crate::token::{Keyword, Punct, TokenKind};

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
}

/// A prefix operator.
#[derive(Clone, Copy)]
pub(crate) enum Prefix {
    Unary(UnaryOp),
    Update(UpdateOp),
}

type ExprParser<'a> = fn(&mut Parser<'a>) -> R<ExprId>;

impl<'a> Parser<'a> {
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
        let id = self.reserve_expr(Expr::Sequence(vec![first]));
        while self.eat(Punct::Comma)? {
            let e = self.assignment(no_in)?;
            self.append_expr(id, Some(e));
        }
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// Appends `e` to the list of the expression `id`: the elements of an array literal (`None`
    /// an elision), the arguments of a call or `new`, the operands of a comma expression.
    fn append_expr(&mut self, id: ExprId, e: Option<ExprId>) {
        match (self.expr_mut(id), e) {
            (Some(Expr::Array(list)), e) => list.push(e),
            (
                Some(
                    Expr::Sequence(list)
                    | Expr::Call {
                        arguments: list, ..
                    }
                    | Expr::New {
                        arguments: Some(list),
                        ..
                    },
                ),
                Some(e),
            ) => list.push(e),
            _ => {}
        }
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

    /// What follows the operand `left` of an assignment expression; refuses `=>`, `**=`, the
    /// logical assignments and a destructuring assignment.
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
                Punct::StarStarEq => return self.refuse("exponentiation operator", 2),
                Punct::AmpAmpEq | Punct::PipePipeEq | Punct::QuestionQuestionEq => {
                    return self.refuse("logical assignment", 2);
                }
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
    /// `no_in`; `**` and `??` are refused.
    #[inline(never)]
    fn binary_operator(&mut self, no_in: bool) -> R<Option<(Binary, u8)>> {
        use BinaryOp::*;
        let op = match &self.tok.kind {
            TokenKind::Punct(p) => match p {
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
                Punct::StarStar => return self.refuse("exponentiation operator", 2),
                Punct::QuestionQuestion => {
                    return self.refuse("nullish coalescing operator", 2);
                }
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

    /// `||` or `&&` at the current token, consumed.
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
        self.operators.push(Pending {
            op,
            precedence,
            left,
        });
        Ok(true)
    }

    /// Combines the pending operators above `base` whose precedence is at least `precedence`
    /// with `right`, innermost first.
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
                Binary::Logical(op) => Expr::Logical { op, left, right },
            };
            right = self.push_expr(expr, span)?;
        }
        Ok(right)
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
            self.prefixes.push((prefix, start));
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

    /// What follows a member expression: a call (when `calls`), `.name`, `[index]` or nothing;
    /// `?.` is refused.
    #[inline(never)]
    fn suffix(&mut self, calls: bool) -> R<Suffix> {
        Ok(match &self.tok.kind {
            TokenKind::Punct(Punct::LParen) if calls => Suffix::Call,
            TokenKind::Punct(Punct::Dot) => Suffix::Dot,
            TokenKind::Punct(Punct::LBracket) => Suffix::Index,
            TokenKind::Punct(Punct::QuestionDot) => return self.refuse("optional chaining", 2),
            _ => Suffix::None,
        })
    }

    /// `LeftHandSideExpression`: a member or `new` expression followed by calls and property
    /// accesses (`#sec-left-hand-side-expressions`).
    fn left_hand_side(&mut self) -> R<ExprId> {
        self.enter()?;
        let start = self.start();
        let mut e = self.member_or_new()?;
        loop {
            e = match self.suffix(true)? {
                Suffix::None => break,
                Suffix::Call => self.call_rest(e, start)?,
                Suffix::Dot => self.dot_rest(e, start)?,
                Suffix::Index => self.index_rest(e, start)?,
            };
        }
        self.leave();
        Ok(e)
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
                Suffix::Index => self.index_rest(e, start)?,
                Suffix::None | Suffix::Call => break,
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

    fn call_rest(&mut self, callee: ExprId, start: Span) -> R<ExprId> {
        self.enter()?;
        let id = self.reserve_expr(Expr::Call {
            callee,
            arguments: Vec::new(),
        });
        self.arguments(id)?;
        let id = self.complete_expr(id, start)?;
        self.leave();
        Ok(id)
    }

    /// `.IdentifierName` after `object`.
    fn dot_rest(&mut self, object: ExprId, start: Span) -> R<ExprId> {
        self.advance()?;
        let property = match &self.tok.kind {
            TokenKind::Name(n) => n.clone(),
            TokenKind::Keyword(k) => k.as_str().to_string(),
            _ => return self.unexpected(),
        };
        self.advance()?;
        self.make(Expr::Member { object, property }, start)
    }

    /// `[ Expression ]` after `object`.
    fn index_rest(&mut self, object: ExprId, start: Span) -> R<ExprId> {
        self.enter()?;
        self.advance()?;
        let index = self.expression(false)?;
        self.expect(Punct::RBracket)?;
        let e = self.make(Expr::Index { object, index }, start)?;
        self.leave();
        Ok(e)
    }

    /// `Arguments` (`#sec-argument-lists`); spread and a trailing comma are refused.
    /// The arguments are appended to the call or `new` expression `id`.
    fn arguments(&mut self, id: ExprId) -> R<()> {
        self.enter()?;
        self.advance()?;
        let mut first = true;
        while self.argument_next(first)? {
            let e = self.assignment(false)?;
            self.append_expr(id, Some(e));
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
            if self.is(Punct::RParen) {
                return self.refuse("trailing comma in an argument or parameter list", 2);
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
    fn primary_kind(&mut self) -> R<ExprParser<'a>> {
        Ok(match &self.tok.kind {
            TokenKind::Keyword(k) => match *k {
                Keyword::This | Keyword::Null | Keyword::True | Keyword::False => Parser::leaf,
                Keyword::Function => Parser::function_expression_node,
                Keyword::Class => return self.refuse("class", 8),
                Keyword::Super => return self.refuse("super", 8),
                Keyword::Import => {
                    let next = self.peek()?.kind.clone();
                    return match next {
                        TokenKind::Punct(Punct::Dot) => self.refuse("import.meta", 11),
                        TokenKind::Punct(Punct::LParen) => self.refuse("dynamic import", 11),
                        _ => self.unexpected(),
                    };
                }
                _ => return self.unexpected(),
            },
            TokenKind::Name(n) => {
                if n == "async" || n == "await" {
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
                } else if n == "arguments" && self.functions > 0 {
                    return self.refuse("reference to arguments inside a function", 2);
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
            _ => return self.unexpected(),
        })
    }

    /// A primary expression of one token: `this`, a literal or an identifier reference.
    #[inline(never)]
    fn leaf(&mut self) -> R<ExprId> {
        let start = self.start();
        let expr = match &self.tok.kind {
            TokenKind::Keyword(Keyword::This) => Expr::This,
            TokenKind::Keyword(Keyword::Null) => Expr::Null,
            TokenKind::Keyword(Keyword::True) => Expr::Boolean(true),
            TokenKind::Keyword(Keyword::False) => Expr::Boolean(false),
            TokenKind::Name(n) => Expr::Identifier(n.clone()),
            TokenKind::Number(n) => Expr::Number(n.clone()),
            TokenKind::String(s) => Expr::String(s.clone()),
            TokenKind::RegExp(r) => Expr::RegExp(r.clone()),
            _ => return self.unexpected(),
        };
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
            self.append_expr(id, Some(e));
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
            self.append_expr(id, None);
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
        let key = match &self.tok.kind {
            TokenKind::Name(n) => PropertyKey::Identifier(n.clone()),
            TokenKind::Keyword(k) => PropertyKey::Identifier(k.as_str().to_string()),
            TokenKind::String(s) => PropertyKey::String(s.clone()),
            TokenKind::Number(n) => PropertyKey::Number(n.clone()),
            TokenKind::Punct(Punct::LBracket) => return self.refuse("computed property name", 6),
            _ => return self.unexpected(),
        };
        self.advance()?;
        Ok(key)
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

    /// Whether the property at the current token is a getter or setter: `get` or `set` followed
    /// by a property name. Refuses spread, computed names, generator and `async` methods.
    #[inline(never)]
    fn accessor_kind(&mut self) -> R<Option<FunctionKind>> {
        let word = match &self.tok.kind {
            TokenKind::Punct(Punct::Ellipsis) => return self.refuse("spread element", 6),
            TokenKind::Punct(Punct::LBracket) => return self.refuse("computed property name", 6),
            TokenKind::Punct(Punct::Star) => return self.refuse("generator function", 7),
            TokenKind::Name(n) if n == "get" => FunctionKind::Getter,
            TokenKind::Name(n) if n == "set" => FunctionKind::Setter,
            TokenKind::Name(n) if n == "async" => {
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
        let next = self.peek()?.kind.clone();
        if next == TokenKind::Punct(Punct::LBracket) {
            self.advance()?;
            return self.refuse("computed property name", 6);
        }
        if Parser::is_property_name(&next) {
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
        if let Some(list) = self.properties(object) {
            list.push(Property {
                key,
                value,
                span: start,
            });
        }
        Ok(id)
    }

    /// `PropertyName : AssignmentExpression`, appended to `object`; refuses methods and
    /// shorthand properties.
    fn value_property_rest(&mut self, object: ExprId, start: Span) -> R<()> {
        self.enter()?;
        self.value_property_head(object, start)?;
        let value = self.assignment(false)?;
        if let Some(p) = self.properties(object).and_then(|l| l.last_mut()) {
            p.value = PropertyValue::Init(value);
        }
        self.property_end(object);
        self.leave();
        Ok(())
    }

    /// The `PropertyName :` of a value property, appended to `object` with its value to come.
    #[inline(never)]
    fn value_property_head(&mut self, object: ExprId, start: Span) -> R<()> {
        let key = self.value_property_key(start)?;
        if let Some(list) = self.properties(object) {
            list.push(Property {
                key,
                value: PropertyValue::Init(ExprId(u32::MAX)),
                span: start,
            });
        }
        Ok(())
    }

    /// The `PropertyName :` of a value property.
    #[inline(never)]
    fn value_property_key(&mut self, start: Span) -> R<PropertyKey> {
        let shorthand_candidate = matches!(self.tok.kind, TokenKind::Name(_));
        let key = self.property_key()?;
        if self.eat(Punct::Colon)? {
            return Ok(key);
        }
        if self.is(Punct::LParen) {
            return self.refuse_at("method definition", 6, start);
        }
        if shorthand_candidate
            && (self.is(Punct::Comma) || self.is(Punct::RBrace) || self.is(Punct::Eq))
        {
            return self.refuse_at("shorthand property", 6, start);
        }
        self.unexpected()
    }

    /// Pushes `expr`, which started at `start` and ends at the previous token.
    #[inline(never)]
    pub(super) fn make(&mut self, expr: Expr, start: Span) -> R<ExprId> {
        let span = self.finish(start);
        self.push_expr(expr, span)
    }
}
