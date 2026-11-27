//! Class definitions (`#sec-class-definitions`): ClassDefinitionEvaluation
//! (`#sec-runtime-semantics-classdefinitionevaluation`) as compiled code in the code around the
//! class (`docs/design/engine.md`, "Classes", item 2), each step an instruction, in the clause's
//! order. Written by `story:class-definition-evaluation`.
//!
//! 1. The class scope ([`ScopeKey::Class`], `resolve.rs`, `class_scope`): the class's own name,
//!    immutable and uninitialized, a record when a function of the class or a lookup by name
//!    reaches it; then, when the class declares a private name, the private record
//!    ([`ScopeKey::Private`], [`Compiler::enter_private_record`]) with a new Private Name per
//!    private identifier (`story:private-names-and-private-methods`).
//! 2. The prototype ([`Instruction::ClassPrototype`]) and F
//!    ([`Instruction::MakeClassConstructor`]), with F's `prototype` and the prototype's
//!    `constructor`.
//! 3. The elements in source order, each through a function of its own: [`Compiler::compile_method`]
//!    defines a method, a getter or a setter, static or not, with a literal or a computed key;
//!    [`Compiler::compile_private_method`] makes a private one's function and stores it in the
//!    private record; [`Compiler::compile_field`] and [`Compiler::compile_static_block`] answer
//!    [`CompileError::Tree`] here: a source with a field or a static block is held by the parser
//!    until the story that replaces the function (`story:public-class-fields`,
//!    `story:class-static-blocks`).
//! 4. The class's own name initialized with F; the initializer of the instance elements stored
//!    in F's box when the class has some ([`Instruction::StoreInitializer`]); the static private
//!    methods added to F ([`Compiler::static_private_adds`]); then the static elements called in
//!    source order ([`Compiler::static_calls`]).
//! 5. The private record and the class scope left; F is the value. A throw in any step leaves
//!    them as a throw leaves a block's.
//!
//! All parts of a class are strict mode code (`#sec-strict-mode-code`). Its computed keys run in
//! the frame of the code around it, whose strictness the run loop reads at every instruction,
//! so in non-strict code the class's code is enclosed by [`Instruction::SetStrict`] `true` and
//! `false` ([`Task::Strict`]); a handler records the strictness of its place
//! ([`crate::bytecode::Handler::strict`]), and a return completion that leaves the class (the
//! `return` of a `yield` in a key) puts the code's own back first (`Compiler::exit_return`).
//! An anonymous class whose name NamedEvaluation gives at run time, under a computed key of an
//! object literal, gets it by [`Instruction::SetClassName`] right after F is made
//! ([`crate::resolve::Resolution::named_by_key`]).
//!
//! The instance elements are one function, their initializer ("Classes", item 4), whose code
//! [`instance_initializer`] emits in two passes, each a function of its own:
//! [`initializer_private_methods`], then [`initializer_fields`], which emits nothing here. A
//! base class constructor whose class has instance elements begins with `This; InitInstance;
//! Pop` ([`constructor_prologue`]). No list of elements and no walk over one exist at run time:
//! the order is the order of the code.

use swell_syntax::ast::{Class, ClassElement, ClassKey, Function, FunctionId, FunctionKind, Span};

use crate::bytecode::Instruction;
use crate::resolve::{Binding, ScopeKey, Site, property_key};
use crate::statement::{leave_scope, slot_names};
use crate::{CompileError, Compiler, Task, Tasks, malformed};

/// The prologue of the template of the class constructor `id` (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`,
/// InitializeInstanceElements before OrdinaryCallEvaluateBody): `This; InitInstance; Pop`
/// before the parameters are bound, when its class has instance elements; nothing otherwise.
pub(crate) fn constructor_prologue(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    span: Span,
) -> Result<(), CompileError> {
    if !c.resolution.instance_elements.contains(&id) {
        return Ok(());
    }
    c.emit(Instruction::This, span)?;
    c.emit(Instruction::InitInstance, span)?;
    c.emit(Instruction::Pop, span)
}

