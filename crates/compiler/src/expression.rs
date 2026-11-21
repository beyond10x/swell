//! Expressions (clause 13, `#sec-ecmascript-language-expressions`): every ES5 expression the
//! parser accepts, each with the evaluation order of its runtime semantics clause.
//!
//! A reference is evaluated before the value that is put into it (`#sec-assignment-operators-runtime-semantics-evaluation`,
//! `#sec-variable-statement-runtime-semantics-evaluation`, `#sec-postfix-increment-operator`):
//! for a name of the global environment [`Instruction::ResolveGlobal`] makes the Reference Record
//! first, for a property the base value (and the key expression) are evaluated first; a binding
//! of a function or a `catch` clause cannot be unresolvable, so it needs no instruction before the
//! value; a name looked up at run time is resolved first by [`Instruction::ResolveName`]. A
//! binary operator evaluates its left operand, then its right one, then applies itself
//! (`#sec-evaluatestringornumericbinaryexpression`); `&&`, `||`, `??`, `?:` and the logical
//! assignments evaluate an operand only when it decides the result.
//!
//! An optional chain (`#sec-optional-chains`) tests the value before each `?.` with
//! `Dup; Null; Eq` (IsLooselyEqual to `null` is true for `undefined` and `null` alone) and jumps
//! to a short-circuit at the end of the chain, which drops what the chain pushed and leaves
//! `undefined`; so a short-circuit skips every link after it to the end of the chain, and a
//! parenthesized chain is a chain of its own (`#sec-optional-chaining-evaluation`).

use swell_num::literal::numeric_literal;
use swell_syntax::REGEXP_REFUSAL;
use swell_syntax::ast::{
    AssignOp, BinaryOp, Expr, ExprId, LogicalOp, PropertyValue, Span, UnaryOp, UpdateOp,
};

use crate::bytecode::Instruction;
use crate::resolve::{Binding, Site, property_key};
use crate::{CompileError, Compiler, Short, Task, malformed};

/// The reference an assignment, an update, a `delete` or a for-in head works on.
pub(crate) enum Target {
    Binding(Binding),
    /// `object.name`, the name a `strings` index.
    Member {
        object: ExprId,
        name: u32,
    },
    /// `object[key]`.
    Index {
        object: ExprId,
        key: ExprId,
    },
}

impl Target {
    /// How many operand stack entries the evaluated reference takes.
    pub(crate) fn reference_items(&self) -> u32 {
        match self {
            Target::Binding(Binding::Global(_) | Binding::Dynamic(_)) | Target::Member { .. } => 1,
            Target::Binding(_) => 0,
            Target::Index { .. } => 2,
        }
    }
}

/// The jump to `decided`, taken when the value on top (a copy of the left value, which the jump
/// consumes) decides a logical operator `op` on its own.
fn logical_test(op: LogicalOp, decided: u32, span: Span, t: &mut Vec<Task>) {
    match op {
        LogicalOp::And => t.push(Task::Emit(Instruction::JumpIfFalse(decided), span)),
        LogicalOp::Or => t.push(Task::Emit(Instruction::JumpIfTrue(decided), span)),
        LogicalOp::Coalesce => {
            t.push(Task::Emit(Instruction::Null, span));
            t.push(Task::Emit(Instruction::Eq, span));
            t.push(Task::Emit(Instruction::JumpIfFalse(decided), span));
        }
    }
}

fn binary(op: BinaryOp) -> Instruction {
    match op {
        BinaryOp::Mul => Instruction::Mul,
        BinaryOp::Div => Instruction::Div,
        BinaryOp::Rem => Instruction::Rem,
        BinaryOp::Add => Instruction::Add,
        BinaryOp::Sub => Instruction::Sub,
        BinaryOp::Shl => Instruction::Shl,
        BinaryOp::Shr => Instruction::Shr,
        BinaryOp::UShr => Instruction::UShr,
        BinaryOp::Lt => Instruction::Lt,
        BinaryOp::Gt => Instruction::Gt,
        BinaryOp::LtEq => Instruction::LtEq,
        BinaryOp::GtEq => Instruction::GtEq,
        BinaryOp::Instanceof => Instruction::InstanceOf,
        BinaryOp::In => Instruction::In,
        BinaryOp::Eq => Instruction::Eq,
        BinaryOp::NotEq => Instruction::NotEq,
        BinaryOp::StrictEq => Instruction::StrictEq,
        BinaryOp::StrictNotEq => Instruction::StrictNotEq,
        BinaryOp::BitAnd => Instruction::BitAnd,
        BinaryOp::BitXor => Instruction::BitXor,
        BinaryOp::BitOr => Instruction::BitOr,
    }
}

