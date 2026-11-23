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
//!
//! An object pattern node destructures the value on top of the stack, which stays: the value
//! of an assignment to it, of a declaration, of a parameter, of a for-in key or of an exception
//! ([`Instruction::RequireObjectCoercible`], then [`Instruction::PatternGet`] per property); an
//! array pattern node the same, through the value's iterator, whose record it holds above the
//! value under a handler of its own ([`HandlerKind::Pattern`]).

use swell_num::literal::numeric_literal;
use swell_syntax::ast::{
    AssignOp, BinaryOp, Expr, ExprId, FunctionKind, LogicalOp, PatternElement, PatternKind,
    PatternProperty, PatternTarget, Property, PropertyKey, PropertyValue, Span, UnaryOp, UpdateOp,
};

use crate::bytecode::{HandlerKind, Instruction, TAG_NORMAL, TAG_RETURN};
use crate::resolve::{Binding, LexicalPlace, Site, anonymous_function, property_key};
use crate::{CompileError, Compiler, Short, Task, Tasks, malformed};

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
fn logical_test(
    op: LogicalOp,
    decided: u32,
    span: Span,
    t: &mut Tasks<'_, '_>,
) -> Result<(), CompileError> {
    match op {
        LogicalOp::And => t.push(Task::Emit(Instruction::JumpIfFalse(decided), span)),
        LogicalOp::Or => t.push(Task::Emit(Instruction::JumpIfTrue(decided), span)),
        LogicalOp::Coalesce => {
            t.push(Task::Emit(Instruction::Null, span))?;
            t.push(Task::Emit(Instruction::Eq, span))?;
            t.push(Task::Emit(Instruction::JumpIfFalse(decided), span))
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

impl<'a, 'm> Compiler<'a, 'm> {
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
    ) -> Result<Tasks<'a, 'm>, CompileError> {
        let mut t = Tasks::new(self.m);
        match target {
            Target::Binding(Binding::Global(name)) => {
                let name = self.string(name)?;
                t.push(Task::Emit(Instruction::ResolveGlobal(name), span))?;
            }
            Target::Binding(Binding::Dynamic(name)) => {
                let name = self.string(name)?;
                t.push(Task::Emit(Instruction::ResolveName(name), span))?;
            }
            Target::Binding(_) => {}
            Target::Member { object, .. } => t.push(Task::Expr(*object))?,
            Target::Index { object, key } => {
                t.push(Task::Expr(*object))?;
                t.push(Task::Expr(*key))?;
                if convert {
                    t.push(Task::Emit(Instruction::PropertyKey, span))?;
                }
            }
        }
        Ok(t)
    }

    /// GetValue of an evaluated reference, which stays on the stack under the value.
    fn get_keeping(&mut self, target: &Target, span: Span) -> Result<Tasks<'a, 'm>, CompileError> {
        let mut t = Tasks::new(self.m);
        match target {
            Target::Binding(b) => {
                let load = self.load(b)?;
                t.push(Task::Emit(load, span))?;
            }
            Target::Member { name, .. } => {
                t.push(Task::Emit(Instruction::Dup, span))?;
                t.push(Task::Emit(Instruction::GetNamed(*name), span))?;
            }
            Target::Index { .. } => {
                t.push(Task::Emit(Instruction::Dup2, span))?;
                t.push(Task::Emit(Instruction::GetIndex, span))?;
            }
        }
        Ok(t)
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
            Binding::Lexical {
                at, name, checked, ..
            } => match (*at, *checked) {
                (LexicalPlace::Local(slot), true) => Instruction::GetLocalChecked {
                    slot,
                    name: self.string(name)?,
                },
                (LexicalPlace::Env { hops, slot }, true) => {
                    Instruction::GetEnvChecked { hops, slot }
                }
                (LexicalPlace::Local(slot), false) => Instruction::GetLocal(slot),
                (LexicalPlace::Env { hops, slot }, false) => Instruction::GetEnv { hops, slot },
            },
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
            // A `const` binding throws for every assignment, a `let` one before its declaration.
            Target::Binding(Binding::Lexical {
                at, name, constant, ..
            }) => match (*at, *constant) {
                (LexicalPlace::Local(slot), true) => Instruction::AssignConstLocal {
                    slot,
                    name: self.string(name)?,
                },
                (LexicalPlace::Env { hops, slot }, true) => {
                    Instruction::AssignConstEnv { hops, slot }
                }
                (LexicalPlace::Local(slot), false) => Instruction::SetLocalChecked {
                    slot,
                    name: self.string(name)?,
                },
                (LexicalPlace::Env { hops, slot }, false) => {
                    Instruction::SetEnvChecked { hops, slot }
                }
            },
        })
    }

    /// The tasks that push the value of the expression `id`.
    pub(crate) fn expression(&mut self, id: ExprId) -> Result<Tasks<'a, 'm>, CompileError> {
        let node = self
            .ast
            .expr(id)
            .ok_or_else(|| malformed("an expression index"))?;
        let span = node.span;
        let emit = |i: Instruction| Task::Emit(i, span);
        let mut t = Tasks::new(self.m);
        match &node.expr {
            // `#sec-this-keyword-runtime-semantics-evaluation`.
            Expr::This => t.push(emit(Instruction::This))?,
            // `#sec-identifiers-runtime-semantics-evaluation`: ResolveBinding, then GetValue.
            Expr::Identifier(_) => {
                let instruction = match self.binding(Site::Expr(id))? {
                    Binding::Global(name) => Instruction::GetGlobal(self.string(&name)?),
                    Binding::Dynamic(name) => Instruction::GetName(self.string(&name)?),
                    b => self.load(&b)?,
                };
                t.push(emit(instruction))?;
            }
            // `#sec-literals-runtime-semantics-evaluation`.
            Expr::Null => t.push(emit(Instruction::Null))?,
            Expr::Boolean(true) => t.push(emit(Instruction::True))?,
            Expr::Boolean(false) => t.push(emit(Instruction::False))?,
            Expr::Number(n) => {
                // `#sec-numericvalue`, computed by `swell-num`.
                let value = numeric_literal(n.text.as_bytes())
                    .ok_or_else(|| malformed("a numeric literal swell-num does not read"))?
                    .value;
                t.push(emit(Instruction::Number(self.number(value)?)))?;
            }
            Expr::String(s) => t.push(emit(Instruction::String(self.string_units(&s.value)?)))?,
            // `#sec-regular-expression-literals-runtime-semantics-evaluation`: RegExpCreate of
            // the BodyText and the FlagText, as written, at each evaluation.
            Expr::RegExp(literal) => {
                let body = self.string_units(&literal.body)?;
                let flags = self.string_units(&literal.flags)?;
                t.push(emit(Instruction::NewRegExp { body, flags }))?;
            }
            // `#sec-array-initializer-runtime-semantics-evaluation` and ArrayAccumulation
            // (`#sec-runtime-semantics-arrayaccumulation`): a run of elisions sets the length to
            // the next index.
            Expr::Array(elements) if self.has_spread(elements.iter().flatten()) => {
                t.push(emit(Instruction::NewArray))?;
                self.spread_elements(elements, span, &mut t)?;
            }
            Expr::Array(elements) => {
                t.push(emit(Instruction::NewArray))?;
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
                                t.push(emit(Instruction::ArrayLength(next)))?;
                                elided = false;
                            }
                            t.push(Task::Expr(*e))?;
                            t.push(emit(Instruction::DefineIndex(next)))?;
                            next = next
                                .checked_add(1)
                                .ok_or(CompileError::TooMany("array elements"))?;
                        }
                    }
                }
                if elided {
                    t.push(emit(Instruction::ArrayLength(next)))?;
                }
            }
            // `#sec-object-initializer-runtime-semantics-evaluation` and
            // PropertyDefinitionEvaluation
            // (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`,
            // `#sec-method-definitions-runtime-semantics-propertydefinitionevaluation`).
            Expr::Object(properties) => {
                t.push(emit(Instruction::NewObject))?;
                for p in properties {
                    if self.dynamic_property(p, &mut t)? {
                        continue;
                    }
                    let key = property_key(&p.key, self.m)?;
                    let proto = !matches!(p.key, swell_syntax::ast::PropertyKey::Number(_))
                        && key.iter().copied().eq("__proto__".encode_utf16());
                    let k = self.string_units(&key);
                    self.m.release(key);
                    let k = k?;
                    match p.value {
                        PropertyValue::Init(e) => {
                            t.push(Task::Expr(e))?;
                            if proto {
                                t.push(Task::Emit(Instruction::InitProto, p.span))?;
                            } else {
                                t.push(Task::Emit(Instruction::DefineField(k), p.span))?;
                            }
                        }
                        PropertyValue::Get(f) | PropertyValue::Set(f) => {
                            let fspan = self.ast.function(f).map(|x| x.span).unwrap_or(p.span);
                            t.push(Task::Emit(Instruction::Closure(f.0), fspan))?;
                            let define = if matches!(p.value, PropertyValue::Get(_)) {
                                Instruction::DefineGetter(k)
                            } else {
                                Instruction::DefineSetter(k)
                            };
                            t.push(Task::Emit(define, p.span))?;
                        }
                        // `#sec-method-definitions-runtime-semantics-propertydefinitionevaluation`:
                        // DefineMethod, then DefineMethodProperty, enumerable.
                        PropertyValue::Method(f) => {
                            let fspan = self.ast.function(f).map(|x| x.span).unwrap_or(p.span);
                            t.push(Task::Emit(Instruction::Closure(f.0), fspan))?;
                            t.push(Task::Emit(Instruction::DefineMethod(k), p.span))?;
                        }
                        // `PropertyDefinition : IdentifierReference`: the value of the reference,
                        // defined as a data property even when the name is `__proto__`.
                        PropertyValue::Shorthand(e) => {
                            t.push(Task::Expr(e))?;
                            t.push(Task::Emit(Instruction::DefineField(k), p.span))?;
                        }
                        // The parser rereads such a literal as a pattern, or refuses it.
                        PropertyValue::CoverInitialized(..) => {
                            return Err(malformed("a CoverInitializedName in an object literal"));
                        }
                        PropertyValue::Spread(_) => {
                            return Err(malformed("a spread read as a property"));
                        }
                    }
                }
            }
            // An object pattern destructures the value on top, which stays.
            Expr::ObjectPattern {
                properties,
                kind,
                rest,
            } => {
                self.object_pattern(id, properties, *kind, *rest, span, &mut t)?;
            }
            // An array pattern destructures the value on top, which stays.
            Expr::ArrayPattern {
                elements,
                kind,
                rest,
            } => {
                self.array_pattern(id, elements, *kind, *rest, span, &mut t)?;
            }
            // The parser leaves a spread only in an array literal and among arguments, which
            // read it themselves.
            Expr::Spread(_) => return Err(malformed("a spread outside a list")),
            // `#sec-function-definitions-runtime-semantics-evaluation`; an arrow function,
            // `#sec-arrow-function-definitions-runtime-semantics-evaluation`, keeps the `this` of
            // the code that makes it.
            Expr::Function(f) => {
                let arrow = self
                    .ast
                    .function(*f)
                    .is_some_and(|x| x.kind == FunctionKind::Arrow);
                let make = if arrow {
                    Instruction::ArrowClosure(f.0)
                } else {
                    Instruction::Closure(f.0)
                };
                t.push(emit(make))?;
            }
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
                t.append(self.reference(&target, true, span)?)?;
                t.append(self.get_keeping(&target, span)?)?;
                t.push(emit(Instruction::ToNumeric))?;
                if !*prefix {
                    t.push(emit(Instruction::Dup))?;
                    if r > 0 {
                        t.push(emit(Instruction::Bury(r + 1)))?;
                    }
                }
                t.push(emit(match op {
                    UpdateOp::Increment => Instruction::Increment,
                    UpdateOp::Decrement => Instruction::Decrement,
                }))?;
                t.push(emit(self.put(&target)?))?;
                if !*prefix {
                    t.push(emit(Instruction::Pop))?;
                }
            }
            // `#sec-evaluatestringornumericbinaryexpression` and the relational and equality
            // clauses: left, right, the operator.
            Expr::Binary { op, left, right } => {
                t.push(Task::Expr(*left))?;
                t.push(Task::Expr(*right))?;
                t.push(emit(binary(*op)))?;
            }
            // `#sec-binary-logical-operators-runtime-semantics-evaluation`: the left value
            // decides when it is falsy for `&&`, truthy for `||`, and neither `undefined` nor
            // `null` for `??`.
            Expr::Logical { op, left, right } => {
                let end = self.label();
                t.push(Task::Expr(*left))?;
                t.push(emit(Instruction::Dup))?;
                logical_test(*op, end, span, &mut t)?;
                t.push(emit(Instruction::Pop))?;
                t.push(Task::Expr(*right))?;
                t.push(Task::Bind(end))?;
            }
            // `#sec-conditional-operator-runtime-semantics-evaluation`.
            Expr::Conditional {
                test,
                consequent,
                alternate,
            } => {
                let (otherwise, end) = (self.label(), self.label());
                t.push(Task::Expr(*test))?;
                t.push(emit(Instruction::JumpIfFalse(otherwise)))?;
                t.push(Task::Expr(*consequent))?;
                t.push(Task::Goto(end, span))?;
                t.push(Task::Bind(otherwise))?;
                t.push(Task::Expr(*alternate))?;
                t.push(Task::Bind(end))?;
            }
            // `#sec-assignment-operators-runtime-semantics-evaluation`. NamedEvaluation of an
            // anonymous function is the name of its template.
            Expr::Assign { op, target, value } => {
                // `AssignmentExpression : LeftHandSideExpression = AssignmentExpression` with an
                // ObjectAssignmentPattern or an ArrayAssignmentPattern: the value, destructured,
                // is the result.
                if let Some(Expr::ObjectPattern { .. } | Expr::ArrayPattern { .. }) =
                    self.ast.expr(*target).map(|n| &n.expr)
                {
                    if *op != AssignOp::Assign {
                        return Err(malformed("a compound assignment to a pattern"));
                    }
                    t.push(Task::Expr(*value))?;
                    t.push(Task::Expr(*target))?;
                    return Ok(t);
                }
                let target = self.target(*target)?;
                match op {
                    AssignOp::Assign => {
                        t.append(self.reference(&target, false, span)?)?;
                        t.push(Task::Expr(*value))?;
                    }
                    AssignOp::Compound(op) => {
                        t.append(self.reference(&target, true, span)?)?;
                        t.append(self.get_keeping(&target, span)?)?;
                        t.push(Task::Expr(*value))?;
                        t.push(emit(binary(*op)))?;
                    }
                    // `&&=`, `||=` and `??=`: when the left value decides, it is the result and
                    // nothing is put; the reference under it is dropped.
                    AssignOp::Logical(op) => {
                        let r = target.reference_items();
                        let keep = self.label();
                        t.append(self.reference(&target, true, span)?)?;
                        t.append(self.get_keeping(&target, span)?)?;
                        t.push(emit(Instruction::Dup))?;
                        logical_test(*op, keep, span, &mut t)?;
                        t.push(emit(Instruction::Pop))?;
                        t.push(Task::Expr(*value))?;
                        t.push(emit(self.put(&target)?))?;
                        if r > 0 {
                            let end = self.label();
                            t.push(Task::Goto(end, span))?;
                            t.push(Task::Bind(keep))?;
                            t.push(emit(Instruction::Nip(r)))?;
                            t.push(Task::Bind(end))?;
                        } else {
                            t.push(Task::Bind(keep))?;
                        }
                        return Ok(t);
                    }
                }
                t.push(emit(self.put(&target)?))?;
            }
            // `#sec-comma-operator-runtime-semantics-evaluation`.
            Expr::Sequence(list) => {
                for (i, e) in list.iter().enumerate() {
                    if i > 0 {
                        t.push(emit(Instruction::Pop))?;
                    }
                    t.push(Task::Expr(*e))?;
                }
            }
            // `#sec-property-accessors-runtime-semantics-evaluation`.
            Expr::Member { object, property } => {
                t.push(Task::Expr(*object))?;
                t.push(emit(Instruction::GetNamed(self.string(property)?)))?;
            }
            Expr::Index { object, index } => {
                t.push(Task::Expr(*object))?;
                t.push(Task::Expr(*index))?;
                t.push(emit(Instruction::GetIndex))?;
            }
            // `#sec-optional-chaining-evaluation`: the chain's links, then its short-circuits.
            Expr::OptionalChain(inner) => {
                t.push(Task::ChainOpen(Short::Undefined))?;
                t.push(Task::Expr(*inner))?;
                t.push(Task::ChainClose(span))?;
            }
            // `#sec-optional-chaining-chain-evaluation`: `?.` tests the base value first.
            Expr::OptionalMember { object, property } => {
                t.push(Task::Expr(*object))?;
                t.push(Task::ChainCheck(1, span))?;
                t.push(emit(Instruction::GetNamed(self.string(property)?)))?;
            }
            Expr::OptionalIndex { object, index } => {
                t.push(Task::Expr(*object))?;
                t.push(Task::ChainCheck(1, span))?;
                t.push(Task::Expr(*index))?;
                t.push(emit(Instruction::GetIndex))?;
            }
            // `#sec-function-calls-runtime-semantics-evaluation`: the function and its this value
            // (the base of a property reference, the binding object of a `with` statement for a
            // name looked up by name, otherwise `undefined`), the arguments in order, then
            // EvaluateCall. `?.(...)` tests the function value, with its this value under it. A
            // call of the name `eval` is a direct eval when it calls `%eval%` (step 6), which
            // only the run tells: `CallEval`.
            Expr::Call { callee, arguments } | Expr::OptionalCall { callee, arguments } => {
                self.callee(*callee, span, &mut t)?;
                let optional = matches!(node.expr, Expr::OptionalCall { .. });
                if optional {
                    t.push(Task::ChainCheck(2, span))?;
                }
                // ArgumentListEvaluation with a spread: the list as an Array.
                if self.has_spread(arguments.iter()) {
                    self.argument_array(arguments, span, &mut t)?;
                    if !optional && self.ast.is_direct_eval_callee(*callee) {
                        t.push(emit(Instruction::CallEvalSpread))?;
                    } else {
                        t.push(emit(Instruction::CallSpread))?;
                    }
                    return Ok(t);
                }
                t.extend(arguments.iter().map(|e| Task::Expr(*e)))?;
                let n = u32::try_from(arguments.len())
                    .map_err(|_| CompileError::TooMany("arguments"))?;
                if !optional && self.ast.is_direct_eval_callee(*callee) {
                    t.push(emit(Instruction::CallEval(n)))?;
                } else {
                    t.push(emit(Instruction::Call(n)))?;
                }
            }
            // `#sec-new-operator-runtime-semantics-evaluation`.
            Expr::New { callee, arguments } => {
                t.push(Task::Expr(*callee))?;
                let arguments = arguments.as_deref().unwrap_or(&[]);
                if self.has_spread(arguments.iter()) {
                    self.argument_array(arguments, span, &mut t)?;
                    t.push(emit(Instruction::NewSpread))?;
                    return Ok(t);
                }
                t.extend(arguments.iter().map(|e| Task::Expr(*e)))?;
                let n = u32::try_from(arguments.len())
                    .map_err(|_| CompileError::TooMany("arguments"))?;
                t.push(emit(Instruction::New(n)))?;
            }
            // `#sec-template-literals-runtime-semantics-evaluation`: the TV of each template
            // string and the value of each substitution through ToString, in source order,
            // concatenated as they come (`Add` of two Strings). An empty string adds nothing and
            // is left out; a template of empty strings alone is the empty String.
            Expr::Template {
                strings,
                substitutions,
            } => {
                if strings.len() != substitutions.len().saturating_add(1) {
                    return Err(malformed("a template literal"));
                }
                let mut started = false;
                for (i, string) in strings.iter().enumerate() {
                    if let Some(e) = i.checked_sub(1).and_then(|s| substitutions.get(s)) {
                        t.push(Task::Expr(*e))?;
                        t.push(emit(Instruction::ToString))?;
                        if started {
                            t.push(emit(Instruction::Add))?;
                        }
                        started = true;
                    }
                    let last = i + 1 == strings.len();
                    if !string.cooked.is_empty() || (last && !started) {
                        let k = self.string_units(&string.cooked)?;
                        t.push(emit(Instruction::String(k)))?;
                        if started {
                            t.push(emit(Instruction::Add))?;
                        }
                        started = true;
                    }
                }
            }
            // `#sec-tagged-templates-runtime-semantics-evaluation`: the tag and its this value as
            // for a call (`#sec-evaluatecall`), then ArgumentListEvaluation of the template
            // (`#sec-template-literals-runtime-semantics-argumentlistevaluation`): the template
            // object of the site, then the substitutions in order; then the call, never a direct
            // eval.
            Expr::TaggedTemplate {
                tag,
                strings,
                substitutions,
            } => {
                if strings.len() != substitutions.len().saturating_add(1) {
                    return Err(malformed("a tagged template"));
                }
                self.callee(*tag, span, &mut t)?;
                let site = self.constants.template_site(strings, self.m)?;
                t.push(emit(Instruction::TemplateObject(site)))?;
                t.extend(substitutions.iter().map(|e| Task::Expr(*e)))?;
                let n =
                    u32::try_from(strings.len()).map_err(|_| CompileError::TooMany("arguments"))?;
                t.push(emit(Instruction::Call(n)))?;
            }
            // A YieldExpression stands only in a generator; the parse of `yield*` ends on the
            // refusal of slice 7: never a parsed script.
            Expr::Yield {
                argument,
                delegate: false,
            } => self.yield_expression(*argument, span, &mut t)?,
            Expr::Yield { delegate: true, .. } => return Err(malformed("a yield* expression")),
        }
        Ok(t)
    }

    /// BindingInitialization of the ObjectBindingPattern `id` (`#sec-runtime-semantics-bindinginitialization`,
    /// KeyedBindingInitialization, `#sec-runtime-semantics-keyedbindinginitialization`) or
    /// DestructuringAssignmentEvaluation of the ObjectAssignmentPattern `id`
    /// (`#sec-runtime-semantics-destructuringassignmentevaluation`,
    /// `#sec-runtime-semantics-keyeddestructuringassignmentevaluation`), of the value on top,
    /// which stays: RequireObjectCoercible, then each property in source order. A target that is
    /// a reference (a `var` name, an assignment target) is evaluated first; then GetV of the
    /// key; the default for `undefined`; then PutValue, the initialization of the binding, or
    /// the nested pattern.
    ///
    /// A computed key is evaluated and converted before the reference of its target
    /// (`#sec-destructuring-binding-patterns-runtime-semantics-propertybindinginitialization`,
    /// `#sec-runtime-semantics-propertydestructuringassignmentevaluation`) and read with
    /// [`Instruction::PatternGetKey`]. With a `rest` property, last, every key before it is pushed
    /// and stays on the stack, the list of its excluded names, until the rest property is
    /// destructured ([`Instruction::ObjectRest`]); without one, a computed key is dropped after
    /// its property.
    fn object_pattern(
        &mut self,
        id: ExprId,
        properties: &[PatternProperty],
        kind: PatternKind,
        rest: bool,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        t.push(Task::Emit(Instruction::RequireObjectCoercible, span))?;
        let mut kept: u32 = 0;
        for (i, p) in properties.iter().enumerate() {
            let n = u32::try_from(i).map_err(|_| CompileError::TooMany("properties"))?;
            if rest && i + 1 == properties.len() {
                return self.rest_property(id, n, p, kind, kept, t);
            }
            let name = self.pattern_key(p, rest, t)?;
            let pspan = p.span;
            let get = |depth: u32| {
                let read = match name {
                    Some(name) => Instruction::PatternGet { depth, name },
                    None => Instruction::PatternGetKey { depth, kept },
                };
                Task::Emit(read, pspan)
            };
            match &p.target {
                // `var`: ResolveBinding before the value is read, then PutValue.
                PatternTarget::Binding(leaf) if kind == PatternKind::Var => {
                    let target = Target::Binding(self.binding(Site::Bound(id, n))?);
                    t.append(self.reference(&target, false, leaf.span)?)?;
                    t.push(get(target.reference_items()))?;
                    self.pattern_default(p.default, pspan, t)?;
                    t.push(Task::Emit(self.put(&target)?, leaf.span))?;
                }
                // InitializeReferencedBinding: the store into the slot, or into the global
                // environment's [[DeclarativeRecord]] for a name of the script's top level.
                PatternTarget::Binding(leaf) => {
                    let store = self.bound_store(id, n)?;
                    t.push(get(0))?;
                    self.pattern_default(p.default, pspan, t)?;
                    t.push(Task::Emit(store, leaf.span))?;
                }
                PatternTarget::Expr(e) => {
                    let nested = matches!(
                        self.ast.expr(*e).map(|x| &x.expr),
                        Some(Expr::ObjectPattern { .. } | Expr::ArrayPattern { .. })
                    );
                    if nested {
                        t.push(get(0))?;
                        self.pattern_default(p.default, pspan, t)?;
                        t.push(Task::Expr(*e))?;
                    } else {
                        let target = self.target(*e)?;
                        t.append(self.reference(&target, false, pspan)?)?;
                        t.push(get(target.reference_items()))?;
                        self.pattern_default(p.default, pspan, t)?;
                        t.push(Task::Emit(self.put(&target)?, pspan))?;
                    }
                }
            }
            t.push(Task::Emit(Instruction::Pop, pspan))?;
            if name.is_none() {
                if rest {
                    kept = kept
                        .checked_add(1)
                        .ok_or(CompileError::TooMany("properties"))?;
                } else {
                    t.push(Task::Emit(Instruction::Pop, pspan))?;
                }
            }
        }
        Ok(())
    }

    /// The key of the property `p` of an object pattern: the `strings` index of a literal key
    /// read by [`Instruction::PatternGet`], or `None` when the key is pushed first, a computed
    /// one evaluated and converted, a literal one as a String when the pattern has a `rest`
    /// property.
    fn pattern_key(
        &mut self,
        p: &PatternProperty,
        rest: bool,
        t: &mut Tasks<'_, '_>,
    ) -> Result<Option<u32>, CompileError> {
        if let PropertyKey::Computed(e) = p.key {
            let at = self.ast.expr(e).map_or(p.span, |x| x.span);
            t.push(Task::Expr(e))?;
            t.push(Task::Emit(Instruction::ToPropertyKey, at))?;
            return Ok(None);
        }
        let key = property_key(&p.key, self.m)?;
        let name = self.string_units(&key);
        self.m.release(key);
        let name = name?;
        if rest {
            t.push(Task::Emit(Instruction::String(name), p.span))?;
            return Ok(None);
        }
        Ok(Some(name))
    }

    /// The InitializeReferencedBinding of the `n`th property of the binding pattern `id`: the
    /// store into the slot, or into the global environment's [[DeclarativeRecord]] for a name of
    /// the script's top level.
    fn bound_store(&mut self, id: ExprId, n: u32) -> Result<Instruction, CompileError> {
        Ok(match self.binding(Site::Bound(id, n))? {
            Binding::Local(slot) => Instruction::SetLocal(slot),
            Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
            Binding::Global(name) | Binding::Dynamic(name) => {
                Instruction::InitGlobal(self.string(&name)?)
            }
            _ => return Err(malformed("a binding of a pattern")),
        })
    }

    /// The rest property `p`, the `n`th and last of the pattern `id`, with the `kept` keys of the
    /// properties before it on the stack: the reference of its target, then
    /// [`Instruction::ObjectRest`], then PutValue or the initialization of the binding
    /// (`#sec-destructuring-binding-patterns-runtime-semantics-restbindinginitialization`,
    /// `#sec-runtime-semantics-restdestructuringassignmentevaluation`); then the keys are dropped.
    fn rest_property(
        &mut self,
        id: ExprId,
        n: u32,
        p: &PatternProperty,
        kind: PatternKind,
        kept: u32,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let pspan = p.span;
        let rest = |depth: u32| Task::Emit(Instruction::ObjectRest { depth, kept }, pspan);
        match &p.target {
            PatternTarget::Binding(leaf) if kind == PatternKind::Var => {
                let target = Target::Binding(self.binding(Site::Bound(id, n))?);
                t.append(self.reference(&target, false, leaf.span)?)?;
                t.push(rest(target.reference_items()))?;
                t.push(Task::Emit(self.put(&target)?, leaf.span))?;
            }
            PatternTarget::Binding(leaf) => {
                let store = self.bound_store(id, n)?;
                t.push(rest(0))?;
                t.push(Task::Emit(store, leaf.span))?;
            }
            PatternTarget::Expr(e) => {
                let target = self.target(*e)?;
                t.append(self.reference(&target, false, pspan)?)?;
                t.push(rest(target.reference_items()))?;
                t.push(Task::Emit(self.put(&target)?, pspan))?;
            }
        }
        if kept > 0 {
            t.push(Task::Emit(Instruction::Nip(kept), pspan))?;
        }
        t.push(Task::Emit(Instruction::Pop, pspan))
    }

    /// Whether one of `list` is a spread element.
    fn has_spread<'e>(&self, mut list: impl Iterator<Item = &'e ExprId>) -> bool {
        list.any(|e| matches!(self.ast.expr(*e).map(|n| &n.expr), Some(Expr::Spread(_))))
    }

    /// The operand of the spread element `e`, with its span, or `None` when `e` is none.
    fn spread_operand(&self, e: ExprId) -> Option<(ExprId, Span)> {
        match self.ast.expr(e).map(|n| &n.expr) {
            Some(Expr::Spread(inner)) => {
                let span = self.ast.expr(*inner).map(|n| n.span)?;
                Some((*inner, span))
            }
            _ => None,
        }
    }

    /// ArrayAccumulation (`#sec-runtime-semantics-arrayaccumulation`) of the elements of an
    /// array literal with a spread element, onto the new Array on top: before the first spread
    /// each element at its index, as without one; from it on, the index is the length the run
    /// reaches, so each element is appended ([`Instruction::ArrayAppend`]), each spread
    /// ([`Instruction::SpreadArray`]) appends its values, and a run of elisions grows the
    /// length ([`Instruction::ArrayGrow`]).
    fn spread_elements(
        &mut self,
        elements: &[Option<ExprId>],
        mut span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let mut next: u32 = 0;
        let mut elided: u32 = 0;
        let mut spread = false;
        for element in elements {
            let Some(e) = element else {
                elided = elided
                    .checked_add(1)
                    .ok_or(CompileError::TooMany("array elements"))?;
                continue;
            };
            span = self.ast.expr(*e).map_or(span, |n| n.span);
            if elided > 0 {
                let grow = if spread {
                    Instruction::ArrayGrow(elided)
                } else {
                    next = next
                        .checked_add(elided)
                        .ok_or(CompileError::TooMany("array elements"))?;
                    Instruction::ArrayLength(next)
                };
                t.push(Task::Emit(grow, span))?;
                elided = 0;
            }
            if let Some((inner, at)) = self.spread_operand(*e) {
                t.push(Task::Expr(inner))?;
                t.push(Task::Emit(Instruction::SpreadArray, at))?;
                spread = true;
            } else if spread {
                t.push(Task::Expr(*e))?;
                t.push(Task::Emit(Instruction::ArrayAppend, span))?;
            } else {
                t.push(Task::Expr(*e))?;
                t.push(Task::Emit(Instruction::DefineIndex(next), span))?;
                next = next
                    .checked_add(1)
                    .ok_or(CompileError::TooMany("array elements"))?;
            }
        }
        if elided > 0 {
            t.push(Task::Emit(Instruction::ArrayGrow(elided), span))?;
        }
        Ok(())
    }

    /// ArgumentListEvaluation (`#sec-runtime-semantics-argumentlistevaluation`) of `arguments`
    /// with a spread element, as a new Array: each argument appended, each spread's values
    /// appended in order.
    fn argument_array(
        &mut self,
        arguments: &[ExprId],
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        t.push(Task::Emit(Instruction::NewArray, span))?;
        for e in arguments {
            match self.spread_operand(*e) {
                Some((inner, at)) => {
                    t.push(Task::Expr(inner))?;
                    t.push(Task::Emit(Instruction::SpreadArray, at))?;
                }
                None => {
                    let at = self.ast.expr(*e).map_or(span, |n| n.span);
                    t.push(Task::Expr(*e))?;
                    t.push(Task::Emit(Instruction::ArrayAppend, at))?;
                }
            }
        }
        Ok(())
    }

    /// `yield` with its operand or none (`#sec-generator-function-definitions-runtime-semantics-evaluation`,
    /// Yield, `#sec-yield`): the operand, or `undefined`, then [`Instruction::Yield`], which
    /// suspends; the resumption leaves the received value and its tag, and the code dispatches
    /// on the tag as the end of a `finally` block does ("Suspended frames", item 6,
    /// `docs/design/engine.md`): a normal completion is the value of the expression, a return
    /// completion returns the value as a `return` statement here does, through every `finally`
    /// block around it, after [`Instruction::YieldReturn`] dropped what the expression around
    /// the `yield` holds, and [`Instruction::EndFinally`] throws a throw completion here.
    fn yield_expression(
        &mut self,
        argument: Option<ExprId>,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let slot = self
            .generator
            .ok_or_else(|| malformed("a yield outside a generator"))?;
        match argument {
            Some(e) => t.push(Task::Expr(e))?,
            None => t.push(Task::Emit(Instruction::Undefined, span))?,
        }
        let (normal, returned) = (self.label(), self.label());
        t.push(Task::Emit(Instruction::Yield(slot), span))?;
        t.push(Task::Emit(
            Instruction::JumpIfTag {
                tag: TAG_NORMAL,
                target: normal,
            },
            span,
        ))?;
        t.push(Task::Emit(
            Instruction::JumpIfTag {
                tag: TAG_RETURN,
                target: returned,
            },
            span,
        ))?;
        t.push(Task::Emit(Instruction::EndFinally, span))?;
        t.push(Task::Bind(returned))?;
        t.push(Task::Emit(Instruction::YieldReturn(self.items), span))?;
        t.push(Task::Return(span))?;
        t.push(Task::Bind(normal))?;
        Ok(())
    }

    /// IteratorBindingInitialization of the ArrayBindingPattern `id`
    /// (`#sec-runtime-semantics-iteratorbindinginitialization`) or
    /// IteratorDestructuringAssignmentEvaluation of the ArrayAssignmentPattern `id`
    /// (`#sec-runtime-semantics-iteratordestructuringassignmentevaluation`), of the value on
    /// top, which stays: GetIterator of a copy, its record kept above the value; then each
    /// element in source order, the reference of its target first, then its step
    /// ([`Instruction::PatternStep`], [`Instruction::PatternSkip`] for an elision,
    /// [`Instruction::PatternRest`] for the rest element), the default for `undefined`, then
    /// PutValue, the initialization of the binding, or the nested pattern; at the end
    /// IteratorClose with a normal completion unless the iterator is done, and the record
    /// dropped. A throw from the first step to the end of the last element enters the
    /// pattern's handler ([`HandlerKind::Pattern`]), which runs the same close with the throw
    /// completion and throws again (`#sec-runtime-semantics-bindinginitialization`, step 4 of
    /// `BindingPattern : ArrayBindingPattern`, and
    /// `#sec-runtime-semantics-destructuringassignmentevaluation`, step 4 of
    /// `ArrayAssignmentPattern`). `docs/design/engine.md`, decision 3, item 5 of "The iterator
    /// of `for`-`of`, of an array pattern and of a spread".
    fn array_pattern(
        &mut self,
        id: ExprId,
        elements: &[Option<PatternElement>],
        kind: PatternKind,
        rest: bool,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let (start, end, entry) = (self.label(), self.label(), self.label());
        t.push(Task::Emit(Instruction::Dup, span))?;
        t.push(Task::Emit(Instruction::GetIterator, span))?;
        t.push(Task::Bind(start))?;
        for (i, element) in elements.iter().enumerate() {
            let n = u32::try_from(i).map_err(|_| CompileError::TooMany("elements"))?;
            match element {
                None => t.push(Task::Emit(Instruction::PatternSkip, span))?,
                Some(p) => {
                    let last = rest && i + 1 == elements.len();
                    self.array_element(id, n, p, kind, last, t)?;
                }
            }
        }
        t.push(Task::Bind(end))?;
        t.push(Task::Emit(Instruction::Undefined, span))?;
        t.push(Task::Emit(Instruction::Tag(TAG_NORMAL), span))?;
        t.push(Task::Bind(entry))?;
        t.push(Task::Emit(Instruction::IteratorClose, span))?;
        t.push(Task::Emit(Instruction::Bury(2), span))?;
        t.push(Task::Emit(Instruction::Bury(2), span))?;
        t.push(Task::Emit(Instruction::Pop, span))?;
        t.push(Task::Emit(Instruction::EndFinally, span))?;
        t.push(Task::Handler {
            kind: HandlerKind::Pattern,
            start,
            end,
            target: entry,
            items: self.items,
            envs: self.envs,
        })
    }

    /// The element `p`, the `n`th of the array pattern `id`, the rest element when `rest`: the
    /// reference of its target, its step, its default, then PutValue, the initialization of
    /// the binding or the nested pattern, and the value dropped.
    fn array_element(
        &mut self,
        id: ExprId,
        n: u32,
        p: &PatternElement,
        kind: PatternKind,
        rest: bool,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let pspan = p.span;
        let step = |depth: u32| {
            let step = if rest {
                Instruction::PatternRest(depth)
            } else {
                Instruction::PatternStep(depth)
            };
            Task::Emit(step, pspan)
        };
        match &p.target {
            // `var`: ResolveBinding before the value is read, then PutValue.
            PatternTarget::Binding(leaf) if kind == PatternKind::Var => {
                let target = Target::Binding(self.binding(Site::Bound(id, n))?);
                t.append(self.reference(&target, false, leaf.span)?)?;
                t.push(step(target.reference_items()))?;
                self.pattern_default(p.default, pspan, t)?;
                t.push(Task::Emit(self.put(&target)?, leaf.span))?;
            }
            PatternTarget::Binding(leaf) => {
                let store = self.bound_store(id, n)?;
                t.push(step(0))?;
                self.pattern_default(p.default, pspan, t)?;
                t.push(Task::Emit(store, leaf.span))?;
            }
            PatternTarget::Expr(e) => {
                let nested = matches!(
                    self.ast.expr(*e).map(|x| &x.expr),
                    Some(Expr::ObjectPattern { .. } | Expr::ArrayPattern { .. })
                );
                if nested {
                    t.push(step(0))?;
                    self.pattern_default(p.default, pspan, t)?;
                    t.push(Task::Expr(*e))?;
                } else {
                    let target = self.target(*e)?;
                    t.append(self.reference(&target, false, pspan)?)?;
                    t.push(step(target.reference_items()))?;
                    self.pattern_default(p.default, pspan, t)?;
                    t.push(Task::Emit(self.put(&target)?, pspan))?;
                }
            }
        }
        t.push(Task::Emit(Instruction::Pop, pspan))
    }

    /// The property `p` of an object literal when its key is computed or it is a spread, `true`
    /// then: the key evaluated and converted, then the value, a getter, a setter or a method
    /// defined under it, an anonymous function named after it at run time
    /// (`#sec-object-initializer-runtime-semantics-propertydefinitionevaluation`,
    /// `#sec-runtime-semantics-definemethod`); a spread's value copied by CopyDataProperties.
    fn dynamic_property(
        &mut self,
        p: &Property,
        t: &mut Tasks<'_, '_>,
    ) -> Result<bool, CompileError> {
        if let PropertyValue::Spread(e) = p.value {
            let at = self.ast.expr(e).map_or(p.span, |x| x.span);
            t.push(Task::Expr(e))?;
            t.push(Task::Emit(Instruction::CopyDataProperties, at))?;
            return Ok(true);
        }
        let PropertyKey::Computed(key) = p.key else {
            return Ok(false);
        };
        let at = self.ast.expr(key).map_or(p.span, |x| x.span);
        t.push(Task::Expr(key))?;
        t.push(Task::Emit(Instruction::ToPropertyKey, at))?;
        match p.value {
            PropertyValue::Init(e) => {
                t.push(Task::Expr(e))?;
                if anonymous_function(self.ast, e).is_some() {
                    t.push(Task::Emit(Instruction::SetFunctionName, p.span))?;
                }
                t.push(Task::Emit(Instruction::DefineComputedField, p.span))?;
            }
            PropertyValue::Get(f) | PropertyValue::Set(f) | PropertyValue::Method(f) => {
                let fspan = self.ast.function(f).map_or(p.span, |x| x.span);
                t.push(Task::Emit(Instruction::Closure(f.0), fspan))?;
                let define = match p.value {
                    PropertyValue::Get(_) => Instruction::DefineComputedGetter,
                    PropertyValue::Set(_) => Instruction::DefineComputedSetter,
                    _ => Instruction::DefineComputedMethod,
                };
                t.push(Task::Emit(define, p.span))?;
            }
            PropertyValue::Shorthand(_)
            | PropertyValue::CoverInitialized(..)
            | PropertyValue::Spread(_) => {
                return Err(malformed("a computed key without a value"));
            }
        }
        Ok(true)
    }

    /// The `Initializer` `default` of a property of a pattern, if any, for the value on top:
    /// evaluated in its place only when the value is `undefined`.
    fn pattern_default(
        &mut self,
        default: Option<ExprId>,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let Some(default) = default else {
            return Ok(());
        };
        let given = self.label();
        t.push(Task::Emit(Instruction::JumpIfNotUndefined(given), span))?;
        t.push(Task::Expr(default))?;
        t.push(Task::Bind(given))?;
        Ok(())
    }

    /// The this value and the function of a call with callee `callee` (`#sec-function-calls`),
    /// `[this, f]`: a property access gives its base as this value, also after a `?.` and in a
    /// parenthesized optional chain, whose short-circuit then leaves `undefined` for both
    /// (`#sec-optional-chaining-evaluation` gives the Reference of the chain's last link).
    fn callee(
        &mut self,
        callee: ExprId,
        span: Span,
        t: &mut Tasks<'_, '_>,
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
                    t.push(Task::Emit(Instruction::Undefined, span))?;
                    t.push(Task::Expr(callee))?;
                    return Ok(());
                }
                t.push(Task::ChainOpen(Short::Pair))?;
                (inner_node, Some(node.span))
            }
            _ => (node, None),
        };
        match &link.expr {
            Expr::Member { object, property } | Expr::OptionalMember { object, property } => {
                t.push(Task::Expr(*object))?;
                if matches!(link.expr, Expr::OptionalMember { .. }) {
                    t.push(Task::ChainCheck(1, link.span))?;
                }
                t.push(Task::Emit(Instruction::Dup, span))?;
                t.push(Task::Emit(
                    Instruction::GetNamed(self.string(property)?),
                    span,
                ))?;
            }
            Expr::Index { object, index } | Expr::OptionalIndex { object, index } => {
                t.push(Task::Expr(*object))?;
                if matches!(link.expr, Expr::OptionalIndex { .. }) {
                    t.push(Task::ChainCheck(1, link.span))?;
                }
                t.push(Task::Emit(Instruction::Dup, span))?;
                t.push(Task::Expr(*index))?;
                t.push(Task::Emit(Instruction::GetIndex, span))?;
            }
            // A name looked up by name gives its record's WithBaseObject as this value.
            Expr::Identifier(_) if chain.is_none() => match self.binding(Site::Expr(callee))? {
                Binding::Dynamic(name) => {
                    t.push(Task::Emit(
                        Instruction::GetNameForCall(self.string(&name)?),
                        span,
                    ))?;
                }
                _ => {
                    t.push(Task::Emit(Instruction::Undefined, span))?;
                    t.push(Task::Expr(callee))?;
                }
            },
            _ => {
                t.push(Task::Emit(Instruction::Undefined, span))?;
                t.push(Task::Expr(callee))?;
            }
        }
        if let Some(chain_span) = chain {
            t.push(Task::ChainClose(chain_span))?;
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
    ) -> Result<Option<Tasks<'a, 'm>>, CompileError> {
        let link = self
            .ast
            .expr(inner)
            .ok_or_else(|| malformed("an optional chain"))?;
        let mut t = Tasks::new(self.m);
        t.push(Task::ChainOpen(Short::True))?;
        match &link.expr {
            Expr::Member { object, property } | Expr::OptionalMember { object, property } => {
                t.push(Task::Expr(*object))?;
                if matches!(link.expr, Expr::OptionalMember { .. }) {
                    t.push(Task::ChainCheck(1, link.span))?;
                }
                t.push(Task::Emit(
                    Instruction::DeleteNamed(self.string(property)?),
                    span,
                ))?;
            }
            Expr::Index { object, index } | Expr::OptionalIndex { object, index } => {
                t.push(Task::Expr(*object))?;
                if matches!(link.expr, Expr::OptionalIndex { .. }) {
                    t.push(Task::ChainCheck(1, link.span))?;
                }
                t.push(Task::Expr(*index))?;
                t.push(Task::Emit(Instruction::DeleteIndex, span))?;
            }
            _ => return Ok(None),
        }
        t.push(Task::ChainClose(chain_span))?;
        Ok(Some(t))
    }

    /// The unary operators (`#sec-unary-operators`).
    fn unary(
        &mut self,
        op: UnaryOp,
        operand: ExprId,
        span: Span,
        t: &mut Tasks<'_, '_>,
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
                        t.push(emit(Instruction::DeleteGlobal(self.string(&name)?)))?;
                    }
                    Binding::Dynamic(name) => {
                        t.push(emit(Instruction::DeleteName(self.string(&name)?)))?;
                    }
                    _ => t.push(emit(Instruction::False))?,
                },
                Expr::Member { object, property } => {
                    t.push(Task::Expr(*object))?;
                    t.push(emit(Instruction::DeleteNamed(self.string(property)?)))?;
                }
                Expr::Index { object, index } => {
                    t.push(Task::Expr(*object))?;
                    t.push(Task::Expr(*index))?;
                    t.push(emit(Instruction::DeleteIndex))?;
                }
                Expr::OptionalChain(inner) => {
                    match self.delete_chain(*inner, operand_node.span, span)? {
                        Some(tasks) => t.append(tasks)?,
                        None => {
                            t.push(Task::Expr(operand))?;
                            t.push(emit(Instruction::Pop))?;
                            t.push(emit(Instruction::True))?;
                        }
                    }
                }
                _ => {
                    t.push(Task::Expr(operand))?;
                    t.push(emit(Instruction::Pop))?;
                    t.push(emit(Instruction::True))?;
                }
            },
            // `#sec-void-operator-runtime-semantics-evaluation`.
            UnaryOp::Void => {
                t.push(Task::Expr(operand))?;
                t.push(emit(Instruction::Pop))?;
                t.push(emit(Instruction::Undefined))?;
            }
            // `#sec-typeof-operator-runtime-semantics-evaluation`: an unresolvable reference
            // gives "undefined".
            UnaryOp::Typeof => match self.name_binding(operand)? {
                Some(Binding::Global(name)) => {
                    t.push(emit(Instruction::TypeofGlobal(self.string(&name)?)))?;
                }
                Some(Binding::Dynamic(name)) => {
                    t.push(emit(Instruction::TypeofName(self.string(&name)?)))?;
                }
                _ => {
                    t.push(Task::Expr(operand))?;
                    t.push(emit(Instruction::Typeof))?;
                }
            },
            UnaryOp::Plus => {
                t.push(Task::Expr(operand))?;
                t.push(emit(Instruction::ToNumber))?;
            }
            UnaryOp::Minus => {
                t.push(Task::Expr(operand))?;
                t.push(emit(Instruction::Negate))?;
            }
            UnaryOp::BitNot => {
                t.push(Task::Expr(operand))?;
                t.push(emit(Instruction::BitNot))?;
            }
            UnaryOp::Not => {
                t.push(Task::Expr(operand))?;
                t.push(emit(Instruction::Not))?;
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