/// The code of the initializer of a class's instance elements, the `FieldInitializer` template
/// `f` (`docs/design/engine.md`, "Classes", item 4; `#sec-initializeinstanceelements`): pass 1
/// adds the private methods to `this`, pass 2 defines the fields in source order, then
/// `Undefined; Return`.
pub(crate) fn instance_initializer(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    f: &Function,
) -> Result<(), CompileError> {
    initializer_private_methods(c, id, f)?;
    initializer_fields(c, f)?;
    c.emit(Instruction::Undefined, f.span)?;
    c.emit(Instruction::Return, f.span)
}

/// Pass 1 of [`instance_initializer`] `id`: PrivateMethodOrAccessorAdd of each private method,
/// getter and setter of the class that is not static, in source order
/// (`#sec-initializeinstanceelements`, step 3; `#sec-privatemethodoraccessoradd`): `This;
/// AddPrivateMethod; Pop` each, the Private Name and the function read from the private
/// record, which is the initializer's [[Environment]].
fn initializer_private_methods(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    f: &Function,
) -> Result<(), CompileError> {
    let Some(methods) = c.resolution.initializer_methods.get(&id) else {
        return Ok(());
    };
    for &method in methods {
        let add = c.private_add(method)?;
        c.emit(Instruction::This, f.span)?;
        c.emit(add, f.span)?;
        c.emit(Instruction::Pop, f.span)?;
    }
    Ok(())
}

/// Pass 2 of [`instance_initializer`]: DefineField of each field that is not static, in source
/// order (`#sec-definefield`). Nothing here: `story:public-class-fields` fills it.
fn initializer_fields(_c: &mut Compiler<'_, '_>, _f: &Function) -> Result<(), CompileError> {
    Ok(())
}

impl<'a, 'm> Compiler<'a, 'm> {
    /// ClassDefinitionEvaluation of `class` (`#sec-runtime-semantics-classdefinitionevaluation`):
    /// the tasks that leave F on top, with `span` the class's.
    pub(crate) fn class_definition(
        &mut self,
        class: &Class,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        // The parse holds a heritage back (`swell_syntax::HOLDS`): `story:derived-classes-and-super-call`.
        if class.heritage.is_some() {
            return Err(malformed("a class heritage"));
        }
        let ctor = class.constructor;
        // All parts of a class are strict mode code (`#sec-strict-mode-code`): in non-strict
        // code the class's code is enclosed by `SetStrict(true)` and `SetStrict(false)`; a
        // class inside the code of a class, which is strict, emits neither.
        let enclosing = self.strict;
        if !enclosing {
            t.push(Task::Strict(true, span))?;
        }
        // Step 1, steps 1 to 6 of the clause: the class scope. The heritage, evaluated in it
        // (`story:derived-classes-and-super-call`), goes after it and before the prototype,
        // inside the class's strict code.
        let record = self.enter_scope(ScopeKey::Class(ctor), span, t)?;
        // Step 1, the end: the private record, with a new Private Name per private identifier.
        let private = self.enter_private_record(ctor, span, t)?;
        // Step 2, steps 7 to 19: the prototype, then F with `prototype` and `constructor`.
        t.push(Task::Emit(Instruction::ClassPrototype, span))?;
        t.push(Task::Emit(
            Instruction::MakeClassConstructor {
                function: ctor.0,
                derived: false,
            },
            span,
        ))?;
        // SetFunctionName(F, className) when NamedEvaluation gives the name at run time, before
        // any element is defined.
        if self.resolution.named_by_key.contains(&ctor) {
            t.push(Task::Emit(Instruction::SetClassName, span))?;
        }
        // Step 3, steps 20 to 26: the elements, in source order, `[p, F]` before and after each.
        for element in &class.elements {
            match element {
                ClassElement::Method {
                    key: ClassKey::Private(_),
                    ..
                } => self.compile_private_method(element, t)?,
                ClassElement::Method { .. } => self.compile_method(element, t)?,
                ClassElement::Field { .. } => self.compile_field(element, t)?,
                ClassElement::StaticBlock { .. } => {
                    self.compile_static_block(element)?;
                }
            }
        }
        // Step 4, steps 27 to 31: the own name initialized with F, then `[F]`.
        if class.name.is_some() {
            let store = self.plain_store(Site::ClassName(ctor))?;
            t.push(Task::Emit(store, span))?;
        }
        t.push(Task::Emit(Instruction::Nip(1), span))?;
        if self.resolution.instance_elements.contains(&ctor) {
            let f = self.initializer_template(class)?;
            t.push(Task::Emit(Instruction::StoreInitializer(f.0), span))?;
        }
        self.static_private_adds(class, t)?;
        self.static_calls(class, span, t)?;
        // Step 5: the private record and the scope left; F is the value. The code after the
        // class has the strictness of the code around it.
        leave_scope(private, span, t)?;
        leave_scope(record, span, t)?;
        if !enclosing {
            t.push(Task::Strict(false, span))?;
        }
        Ok(())
    }