impl<'a> Compiler<'a> {
    /// The reference an expression in the place of an assignment target denotes
    /// (`#sec-static-semantics-assignmenttargettype`: an identifier or a property access).
    pub(crate) fn target(&mut self, e: ExprId) -> Result<Target, CompileError> {
        let node = self
            .ast
            .expr(e)
            .ok_or_else(|| malformed("an expression index"))?;
        match &node.expr {
            Expr::Identifier(_) => Ok(Target::Binding(self.binding(Site::Expr(e))?)),
            Expr::Member { object, property } => Ok(Target::Member {
                object: *object,
                name: self.string(property)?,
            }),
            Expr::Index { object, index } => Ok(Target::Index {
                object: *object,
                key: *index,
            }),
            _ => Err(malformed("an assignment target that is no reference")),
        }
    }

    /// The tasks that evaluate a reference onto the stack; with `convert`, the key of a property
    /// access with an expression is converted at once ([`Instruction::PropertyKey`]), because
    /// the reference is read before it is written.
    pub(crate) fn reference(
        &mut self,
        target: &Target,
        convert: bool,
        span: Span,
    ) -> Result<Vec<Task>, CompileError> {
        Ok(match target {
            Target::Binding(Binding::Global(name)) => {
                vec![Task::Emit(
                    Instruction::ResolveGlobal(self.string(name)?),
                    span,
                )]
            }
            Target::Binding(Binding::Dynamic(name)) => {
                vec![Task::Emit(
                    Instruction::ResolveName(self.string(name)?),
                    span,
                )]
            }
            Target::Binding(_) => Vec::new(),
            Target::Member { object, .. } => vec![Task::Expr(*object)],
            Target::Index { object, key } => {
                let mut t = vec![Task::Expr(*object), Task::Expr(*key)];
                if convert {
                    t.push(Task::Emit(Instruction::PropertyKey, span));
                }
                t
            }
        })
    }

    /// GetValue of an evaluated reference, which stays on the stack under the value.
    fn get_keeping(&mut self, target: &Target, span: Span) -> Result<Vec<Task>, CompileError> {
        Ok(match target {
            Target::Binding(b) => vec![Task::Emit(self.load(b)?, span)],
            Target::Member { name, .. } => vec![
                Task::Emit(Instruction::Dup, span),
                Task::Emit(Instruction::GetNamed(*name), span),
            ],
            Target::Index { .. } => vec![
                Task::Emit(Instruction::Dup2, span),
                Task::Emit(Instruction::GetIndex, span),
            ],
        })
    }

    /// The instruction that reads a binding: for a global name, the read of the reference that
    /// [`Instruction::ResolveGlobal`] made.
    fn load(&mut self, b: &Binding) -> Result<Instruction, CompileError> {
        Ok(match b {
            Binding::Local(n) => Instruction::GetLocal(*n),
            Binding::Env { hops, slot } | Binding::Immutable { hops, slot, .. } => {
                Instruction::GetEnv {
                    hops: *hops,
                    slot: *slot,
                }
            }
            Binding::Global(name) => Instruction::GetGlobalRef(self.string(name)?),
            Binding::Dynamic(name) => Instruction::GetNameRef(self.string(name)?),
        })
    }

