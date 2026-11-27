//! Class definitions (`#sec-class-definitions`): ClassDefinitionEvaluation
//! (`#sec-runtime-semantics-classdefinitionevaluation`) as compiled code in the code around the
//! class (`docs/design/engine.md`, "Classes", item 2), each step an instruction, in the clause's
//! order. Written by `story:class-definition-evaluation`.
//!
//! 1. The class scope ([`ScopeKey::Class`], `resolve.rs`, `class_scope`): the class's own name,
//!    immutable and uninitialized, a record when a function of the class or a lookup by name
//!    reaches it.
//! 2. The prototype ([`Instruction::ClassPrototype`]) and F
//!    ([`Instruction::MakeClassConstructor`]), with F's `prototype` and the prototype's
//!    `constructor`.
//! 3. The elements in source order, each through a function of its own: [`Compiler::compile_method`]
//!    defines a method, a getter or a setter, static or not, with a literal or a computed key;
//!    [`Compiler::compile_field`] keeps a field's computed key in its hidden binding of the class
//!    scope (`story:public-class-fields`); [`Compiler::compile_static_block`] and
//!    [`Compiler::compile_private_method`] answer [`CompileError::Tree`] here: a source with a
//!    static block or a private method is held by the parser until the story that replaces the
//!    function (`story:class-static-blocks`, `story:private-names-and-private-methods`).
//! 4. The class's own name initialized with F; the initializer of the instance elements stored
//!    in F's box when the class has some ([`Instruction::StoreInitializer`]); the static private
//!    methods added to F ([`Compiler::static_private_adds`]); then the static elements called in
//!    source order ([`Compiler::static_calls`]).
//! 5. The class scope left; F is the value. A throw in any step leaves the scope as a throw
//!    leaves a block's.
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
//! [`initializer_private_methods`] (nothing here yet), then [`initializer_fields`]; the function
//! of a run of static fields is the same kind of template, whose code defines the run's fields
//! on F, and the class definition calls it in source order ([`Compiler::static_calls`]). A base class constructor
//! whose class has instance elements begins with `This; InitInstance; Pop`
//! ([`constructor_prologue`]). No list of elements and no walk over one exist at run time: the
//! order is the order of the code.

use swell_syntax::ast::{
    Class, ClassElement, ClassKey, Expr, ExprId, Function, FunctionId, FunctionKind, Span,
};

use crate::bytecode::Instruction;
use crate::resolve::{
    Binding, LexicalPlace, ScopeKey, Site, anonymous_function, initializer_fields_of, property_key,
};
use crate::statement::leave_scope;
use crate::{CompileError, Compiler, Task, Tasks, ThisUse, malformed};

/// The prologue of the template of the class constructor `id` (`#sec-ecmascript-function-objects-construct-argumentslist-newtarget`,
/// InitializeInstanceElements before OrdinaryCallEvaluateBody): of a derived constructor the
/// hidden binding `this` made uninitialized, in its local or record form, before the parameters
/// are bound (`docs/design/engine.md`, "Classes", item 6: `constructor(a = super())` binds it in
/// a parameter's initializer), its instance elements initialized after each SuperCall instead;
/// of a base one `This; InitInstance; Pop` when its class has instance elements; nothing
/// otherwise.
pub(crate) fn constructor_prologue(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    span: Span,
) -> Result<(), CompileError> {
    if c.derived {
        let uninitialize = match c.binding(Site::This(id))? {
            Binding::Lexical {
                at: LexicalPlace::Local(slot),
                ..
            } => Instruction::Uninitialize(slot),
            Binding::Lexical {
                at: LexicalPlace::Env { hops, slot },
                ..
            } => Instruction::UninitializeEnv { hops, slot },
            _ => return Err(malformed("the `this` binding of a derived constructor")),
        };
        return c.emit(uninitialize, span);
    }
    if !c.resolution.instance_elements.contains(&id) {
        return Ok(());
    }
    c.emit(Instruction::This, span)?;
    c.emit(Instruction::InitInstance, span)?;
    c.emit(Instruction::Pop, span)
}

/// The code of the default constructor of a class with a heritage (`docs/design/engine.md`,
/// "Classes", item 7; ClassDefinitionEvaluation, step 14.a, the default constructor's
/// [[Construct]] behaviour): its template keeps its arguments (`binds_parameters` with no
/// parameter), which follow its `locals` own slots; `GetSuperConstructor`, `SuperCallKept`
/// of those slots, which passes them on with no iterator read, `BindThis`, the instance
/// elements initialized (`Dup; InitInstance; Pop`), then `KeepThis; Return`. Its prologue made
/// the binding uninitialized ([`constructor_prologue`]).
pub(crate) fn default_derived_constructor(
    c: &mut Compiler<'_, '_>,
    locals: u32,
    span: Span,
) -> Result<(), CompileError> {
    c.emit(Instruction::GetSuperConstructor, span)?;
    c.emit(Instruction::SuperCallKept(locals), span)?;
    let bind = c.this_instruction(ThisUse::Bind)?;
    c.emit(bind, span)?;
    c.emit(Instruction::Dup, span)?;
    c.emit(Instruction::InitInstance, span)?;
    c.emit(Instruction::Pop, span)?;
    c.emit_return(span)
}