    /// The instruction that initializes the binding of `site` through its plain place: a slot
    /// of the frame or of a record, or the global environment's declarative record.
    pub(crate) fn plain_store(&mut self, site: Site) -> Result<Instruction, CompileError> {
        Ok(match self.binding(site)? {
            crate::resolve::Binding::Local(n) => Instruction::SetLocal(n),
            crate::resolve::Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
            crate::resolve::Binding::Global(name) | crate::resolve::Binding::Dynamic(name) => {
                Instruction::InitGlobal(self.string(&name)?)
            }
            _ => return Err(malformed("a class binding")),
        })
    }

    /// ClassElementEvaluation of a method, a getter or a setter with a public key
    /// (`#sec-runtime-semantics-classelementevaluation`, `#sec-runtime-semantics-methoddefinitionevaluation`):
    /// its home object on top (the prototype, or F when it is static), the key converted by
    /// `ToPropertyKey` when it is computed, the closure, then the property, not enumerable.
    fn compile_method(
        &mut self,
        element: &ClassElement,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let ClassElement::Method {
            is_static,
            key,
            kind,
            function,
            span,
        } = element
        else {
            return Err(malformed("a class method"));
        };
        let span = *span;
        if *is_static {
            t.push(Task::Emit(Instruction::Dup, span))?;
        } else {
            t.push(Task::Emit(Instruction::Dup2, span))?;
            t.push(Task::Emit(Instruction::Pop, span))?;
        }
        let fspan = self.ast.function(*function).map_or(span, |x| x.span);
        match key {
            ClassKey::Literal(key) => {
                let units = property_key(key, self.m)?;
                let k = self.string_units(&units);
                self.m.release(units);
                let k = k?;
                let define = match kind {
                    FunctionKind::Getter => Instruction::DefineClassGetter(k),
                    FunctionKind::Setter => Instruction::DefineClassSetter(k),
                    _ => Instruction::DefineClassMethod(k),
                };
                t.push(Task::Emit(Instruction::Closure(function.0), fspan))?;
                t.push(Task::Emit(define, span))?;
            }
            ClassKey::Computed(e) => {
                let at = self.ast.expr(*e).map_or(span, |x| x.span);
                t.push(Task::Expr(*e))?;
                t.push(Task::Emit(Instruction::ToPropertyKey, at))?;
                let define = match kind {
                    FunctionKind::Getter => Instruction::DefineClassComputedGetter,
                    FunctionKind::Setter => Instruction::DefineClassComputedSetter,
                    _ => Instruction::DefineClassComputedMethod,
                };
                t.push(Task::Emit(Instruction::Closure(function.0), fspan))?;
                t.push(Task::Emit(define, span))?;
            }
            ClassKey::Private(_) => return Err(malformed("a private method as a public one")),
        }
        t.push(Task::Emit(Instruction::Pop, span))?;
        Ok(())
    }