    /// PutValue of the value on top into the evaluated reference under it; the value stays.
    pub(crate) fn put(&mut self, target: &Target) -> Result<Instruction, CompileError> {
        Ok(match target {
            Target::Binding(Binding::Local(n)) => Instruction::SetLocal(*n),
            Target::Binding(Binding::Env { hops, slot }) => Instruction::SetEnv {
                hops: *hops,
                slot: *slot,
            },
            Target::Binding(Binding::Immutable { name, .. }) => {
                Instruction::AssignImmutable(self.string(name)?)
            }
            Target::Binding(Binding::Global(name)) => Instruction::PutGlobalRef(self.string(name)?),
            Target::Binding(Binding::Dynamic(name)) => Instruction::PutNameRef(self.string(name)?),
            Target::Member { name, .. } => Instruction::PutNamed(*name),
            Target::Index { .. } => Instruction::PutIndex,
        })
    }

    /// The tasks that push the value of the expression `id`.
    pub(crate) fn expression(&mut self, id: ExprId) -> Result<Vec<Task>, CompileError> {
        let node = self
            .ast
            .expr(id)
            .ok_or_else(|| malformed("an expression index"))?;
        let span = node.span;
        let emit = |i: Instruction| Task::Emit(i, span);
        let mut t = Vec::new();
        match &node.expr {
            // `#sec-this-keyword-runtime-semantics-evaluation`.
            Expr::This => t.push(emit(Instruction::This)),
            // `#sec-identifiers-runtime-semantics-evaluation`: ResolveBinding, then GetValue.
            Expr::Identifier(_) => {
                let instruction = match self.binding(Site::Expr(id))? {
                    Binding::Global(name) => Instruction::GetGlobal(self.string(&name)?),
                    Binding::Dynamic(name) => Instruction::GetName(self.string(&name)?),
                    b => self.load(&b)?,
                };
                t.push(emit(instruction));
            }
            // `#sec-literals-runtime-semantics-evaluation`.
            Expr::Null => t.push(emit(Instruction::Null)),
            Expr::Boolean(true) => t.push(emit(Instruction::True)),
            Expr::Boolean(false) => t.push(emit(Instruction::False)),
            Expr::Number(n) => {
                // `#sec-numericvalue`, computed by `swell-num`.
                let value = numeric_literal(n.text.as_bytes())
                    .ok_or_else(|| malformed("a numeric literal swell-num does not read"))?
                    .value;
                t.push(emit(Instruction::Number(self.constants.number(value)?)));
            }
            Expr::String(s) => t.push(emit(Instruction::String(self.constants.string(&s.value)?))),
            Expr::RegExp(_) => {
                t.push(emit(Instruction::ThrowSyntaxError(
                    self.string(REGEXP_REFUSAL)?,
                )));
            }
            // `#sec-array-initializer-runtime-semantics-evaluation` and ArrayAccumulation
            // (`#sec-runtime-semantics-arrayaccumulation`): a run of elisions sets the length to
            // the next index.
            Expr::Array(elements) => {
                t.push(emit(Instruction::NewArray));
                let mut next: u32 = 0;
                let mut elided = false;
                for element in elements {
                    match element {
                        None => {
                            next = next
                                .checked_add(1)
                                .ok_or(CompileError::TooMany("array elements"))?;
                            elided = true;
                        }
                        Some(e) => {
                            if elided {
                                t.push(emit(Instruction::ArrayLength(next)));
                                elided = false;
                            }
                            t.push(Task::Expr(*e));
                            t.push(emit(Instruction::DefineIndex(next)));
                            next = next
                                .checked_add(1)
                                .ok_or(CompileError::TooMany("array elements"))?;
                        }
                    }
                }
                if elided {
                    t.push(emit(Instruction::ArrayLength(next)));
                }
            }
            // `#sec-object-initializer-runtime-semantics-evaluation` and
            // PropertyDefinitionEvaluation
            // (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`,
            // `#sec-method-definitions-runtime-semantics-propertydefinitionevaluation`).
            Expr::Object(properties) => {
                t.push(emit(Instruction::NewObject));
                for p in properties {
                    let key = property_key(&p.key)?;
                    let proto = !matches!(p.key, swell_syntax::ast::PropertyKey::Number(_))
                        && key.iter().copied().eq("__proto__".encode_utf16());
                    let k = self.constants.string(&key)?;
                    match p.value {
                        PropertyValue::Init(e) => {
                            t.push(Task::Expr(e));
                            if proto {
                                t.push(Task::Emit(Instruction::InitProto, p.span));
                            } else {
                                t.push(Task::Emit(Instruction::DefineField(k), p.span));
                            }
                        }
                        PropertyValue::Get(f) | PropertyValue::Set(f) => {
                            let fspan = self.ast.function(f).map(|x| x.span).unwrap_or(p.span);
                            t.push(Task::Emit(Instruction::Closure(f.0), fspan));
                            let define = if matches!(p.value, PropertyValue::Get(_)) {
                                Instruction::DefineGetter(k)
                            } else {
                                Instruction::DefineSetter(k)
                            };
                            t.push(Task::Emit(define, p.span));
                        }
                        // `#sec-method-definitions-runtime-semantics-propertydefinitionevaluation`:
                        // DefineMethod, then DefineMethodProperty, enumerable.
                        PropertyValue::Method(f) => {
                            let fspan = self.ast.function(f).map(|x| x.span).unwrap_or(p.span);
                            t.push(Task::Emit(Instruction::Closure(f.0), fspan));
                            t.push(Task::Emit(Instruction::DefineMethod(k), p.span));
                        }
                        // `PropertyDefinition : IdentifierReference`: the value of the reference,
                        // defined as a data property even when the name is `__proto__`.
                        PropertyValue::Shorthand(e) => {
                            t.push(Task::Expr(e));
                            t.push(Task::Emit(Instruction::DefineField(k), p.span));
                        }
                    }
                }
            }
            // `#sec-function-definitions-runtime-semantics-evaluation`.
            Expr::Function(f) => t.push(emit(Instruction::Closure(f.0))),
            Expr::Unary { op, operand } => self.unary(*op, *operand, span, &mut t)?,
            Expr::Update {
                op,
                prefix,
                operand,
            } => {
                // `#sec-prefix-increment-operator`, `#sec-postfix-increment-operator` and the
                // decrement ones: the old value is ToNumeric of GetValue.
                let target = self.target(*operand)?;
                let r = target.reference_items();
                t.extend(self.reference(&target, true, span)?);
                t.extend(self.get_keeping(&target, span)?);
                t.push(emit(Instruction::ToNumeric));
                if !*prefix {
                    t.push(emit(Instruction::Dup));
                    if r > 0 {
                        t.push(emit(Instruction::Bury(r + 1)));
                    }
                }
                t.push(emit(match op {
                    UpdateOp::Increment => Instruction::Increment,
                    UpdateOp::Decrement => Instruction::Decrement,
                }));
                t.push(emit(self.put(&target)?));
                if !*prefix {
                    t.push(emit(Instruction::Pop));
                }
            }
            // `#sec-evaluatestringornumericbinaryexpression` and the relational and equality
            // clauses: left, right, the operator.
            Expr::Binary { op, left, right } => {
                t.push(Task::Expr(*left));
                t.push(Task::Expr(*right));
                t.push(emit(binary(*op)));
            }
            // `#sec-binary-logical-operators-runtime-semantics-evaluation`: the left value
            // decides when it is falsy for `&&`, truthy for `||`, and neither `undefined` nor
            // `null` for `??`.
            Expr::Logical { op, left, right } => {
                let end = self.label();
                t.push(Task::Expr(*left));
                t.push(emit(Instruction::Dup));
                logical_test(*op, end, span, &mut t);
                t.push(emit(Instruction::Pop));
                t.push(Task::Expr(*right));
                t.push(Task::Bind(end));
            }
            // `#sec-conditional-operator-runtime-semantics-evaluation`.
            Expr::Conditional {
                test,
                consequent,
                alternate,
            } => {
                let (otherwise, end) = (self.label(), self.label());
                t.push(Task::Expr(*test));
                t.push(emit(Instruction::JumpIfFalse(otherwise)));
                t.push(Task::Expr(*consequent));
                t.push(Task::Goto(end, span));
                t.push(Task::Bind(otherwise));
                t.push(Task::Expr(*alternate));
                t.push(Task::Bind(end));
            }
            // `#sec-assignment-operators-runtime-semantics-evaluation`. NamedEvaluation of an
            // anonymous function is the name of its template.
            Expr::Assign { op, target, value } => {
                let target = self.target(*target)?;
                match op {
                    AssignOp::Assign => {
                        t.extend(self.reference(&target, false, span)?);
                        t.push(Task::Expr(*value));
                    }
                    AssignOp::Compound(op) => {
                        t.extend(self.reference(&target, true, span)?);
                        t.extend(self.get_keeping(&target, span)?);
                        t.push(Task::Expr(*value));
                        t.push(emit(binary(*op)));
                    }
                    // `&&=`, `||=` and `??=`: when the left value decides, it is the result and
                    // nothing is put; the reference under it is dropped.
                    AssignOp::Logical(op) => {
                        let r = target.reference_items();
                        let keep = self.label();
                        t.extend(self.reference(&target, true, span)?);
                        t.extend(self.get_keeping(&target, span)?);
                        t.push(emit(Instruction::Dup));
                        logical_test(*op, keep, span, &mut t);
                        t.push(emit(Instruction::Pop));
                        t.push(Task::Expr(*value));
                        t.push(emit(self.put(&target)?));
                        if r > 0 {
                            let end = self.label();
                            t.push(Task::Goto(end, span));
                            t.push(Task::Bind(keep));
                            t.push(emit(Instruction::Nip(r)));
                            t.push(Task::Bind(end));
                        } else {
                            t.push(Task::Bind(keep));
                        }
                        return Ok(t);
                    }
                }
                t.push(emit(self.put(&target)?));
            }
            // `#sec-comma-operator-runtime-semantics-evaluation`.
            Expr::Sequence(list) => {
                for (i, e) in list.iter().enumerate() {
                    if i > 0 {
                        t.push(emit(Instruction::Pop));
                    }
                    t.push(Task::Expr(*e));
                }
            }
            // `#sec-property-accessors-runtime-semantics-evaluation`.
            Expr::Member { object, property } => {
                t.push(Task::Expr(*object));
                t.push(emit(Instruction::GetNamed(self.string(property)?)));
            }
            Expr::Index { object, index } => {
                t.push(Task::Expr(*object));
                t.push(Task::Expr(*index));
                t.push(emit(Instruction::GetIndex));
            }
            // `#sec-optional-chaining-evaluation`: the chain's links, then its short-circuits.
            Expr::OptionalChain(inner) => {
                t.push(Task::ChainOpen(Short::Undefined));
                t.push(Task::Expr(*inner));
                t.push(Task::ChainClose(span));
            }
            // `#sec-optional-chaining-chain-evaluation`: `?.` tests the base value first.
            Expr::OptionalMember { object, property } => {
                t.push(Task::Expr(*object));
                t.push(Task::ChainCheck(1, span));
                t.push(emit(Instruction::GetNamed(self.string(property)?)));
            }
            Expr::OptionalIndex { object, index } => {
                t.push(Task::Expr(*object));
                t.push(Task::ChainCheck(1, span));
                t.push(Task::Expr(*index));
                t.push(emit(Instruction::GetIndex));
            }
            // `#sec-function-calls-runtime-semantics-evaluation`: the function and its this value
            // (the base of a property reference, the binding object of a `with` statement for a
            // name looked up by name, otherwise `undefined`), the arguments in order, then
            // EvaluateCall. `?.(...)` tests the function value, with its this value under it.
            Expr::Call { callee, arguments } | Expr::OptionalCall { callee, arguments } => {
                self.callee(*callee, span, &mut t)?;
                if matches!(node.expr, Expr::OptionalCall { .. }) {
                    t.push(Task::ChainCheck(2, span));
                }
                t.extend(arguments.iter().map(|e| Task::Expr(*e)));
                let n = u32::try_from(arguments.len())
                    .map_err(|_| CompileError::TooMany("arguments"))?;
                t.push(emit(Instruction::Call(n)));
            }
            // `#sec-new-operator-runtime-semantics-evaluation`.
            Expr::New { callee, arguments } => {
                t.push(Task::Expr(*callee));
                let arguments = arguments.as_deref().unwrap_or(&[]);
                t.extend(arguments.iter().map(|e| Task::Expr(*e)));
                let n = u32::try_from(arguments.len())
                    .map_err(|_| CompileError::TooMany("arguments"))?;
                t.push(emit(Instruction::New(n)));
            }
        }
        Ok(t)
    }