/// The code of a `FieldInitializer` template `f`, function `id` (`docs/design/engine.md`,
/// "Classes", item 4; `#sec-initializeinstanceelements`): the initializer of a class's instance
/// elements, whose pass 1 adds the private methods to `this` and pass 2 defines the fields in
/// source order, or the function of a run of static fields, which defines the run's fields on
/// `this` the same way; then `Undefined; Return`.
pub(crate) fn instance_initializer(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    f: &Function,
) -> Result<(), CompileError> {
    initializer_private_methods(c, f)?;
    initializer_fields(c, id, f)?;
    c.emit(Instruction::Undefined, f.span)?;
    c.emit(Instruction::Return, f.span)
}

/// Pass 1 of [`instance_initializer`]: PrivateMethodOrAccessorAdd of each private method of the
/// class (`#sec-privatemethodoraccessoradd`). Nothing here: `story:private-names-and-private-methods`
/// fills it.
fn initializer_private_methods(
    _c: &mut Compiler<'_, '_>,
    _f: &Function,
) -> Result<(), CompileError> {
    Ok(())
}

/// Pass 2 of [`instance_initializer`]: DefineField (`#sec-definefield`) of each field whose
/// initializer is code of the function `id`, in source order: the fields that are not static,
/// or the fields of one run of static fields. `This` once, then per field its value (the
/// initializer, or `undefined`) and `DefineField(k)` for a literal key, or the key read from its
/// hidden binding, the value, `SetFunctionName` for an anonymous function (an anonymous class
/// names itself, `SetClassName`) and `DefineComputedField` for a computed one; then `Pop`. Each
/// is CreateDataPropertyOrThrow, which throws a `TypeError` where `this` refuses the property.
fn initializer_fields(
    c: &mut Compiler<'_, '_>,
    id: FunctionId,
    f: &Function,
) -> Result<(), CompileError> {
    let (at, first) = c
        .resolution
        .field_classes
        .get(&id)
        .copied()
        .ok_or_else(|| malformed("a field initializer outside a class"))?;
    let ast = c.ast;
    let mut receiver = false;
    for element in initializer_fields_of(at.class(ast)?, id, first) {
        let ClassElement::Field {
            key,
            initializer,
            span,
            ..
        } = element
        else {
            continue;
        };
        let span = *span;
        if !receiver {
            c.emit(Instruction::This, f.span)?;
            receiver = true;
        }
        match key {
            ClassKey::Literal(key) => {
                field_value(c, *initializer, span)?;
                let units = property_key(key, c.m)?;
                let k = c.string_units(&units);
                c.m.release(units);
                c.emit(Instruction::DefineField(k?), span)?;
            }
            ClassKey::Computed(e) => {
                let load = match c.binding(Site::FieldKeyRead(*e))? {
                    Binding::Local(n) => Instruction::GetLocal(n),
                    Binding::Env { hops, slot } => Instruction::GetEnv { hops, slot },
                    _ => return Err(malformed("a field key's binding")),
                };
                c.emit(load, span)?;
                field_value(c, *initializer, span)?;
                if let Some(value) = initializer
                    && !matches!(ast.expr(*value).map(|x| &x.expr), Some(Expr::Class(_)))
                    && anonymous_function(ast, *value).is_some()
                {
                    c.emit(Instruction::SetFunctionName, span)?;
                }
                c.emit(Instruction::DefineComputedField, span)?;
            }
            // `story:private-fields-and-private-in` adds the private add.
            ClassKey::Private(_) => return Err(malformed("a private field")),
        }
    }
    if receiver {
        c.emit(Instruction::Pop, f.span)?;
    }
    Ok(())
}