    /// ClassFieldDefinitionEvaluation (`#sec-runtime-semantics-classfielddefinitionevaluation`):
    /// a computed key evaluated and kept in its hidden binding. `story:public-class-fields`
    /// replaces it; a source with a field is held until then.
    fn compile_field(
        &mut self,
        _element: &ClassElement,
        _t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        Err(malformed("a class field"))
    }

    /// The template of a ClassStaticBlock (`#sec-runtime-semantics-classstaticblockdefinitionevaluation`),
    /// which [`Compiler::static_calls`] calls. `story:class-static-blocks` replaces it; a source
    /// with a static block is held until then.
    fn compile_static_block(
        &mut self,
        _element: &ClassElement,
    ) -> Result<FunctionId, CompileError> {
        Err(malformed("a class static block"))
    }

    /// The end of step 1 (`#sec-runtime-semantics-classdefinitionevaluation`, steps 6 and 7;
    /// "Private names", item 3): the private record of the class with constructor `ctor`, when
    /// it declares a private name, pushed inside the class scope: `PushEnv`, or `PushLexicalEnv`
    /// with the names of its slots when a lookup by name may reach it (a direct `eval` in the
    /// class body); then `NewPrivateName` into each slot of a private identifier, whose binding
    /// is named with its `#`. Answers whether a record was pushed.
    fn enter_private_record(
        &mut self,
        ctor: FunctionId,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<bool, CompileError> {
        let resolution = self.resolution;
        let Some(scope) = resolution.lexical_scopes.get(&ScopeKey::Private(ctor)) else {
            return Ok(false);
        };
        if scope.env.is_empty() {
            return Ok(false);
        }
        let push = if scope.named {
            let table = slot_names(self.constants, &scope.env, self.m)?;
            let index = u32::try_from(self.scopes.len())
                .map_err(|_| CompileError::TooMany("environments"))?;
            self.m.push(&mut self.scopes, table)?;
            Instruction::PushLexicalEnv(index)
        } else {
            let slots = u32::try_from(scope.env.len())
                .map_err(|_| CompileError::TooMany("environment slots"))?;
            Instruction::PushEnv(slots)
        };
        t.push(Task::Emit(push, span))?;
        t.push(Task::EnvsUp)?;
        for (slot, (name, _)) in scope.env.iter().enumerate() {
            if !name.starts_with('#') {
                continue;
            }
            let slot =
                u32::try_from(slot).map_err(|_| CompileError::TooMany("environment slots"))?;
            let name = self.string(name)?;
            t.push(Task::Emit(Instruction::NewPrivateName { slot, name }, span))?;
        }
        Ok(true)
    }

    /// A private method or accessor (`#sec-runtime-semantics-classelementevaluation`,
    /// `#sec-runtime-semantics-methoddefinitionevaluation`, PrivateIdentifier): with `[p, F]`
    /// on top, its function made with its home object (the prototype, or F when it is static)
    /// and stored in its binding of the private record, which holds it for the adds; nothing
    /// is defined on the home object.
    fn compile_private_method(
        &mut self,
        element: &ClassElement,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let ClassElement::Method {
            is_static,
            function,
            span,
            ..
        } = element
        else {
            return Err(malformed("a private method"));
        };
        let span = *span;
        if *is_static {
            t.push(Task::Emit(Instruction::Dup, span))?;
        } else {
            t.push(Task::Emit(Instruction::Dup2, span))?;
            t.push(Task::Emit(Instruction::Pop, span))?;
        }
        let fspan = self.ast.function(*function).map_or(span, |x| x.span);
        let store = match self.binding(Site::PrivateFunction(*function))? {
            Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
            _ => return Err(malformed("a private method's binding")),
        };
        t.push(Task::Emit(Instruction::MakeMethod(function.0), fspan))?;
        t.push(Task::Emit(store, span))?;
        t.push(Task::Emit(Instruction::Pop, span))?;
        t.push(Task::Emit(Instruction::Pop, span))?;
        Ok(())
    }

    /// `AddPrivateMethod` of the private method, getter or setter `method`, its Private Name and
    /// its function at the places its add sites resolve to.
    pub(crate) fn private_add(&self, method: FunctionId) -> Result<Instruction, CompileError> {
        let narrow =
            |n: u32| u16::try_from(n).map_err(|_| CompileError::TooMany("private record slots"));
        let (
            Binding::Env { hops, slot },
            Binding::Env {
                hops: function_hops,
                slot: function_slot,
            },
        ) = (
            self.binding(Site::PrivateAddName(method))?,
            self.binding(Site::PrivateAddFunction(method))?,
        )
        else {
            return Err(malformed("a private method's binding"));
        };
        Ok(Instruction::AddPrivateMethod {
            hops: narrow(hops)?,
            slot: narrow(slot)?,
            function_hops: narrow(function_hops)?,
            function_slot: narrow(function_slot)?,
        })
    }

    /// The template of the initializer of the instance elements of `class` ("Classes", item 4),
    /// which [`instance_initializer`] compiles.
    fn initializer_template(&mut self, class: &Class) -> Result<FunctionId, CompileError> {
        class
            .instance_elements
            .ok_or_else(|| malformed("the initializer of instance elements"))
    }

    /// Step 4: the static private methods, getters and setters added to F
    /// (`#sec-runtime-semantics-classdefinitionevaluation`, step 31.b:
    /// PrivateMethodOrAccessorAdd), in source order, with F on top, before the first static
    /// element runs.
    fn static_private_adds(
        &mut self,
        class: &Class,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        for element in &class.elements {
            let ClassElement::Method {
                is_static: true,
                key: ClassKey::Private(_),
                function,
                span,
                ..
            } = element
            else {
                continue;
            };
            let add = self.private_add(*function)?;
            t.push(Task::Emit(add, *span))?;
        }
        Ok(())
    }

    /// Step 4, the static elements in source order (`docs/design/engine.md`, "Classes", item 4):
    /// per run of consecutive static fields and per static block, `Dup; MakeMethod(f); Call(0);
    /// Pop` with F on top, f the template its arm answers: [`Compiler::static_field_run`] for a
    /// run, [`Compiler::compile_static_block`] for a block.
    fn static_calls(
        &mut self,
        class: &Class,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let mut in_run = false;
        for element in &class.elements {
            let f = match element {
                ClassElement::Field {
                    is_static: true, ..
                } if in_run => continue,
                ClassElement::Field {
                    is_static: true, ..
                } => {
                    in_run = true;
                    self.static_field_run(class, element)?
                }
                ClassElement::StaticBlock { .. } => {
                    in_run = false;
                    self.compile_static_block(element)?
                }
                _ => {
                    in_run = false;
                    continue;
                }
            };
            t.push(Task::Emit(Instruction::Dup, span))?;
            t.push(Task::Emit(Instruction::MakeMethod(f.0), span))?;
            t.push(Task::Emit(Instruction::Call(0), span))?;
            t.push(Task::Emit(Instruction::Pop, span))?;
        }
        Ok(())
    }

    /// The template of the run of static fields that starts at `first`, a `FieldInitializer`
    /// that defines them on `this`. `story:public-class-fields` replaces it; a source with a
    /// field is held until then.
    fn static_field_run(
        &mut self,
        _class: &Class,
        _first: &ClassElement,
    ) -> Result<FunctionId, CompileError> {
        Err(malformed("a run of static fields"))
    }
}