    /// The this value and the function of a call with callee `callee` (`#sec-function-calls`),
    /// `[this, f]`: a property access gives its base as this value, also after a `?.` and in a
    /// parenthesized optional chain, whose short-circuit then leaves `undefined` for both
    /// (`#sec-optional-chaining-evaluation` gives the Reference of the chain's last link).
    fn callee(
        &mut self,
        callee: ExprId,
        span: Span,
        t: &mut Vec<Task>,
    ) -> Result<(), CompileError> {
        let node = self.ast.expr(callee).ok_or_else(|| malformed("a callee"))?;
        let (link, chain) = match &node.expr {
            Expr::OptionalChain(inner) => {
                let inner_node = self
                    .ast
                    .expr(*inner)
                    .ok_or_else(|| malformed("an optional chain"))?;
                if !matches!(
                    inner_node.expr,
                    Expr::Member { .. }
                        | Expr::Index { .. }
                        | Expr::OptionalMember { .. }
                        | Expr::OptionalIndex { .. }
                ) {
                    t.push(Task::Emit(Instruction::Undefined, span));
                    t.push(Task::Expr(callee));
                    return Ok(());
                }
                t.push(Task::ChainOpen(Short::Pair));
                (inner_node, Some(node.span))
            }
            _ => (node, None),
        };
        match &link.expr {
            Expr::Member { object, property } | Expr::OptionalMember { object, property } => {
                t.push(Task::Expr(*object));
                if matches!(link.expr, Expr::OptionalMember { .. }) {
                    t.push(Task::ChainCheck(1, link.span));
                }
                t.push(Task::Emit(Instruction::Dup, span));
                t.push(Task::Emit(
                    Instruction::GetNamed(self.string(property)?),
                    span,
                ));
            }
            Expr::Index { object, index } | Expr::OptionalIndex { object, index } => {
                t.push(Task::Expr(*object));
                if matches!(link.expr, Expr::OptionalIndex { .. }) {
                    t.push(Task::ChainCheck(1, link.span));
                }
                t.push(Task::Emit(Instruction::Dup, span));
                t.push(Task::Expr(*index));
                t.push(Task::Emit(Instruction::GetIndex, span));
            }
            // A name looked up by name gives its record's WithBaseObject as this value.
            Expr::Identifier(_) if chain.is_none() => match self.binding(Site::Expr(callee))? {
                Binding::Dynamic(name) => {
                    t.push(Task::Emit(
                        Instruction::GetNameForCall(self.string(&name)?),
                        span,
                    ));
                }
                _ => {
                    t.push(Task::Emit(Instruction::Undefined, span));
                    t.push(Task::Expr(callee));
                }
            },
            _ => {
                t.push(Task::Emit(Instruction::Undefined, span));
                t.push(Task::Expr(callee));
            }
        }
        if let Some(chain_span) = chain {
            t.push(Task::ChainClose(chain_span));
        }
        Ok(())
    }