/// The value of a field: its initializer `e`, or `undefined` at `span` without one.
fn field_value(
    c: &mut Compiler<'_, '_>,
    e: Option<ExprId>,
    span: Span,
) -> Result<(), CompileError> {
    match e {
        Some(e) => crate::function::expression_value(c, e),
        None => c.emit(Instruction::Undefined, span),
    }
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
        let ctor = class.constructor;
        // All parts of a class are strict mode code (`#sec-strict-mode-code`): in non-strict
        // code the class's code is enclosed by `SetStrict(true)` and `SetStrict(false)`; a
        // class inside the code of a class, which is strict, emits neither.
        let enclosing = self.strict;
        if !enclosing {
            t.push(Task::Strict(true, span))?;
        }
        // Step 1, steps 1 to 6 of the clause: the class scope.
        let record = self.enter_scope(ScopeKey::Class(ctor), span, t)?;
        // Step 2, steps 7 to 19: the prototype (after the heritage, evaluated in the class
        // scope, inside the class's strict code), then F with `prototype` and `constructor`.
        let derived = class.heritage.is_some();
        match class.heritage {
            Some(heritage) => self.class_heritage(heritage, span, t)?,
            None => t.push(Task::Emit(Instruction::ClassPrototype, span))?,
        }
        t.push(Task::Emit(
            Instruction::MakeClassConstructor {
                function: ctor.0,
                derived,
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
        // Step 5: the scope left; F is the value. The code after the class has the strictness
        // of the code around it.
        leave_scope(record, span, t)?;
        if !enclosing {
            t.push(Task::Strict(false, span))?;
        }
        Ok(())
    }

    /// ClassHeritage of ClassDefinitionEvaluation (`#sec-runtime-semantics-classdefinitionevaluation`,
    /// steps 8.a to 8.h and 10), in the class scope: the expression `heritage`, then
    /// [`Instruction::ClassHeritage`], which checks the parent (`null`, or a constructor whose
    /// `prototype` is an object or `null`) and leaves constructorParent under the prototype it
    /// makes from protoParent; `MakeClassConstructor { derived: true }` takes the first for F.
    fn class_heritage(
        &mut self,
        heritage: swell_syntax::ast::ExprId,
        span: Span,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let at = self.ast.expr(heritage).map_or(span, |x| x.span);
        t.push(Task::Expr(heritage))?;
        t.push(Task::Emit(Instruction::ClassHeritage, at))
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

    /// ClassFieldDefinitionEvaluation (`#sec-runtime-semantics-classfielddefinitionevaluation`),
    /// step 3 of "Classes", item 2: a computed key evaluated, converted by `ToPropertyKey` and
    /// stored in its hidden binding of the class scope ([`Site::FieldKey`]), `[p, F]` before and
    /// after; a literal key makes nothing. The initializer makes nothing here: it is code of the
    /// function of its run ([`instance_initializer`]).
    fn compile_field(
        &mut self,
        element: &ClassElement,
        t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        let ClassElement::Field { key, span, .. } = element else {
            return Err(malformed("a class field"));
        };
        let ClassKey::Computed(e) = key else {
            return Ok(());
        };
        let at = self.ast.expr(*e).map_or(*span, |x| x.span);
        let store = match self.binding(Site::FieldKey(*e))? {
            Binding::Local(n) => Instruction::SetLocal(n),
            Binding::Env { hops, slot } => Instruction::SetEnv { hops, slot },
            _ => return Err(malformed("a field key's binding")),
        };
        t.push(Task::Expr(*e))?;
        t.push(Task::Emit(Instruction::ToPropertyKey, at))?;
        t.push(Task::Emit(store, at))?;
        t.push(Task::Emit(Instruction::Pop, at))?;
        Ok(())
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

    /// A private method or accessor (`#sec-runtime-semantics-classelementevaluation`): its
    /// function made and kept in its hidden binding of the private record.
    /// `story:private-names-and-private-methods` replaces it; a source with a private name is
    /// held until then.
    fn compile_private_method(
        &mut self,
        _element: &ClassElement,
        _t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
        Err(malformed("a private method"))
    }

    /// The template of the initializer of the instance elements of `class` ("Classes", item 4),
    /// which [`instance_initializer`] compiles: [`Class::instance_elements`].
    fn initializer_template(&mut self, class: &Class) -> Result<FunctionId, CompileError> {
        class
            .instance_elements
            .ok_or_else(|| malformed("the initializer of instance elements"))
    }

    /// Step 4: the static private methods added to F (PrivateMethodOrAccessorAdd), before the
    /// first static element runs. Nothing here: `story:private-names-and-private-methods` fills
    /// it.
    fn static_private_adds(
        &mut self,
        _class: &Class,
        _t: &mut Tasks<'_, '_>,
    ) -> Result<(), CompileError> {
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
    /// that defines them on `this` ([`instance_initializer`]): the function the parser gave the
    /// run, which every field of the run names.
    fn static_field_run(
        &mut self,
        _class: &Class,
        first: &ClassElement,
    ) -> Result<FunctionId, CompileError> {
        match first {
            ClassElement::Field {
                is_static: true,
                function,
                ..
            } => Ok(*function),
            _ => Err(malformed("a run of static fields")),
        }
    }
}