    /// `delete` of an optional chain whose last link is a property access: the property is
    /// deleted unless the chain short-circuits, which gives `true`
    /// (`#sec-delete-operator-runtime-semantics-evaluation`). `None` for any other chain.
    fn delete_chain(
        &mut self,
        inner: ExprId,
        chain_span: Span,
        span: Span,
    ) -> Result<Option<Vec<Task>>, CompileError> {
        let link = self
            .ast
            .expr(inner)
            .ok_or_else(|| malformed("an optional chain"))?;
        let mut t = vec![Task::ChainOpen(Short::True)];
        match &link.expr {
            Expr::Member { object, property } | Expr::OptionalMember { object, property } => {
                t.push(Task::Expr(*object));
                if matches!(link.expr, Expr::OptionalMember { .. }) {
                    t.push(Task::ChainCheck(1, link.span));
                }
                t.push(Task::Emit(
                    Instruction::DeleteNamed(self.string(property)?),
                    span,
                ));
            }
            Expr::Index { object, index } | Expr::OptionalIndex { object, index } => {
                t.push(Task::Expr(*object));
                if matches!(link.expr, Expr::OptionalIndex { .. }) {
                    t.push(Task::ChainCheck(1, link.span));
                }
                t.push(Task::Expr(*index));
                t.push(Task::Emit(Instruction::DeleteIndex, span));
            }
            _ => return Ok(None),
        }
        t.push(Task::ChainClose(chain_span));
        Ok(Some(t))
    }

    /// The unary operators (`#sec-unary-operators`).
    fn unary(
        &mut self,
        op: UnaryOp,
        operand: ExprId,
        span: Span,
        t: &mut Vec<Task>,
    ) -> Result<(), CompileError> {
        let emit = |i: Instruction| Task::Emit(i, span);
        let operand_node = self
            .ast
            .expr(operand)
            .ok_or_else(|| malformed("an operand"))?;
        match op {
            // `#sec-delete-operator-runtime-semantics-evaluation`: a property reference is
            // deleted from its base, a binding of a function or catch clause cannot be deleted
            // (`false`), a global name is deleted through the global environment, and any
            // other expression is evaluated and gives `true`.
            UnaryOp::Delete => match &operand_node.expr {
                Expr::Identifier(_) => match self.binding(Site::Expr(operand))? {
                    Binding::Global(name) => {
                        t.push(emit(Instruction::DeleteGlobal(self.string(&name)?)));
                    }
                    Binding::Dynamic(name) => {
                        t.push(emit(Instruction::DeleteName(self.string(&name)?)));
                    }
                    _ => t.push(emit(Instruction::False)),
                },
                Expr::Member { object, property } => {
                    t.push(Task::Expr(*object));
                    t.push(emit(Instruction::DeleteNamed(self.string(property)?)));
                }
                Expr::Index { object, index } => {
                    t.push(Task::Expr(*object));
                    t.push(Task::Expr(*index));
                    t.push(emit(Instruction::DeleteIndex));
                }
                Expr::OptionalChain(inner) => {
                    match self.delete_chain(*inner, operand_node.span, span)? {
                        Some(tasks) => t.extend(tasks),
                        None => {
                            t.push(Task::Expr(operand));
                            t.push(emit(Instruction::Pop));
                            t.push(emit(Instruction::True));
                        }
                    }
                }
                _ => {
                    t.push(Task::Expr(operand));
                    t.push(emit(Instruction::Pop));
                    t.push(emit(Instruction::True));
                }
            },
            // `#sec-void-operator-runtime-semantics-evaluation`.
            UnaryOp::Void => {
                t.push(Task::Expr(operand));
                t.push(emit(Instruction::Pop));
                t.push(emit(Instruction::Undefined));
            }
            // `#sec-typeof-operator-runtime-semantics-evaluation`: an unresolvable reference
            // gives "undefined".
            UnaryOp::Typeof => match self.name_binding(operand)? {
                Some(Binding::Global(name)) => {
                    t.push(emit(Instruction::TypeofGlobal(self.string(&name)?)));
                }
                Some(Binding::Dynamic(name)) => {
                    t.push(emit(Instruction::TypeofName(self.string(&name)?)));
                }
                _ => {
                    t.push(Task::Expr(operand));
                    t.push(emit(Instruction::Typeof));
                }
            },
            UnaryOp::Plus => {
                t.push(Task::Expr(operand));
                t.push(emit(Instruction::ToNumber));
            }
            UnaryOp::Minus => {
                t.push(Task::Expr(operand));
                t.push(emit(Instruction::Negate));
            }
            UnaryOp::BitNot => {
                t.push(Task::Expr(operand));
                t.push(emit(Instruction::BitNot));
            }
            UnaryOp::Not => {
                t.push(Task::Expr(operand));
                t.push(emit(Instruction::Not));
            }
        }
        Ok(())
    }

    /// The binding of the expression, when it is an identifier.
    fn name_binding(&self, e: ExprId) -> Result<Option<Binding>, CompileError> {
        match self.ast.expr(e).map(|n| &n.expr) {
            Some(Expr::Identifier(_)) => Ok(Some(self.binding(Site::Expr(e))?)),
            _ => Ok(None),
        }
    }
}
