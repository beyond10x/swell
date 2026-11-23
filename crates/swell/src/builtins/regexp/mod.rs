//! RegExp objects (ECMA-262 22.2, `#sec-regexp-regular-expression-objects`), written by
//! `story:regexp-objects-exec-and-literals`: the constructor `RegExp ( pattern, flags )`
//! (`#sec-regexp-pattern-flags`) with RegExpAlloc (`#sec-regexpalloc`), RegExpInitialize
//! (`#sec-regexpinitialize`) and RegExpCreate (`#sec-regexpcreate`), `get RegExp [
//! %Symbol.species% ]` (`#sec-get-regexp-%symbol.species%`), and `%RegExp.prototype%`
//! (`#sec-properties-of-the-regexp-prototype-object`), an ordinary object, with `constructor`,
//! `exec` and `test` (`regexp/exec.rs`), the accessors `dotAll`, `flags`
//! (`#sec-get-regexp.prototype.flags`), `global`, `hasIndices`, `ignoreCase`, `multiline`,
//! `source` (`#sec-get-regexp.prototype.source`, with EscapeRegExpPattern,
//! `#sec-escaperegexppattern`), `sticky`, `unicode` and `unicodeSets` (RegExpHasFlag,
//! `#sec-regexphasflag`), and `toString` (`#sec-regexp.prototype.tostring`).
//!
//! RegExpInitialize charges one step per code unit of the pattern before ParsePattern
//! (`docs/design/engine.md`, decision 7, "What one step of the matcher is"), as PerformEval
//! charges eval code, then parses and compiles it with `swell-regexp` through the budget of
//! `regexp/budget.rs`, with the `parser_nesting` of the agent. Any error of the pattern, the
//! nesting limit included, is a `SyntaxError` of the current realm with the crate's message and
//! no position after it, never counted. A refusal of the crate (decision 7, "Refusals until the
//! later stories of slice 5": the flag `v` and a group name outside ASCII from
//! the parse, the flag or a modifier `i` from the compile step) is the same `SyntaxError`, which
//! the agent counts before it throws ([`Agent::count_refusal`]), caught or not. An error of
//! ToString or of the flags is thrown before the parse and is not counted. The program the
//! compile step charged becomes the bytes the object's record owns (`object/regexp.rs`).
//!
//! RegExpAlloc's object is made after the parse and compile step, once the program exists: the
//! steps between (the conversions and the parse) cannot see it, so the order is not observable.
//!
//! The methods keyed by `%Symbol.match%`, `%Symbol.replace%`, `%Symbol.search%` and
//! `%Symbol.split%` (`regexp/symbol.rs`) are `story:regexp-symbol-methods-and-string-match-search`,
//! defined here in the order of 22.2.6 under the keys of their symbols. The one keyed by
//! `%Symbol.matchAll%` with `%RegExpStringIteratorPrototype%` (`regexp/match_all.rs`) and
//! `RegExp.escape` (`#sec-regexp.escape`, `regexp/escape.rs`) are
//! `story:regexp-match-all-and-escape`.

mod budget;
mod escape;
mod exec;
mod match_all;
mod symbol;

use swell_regexp::{CompileError, Flags, ParseError, PatternError, Program, Stop};
use swell_regexp::{compile_pattern, parse_pattern};

use budget::AgentBudget;

use crate::builtins::object::argument;
use crate::builtins::string::StringBuilder;
use crate::heap::Heap;
use crate::heap::string::HeapString;
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host, Termination};
use crate::object::builtin_function::BuiltinId;
use crate::object::ordinary::OrdinaryData;
use crate::object::property::{Key, Property, Slot};
use crate::object::regexp::{RegExpObject, program_box_bytes, regexp_owned_bytes};
use crate::object::{ObjectKind, ObjectRecord};
use crate::realm::intrinsics::{Intrinsic, Making};
use crate::value::{ObjectRef, StringRef, Value};

/// The builtins of this module, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RegExpBuiltin {
    /// `RegExp ( pattern, flags )` (22.2.4.1).
    Constructor,
    /// `RegExp.escape ( string )` (22.2.5.1).
    Escape,
    /// `get RegExp [ %Symbol.species% ]` (22.2.5.3).
    Species,
    /// `RegExp.prototype.exec ( string )` (22.2.6.2).
    Exec,
    /// `get RegExp.prototype.flags` (22.2.6.4).
    Flags,
    /// The getters of one flag each (22.2.6.3, 22.2.6.5 to 22.2.6.7, 22.2.6.10, 22.2.6.15,
    /// 22.2.6.18, 22.2.6.19).
    HasFlag(FlagGetter),
    /// `RegExp.prototype [ %Symbol.match% ] ( string )` (22.2.6.8).
    Match,
    /// `RegExp.prototype [ %Symbol.matchAll% ] ( string )` (22.2.6.9).
    MatchAll,
    /// `RegExp.prototype [ %Symbol.replace% ] ( string, replaceValue )` (22.2.6.11).
    Replace,
    /// `RegExp.prototype [ %Symbol.search% ] ( string )` (22.2.6.12).
    Search,
    /// `get RegExp.prototype.source` (22.2.6.13).
    Source,
    /// `RegExp.prototype [ %Symbol.split% ] ( string, limit )` (22.2.6.14).
    Split,
    /// `RegExp.prototype.test ( S )` (22.2.6.16).
    Test,
    /// `RegExp.prototype.toString ( )` (22.2.6.17).
    ToString,
    /// `%RegExpStringIteratorPrototype%.next ( )` (22.2.9.2.1).
    StringIteratorNext,
}

/// A getter of one flag: the property it is and the code unit RegExpHasFlag looks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FlagGetter {
    DotAll,
    Global,
    HasIndices,
    IgnoreCase,
    Multiline,
    Sticky,
    Unicode,
    UnicodeSets,
}

impl FlagGetter {
    /// The name of the property.
    fn name(self) -> &'static str {
        match self {
            FlagGetter::DotAll => "dotAll",
            FlagGetter::Global => "global",
            FlagGetter::HasIndices => "hasIndices",
            FlagGetter::IgnoreCase => "ignoreCase",
            FlagGetter::Multiline => "multiline",
            FlagGetter::Sticky => "sticky",
            FlagGetter::Unicode => "unicode",
            FlagGetter::UnicodeSets => "unicodeSets",
        }
    }

    /// Whether `flags`, parsed from [[OriginalFlags]], contain the getter's code unit.
    fn reads(self, flags: Flags) -> bool {
        match self {
            FlagGetter::DotAll => flags.dot_all(),
            FlagGetter::Global => flags.global(),
            FlagGetter::HasIndices => flags.has_indices(),
            FlagGetter::IgnoreCase => flags.ignore_case(),
            FlagGetter::Multiline => flags.multiline(),
            FlagGetter::Sticky => flags.sticky(),
            FlagGetter::Unicode => flags.unicode(),
            FlagGetter::UnicodeSets => flags.unicode_sets(),
        }
    }
}

/// The properties of `%RegExp.prototype%` after `constructor`, in the order of 22.2.6.
enum PrototypeProperty {
    /// A method: its name, its "length" and its steps.
    Method(&'static str, u32, RegExpBuiltin),
    /// An accessor whose set accessor function is undefined: its name and its getter's steps.
    Getter(&'static str, RegExpBuiltin),
    /// A method keyed by a well-known symbol: the symbol's name in the method's name, its
    /// "length" and its steps.
    SymbolMethod(&'static str, u32, RegExpBuiltin),
}

const PROTOTYPE_PROPERTIES: [PrototypeProperty; 18] = [
    PrototypeProperty::Method("exec", 1, RegExpBuiltin::Exec),
    PrototypeProperty::Getter("dotAll", RegExpBuiltin::HasFlag(FlagGetter::DotAll)),
    PrototypeProperty::Getter("flags", RegExpBuiltin::Flags),
    PrototypeProperty::Getter("global", RegExpBuiltin::HasFlag(FlagGetter::Global)),
    PrototypeProperty::Getter("hasIndices", RegExpBuiltin::HasFlag(FlagGetter::HasIndices)),
    PrototypeProperty::Getter("ignoreCase", RegExpBuiltin::HasFlag(FlagGetter::IgnoreCase)),
    PrototypeProperty::SymbolMethod("match", 1, RegExpBuiltin::Match),
    PrototypeProperty::SymbolMethod("matchAll", 1, RegExpBuiltin::MatchAll),
    PrototypeProperty::Getter("multiline", RegExpBuiltin::HasFlag(FlagGetter::Multiline)),
    PrototypeProperty::SymbolMethod("replace", 2, RegExpBuiltin::Replace),
    PrototypeProperty::SymbolMethod("search", 1, RegExpBuiltin::Search),
    PrototypeProperty::Getter("source", RegExpBuiltin::Source),
    PrototypeProperty::SymbolMethod("split", 2, RegExpBuiltin::Split),
    PrototypeProperty::Getter("sticky", RegExpBuiltin::HasFlag(FlagGetter::Sticky)),
    PrototypeProperty::Method("test", 1, RegExpBuiltin::Test),
    PrototypeProperty::Method("toString", 0, RegExpBuiltin::ToString),
    PrototypeProperty::Getter("unicode", RegExpBuiltin::HasFlag(FlagGetter::Unicode)),
    PrototypeProperty::Getter(
        "unicodeSets",
        RegExpBuiltin::HasFlag(FlagGetter::UnicodeSets),
    ),
];

/// The properties `get RegExp.prototype.flags` reads, in the order of its steps, with the code
/// unit each appends.
const FLAG_PROPERTIES: [(&str, u8); 8] = [
    ("hasIndices", b'd'),
    ("global", b'g'),
    ("ignoreCase", b'i'),
    ("multiline", b'm'),
    ("dotAll", b's'),
    ("unicode", b'u'),
    ("unicodeSets", b'v'),
    ("sticky", b'y'),
];

/// The most flags a valid flags String has; a longer one is charged a step per code unit before
/// it is read.
const MOST_FLAGS: usize = 8;

/// Why ParsePattern or CompilePattern gave no program.
enum Failure {
    /// The pattern's String names no record: an engine bug.
    Stale,
    /// An error or a refusal of the parse.
    Pattern(PatternError),
    /// A refusal of the compile step.
    Refused(CompileError),
    /// The budget stopped the parse or the compile step.
    Stopped(Stop),
}

/// ParsePattern and then CompilePattern of `units` with `flags` and `nesting`, every growth
/// charged to `budget`; the tree is released once the program is made or refused.
fn parse_and_compile(
    units: &[u16],
    flags: Flags,
    nesting: u32,
    budget: &mut AgentBudget<'_>,
) -> Result<Program, Failure> {
    let tree = match parse_pattern(units, flags, nesting, budget) {
        Ok(tree) => tree,
        Err(ParseError::Pattern(error)) => return Err(Failure::Pattern(error)),
        Err(ParseError::Stopped(stop)) => return Err(Failure::Stopped(stop)),
    };
    let compiled = compile_pattern(&tree, budget);
    tree.release(budget);
    match compiled {
        Ok(program) => Ok(program),
        Err(CompileError::Stopped(stop)) => Err(Failure::Stopped(stop)),
        Err(refused) => Err(Failure::Refused(refused)),
    }
}

/// [`parse_and_compile`] of the code units of a Latin-1 String: they are copied to UTF-16
/// first, the copy charged before it is made and credited when the step ends.
fn parse_and_compile_latin1(
    bytes: &[u8],
    flags: Flags,
    nesting: u32,
    budget: &mut AgentBudget<'_>,
) -> Result<Program, Failure> {
    use swell_regexp::Budget;
    let copy_bytes = bytes.len().saturating_mul(size_of::<u16>());
    budget.charge(copy_bytes).map_err(Failure::Stopped)?;
    let mut units: Vec<u16> = Vec::new();
    if units.try_reserve_exact(bytes.len()).is_err() {
        budget.credit(copy_bytes);
        return Err(Failure::Stopped(Stop::MemoryBudget));
    }
    units.extend(bytes.iter().map(|&byte| u16::from(byte)));
    let built = parse_and_compile(&units, flags, nesting, budget);
    drop(units);
    budget.credit(copy_bytes);
    built
}

impl<H: Host> Agent<H> {
    /// `%RegExp%` (22.2.4) and `%RegExp.prototype%` (22.2.6), answered in that order: the
    /// constructor's "length" 2, its "prototype" { [[Writable]]: false, [[Enumerable]]: false,
    /// [[Configurable]]: false } (22.2.5.2), `escape` of "length" 1 (22.2.5.1, a method as
    /// below) and `get [Symbol.species]` (22.2.5.3); the
    /// prototype, an ordinary object whose [[Prototype]] is `%Object.prototype%` and which is
    /// not a RegExp instance (22.2.6), with its properties in the order of 22.2.6: methods {
    /// [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: true } and accessors {
    /// [[Enumerable]]: false, [[Configurable]]: true } (clause 18).
    pub(crate) fn create_regexp_constructor(
        &mut self,
        making: Making,
    ) -> Completion<(ObjectRef, ObjectRef)> {
        let prototype =
            self.ordinary_object_create(Some(making.object_prototype), OrdinaryData::Plain)?;
        let constructor = self.create_builtin_constructor(
            making,
            "RegExp",
            2,
            BuiltinId::RegExp(RegExpBuiltin::Constructor),
            prototype,
        )?;
        // 22.2.5.1: `RegExp.escape ( string )`.
        self.define_builtin_method(
            making,
            constructor,
            "escape",
            1,
            BuiltinId::RegExp(RegExpBuiltin::Escape),
        )?;
        // 22.2.5.3: an accessor whose set accessor function is undefined, named "get
        // [Symbol.species]".
        let species = self.create_builtin(
            making,
            "get [Symbol.species]",
            0,
            BuiltinId::RegExp(RegExpBuiltin::Species),
        )?;
        self.add_property(
            constructor,
            Key::Symbol(making.symbols.species),
            getter_property(species),
        )?;
        for property in &PROTOTYPE_PROPERTIES {
            match *property {
                PrototypeProperty::Method(name, length, id) => {
                    self.define_builtin_method(
                        making,
                        prototype,
                        name,
                        length,
                        BuiltinId::RegExp(id),
                    )?;
                }
                PrototypeProperty::Getter(name, id) => {
                    let getter = self.create_builtin(
                        making,
                        &format!("get {name}"),
                        0,
                        BuiltinId::RegExp(id),
                    )?;
                    let key = self.name_key(name)?;
                    self.add_property(prototype, key, getter_property(getter))?;
                }
                PrototypeProperty::SymbolMethod(name, length, id) => {
                    let symbol = match id {
                        RegExpBuiltin::Match => making.symbols.r#match,
                        RegExpBuiltin::MatchAll => making.symbols.match_all,
                        RegExpBuiltin::Replace => making.symbols.replace,
                        RegExpBuiltin::Search => making.symbols.search,
                        _ => making.symbols.split,
                    };
                    let method = self.create_builtin(
                        making,
                        &format!("[Symbol.{name}]"),
                        length,
                        BuiltinId::RegExp(id),
                    )?;
                    self.add_property(
                        prototype,
                        Key::Symbol(symbol),
                        Property::data(Value::Object(method), true, false, true),
                    )?;
                }
            }
        }
        Ok((constructor, prototype))
    }

    /// The steps of the builtins of this module.
    pub(crate) fn run_regexp_builtin(
        &mut self,
        id: RegExpBuiltin,
        function: ObjectRef,
        this: Value,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        match id {
            RegExpBuiltin::Constructor => self.regexp_constructor(function, args, new_target),
            RegExpBuiltin::Escape => self.regexp_escape(args),
            // `#sec-get-regexp-%symbol.species%`: 1. Return the this value.
            RegExpBuiltin::Species => Ok(this),
            RegExpBuiltin::Exec => self.regexp_prototype_exec(this, args),
            RegExpBuiltin::Flags => self.regexp_prototype_flags(this),
            RegExpBuiltin::HasFlag(flag) => self.regexp_has_flag(this, flag),
            RegExpBuiltin::Match => self.regexp_symbol_match(this, args),
            RegExpBuiltin::MatchAll => self.regexp_symbol_match_all(this, args),
            RegExpBuiltin::Replace => self.regexp_symbol_replace(this, args),
            RegExpBuiltin::Search => self.regexp_symbol_search(this, args),
            RegExpBuiltin::Source => self.regexp_prototype_source(this),
            RegExpBuiltin::Split => self.regexp_symbol_split(this, args),
            RegExpBuiltin::Test => self.regexp_prototype_test(this, args),
            RegExpBuiltin::ToString => self.regexp_prototype_to_string(this),
            RegExpBuiltin::StringIteratorNext => self.regexp_string_iterator_next(this),
        }
    }

    /// The slots of `object` when it is a RegExp: [[OriginalSource]], [[OriginalFlags]] and the
    /// flags of its program, parsed from [[OriginalFlags]].
    pub(crate) fn regexp_slots(
        &mut self,
        object: ObjectRef,
    ) -> Completion<Option<(StringRef, StringRef, Flags)>> {
        Ok(match &self.engine_object(object)?.kind {
            ObjectKind::RegExp(regexp) => {
                Some((regexp.source, regexp.flags, regexp.program.flags()))
            }
            _ => None,
        })
    }

    /// `RegExp ( pattern, flags )` (`#sec-regexp-pattern-flags`); `function` is the active
    /// function object.
    fn regexp_constructor(
        &mut self,
        function: ObjectRef,
        args: &[Value],
        new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let pattern = argument(args, 0);
        let flags = argument(args, 1);
        // 1. Let patternIsRegExp be ? IsRegExp(pattern).
        let pattern_is_regexp = self.is_reg_exp(pattern)?;
        let new_target = match new_target {
            // 2. If NewTarget is undefined, then a. Let newTarget be the active function object.
            None => {
                // b. If patternIsRegExp is true and flags is undefined, then i. Let
                // patternConstructor be ? Get(pattern, "constructor"). ii. If
                // SameValue(newTarget, patternConstructor) is true, return pattern.
                if pattern_is_regexp
                    && matches!(flags, Value::Undefined)
                    && let Value::Object(object) = pattern
                {
                    let key = self.name_key("constructor")?;
                    let constructor = self.get(object, key)?;
                    if self.same_value(Value::Object(function), constructor)? {
                        return Ok(pattern);
                    }
                }
                function
            }
            // 3. Else, let newTarget be NewTarget.
            Some(new_target) => new_target,
        };
        let mark = self.kept.len();
        let made = self.regexp_constructor_source(new_target, pattern, flags, pattern_is_regexp);
        self.release(mark);
        made.map(Value::Object)
    }

    /// Steps 4 to 8 of `RegExp ( pattern, flags )`: the source and the flags, read by the slots
    /// of a RegExp or by Get of a RegExp-like object and kept, then RegExpAlloc and
    /// RegExpInitialize.
    fn regexp_constructor_source(
        &mut self,
        new_target: ObjectRef,
        pattern: Value,
        flags: Value,
        pattern_is_regexp: bool,
    ) -> Completion<ObjectRef> {
        let slots = match pattern {
            Value::Object(object) => self.regexp_slots(object)?,
            _ => None,
        };
        let (source, flags) = match (slots, pattern) {
            // 4. If pattern is an Object and pattern has a [[RegExpMatcher]] internal slot, then
            // a. Let P be pattern.[[OriginalSource]]. b. If flags is undefined, let F be
            // pattern.[[OriginalFlags]]. c. Else, let F be flags.
            (Some((source, original_flags, _)), _) => {
                let flags = match flags {
                    Value::Undefined => Value::String(original_flags),
                    flags => flags,
                };
                (Value::String(source), flags)
            }
            // 5. Else if patternIsRegExp is true, then a. Let P be ? Get(pattern, "source").
            // b. If flags is undefined, then i. Let F be ? Get(pattern, "flags"). c. Else, let F
            // be flags.
            (None, Value::Object(object)) if pattern_is_regexp => {
                let key = self.name_key("source")?;
                let source = self.get(object, key)?;
                self.keep(source);
                let flags = match flags {
                    Value::Undefined => {
                        let key = self.name_key("flags")?;
                        self.get(object, key)?
                    }
                    flags => flags,
                };
                self.keep(flags);
                (source, flags)
            }
            // 6. Else, a. Let P be pattern. b. Let F be flags.
            _ => (pattern, flags),
        };
        // 7. Let O be ? RegExpAlloc(newTarget): its prototype is read first, by
        // OrdinaryCreateFromConstructor.
        let prototype =
            self.get_prototype_from_constructor(new_target, Intrinsic::RegExpPrototype)?;
        // 8. Return ? RegExpInitialize(O, P, F).
        self.regexp_initialize(prototype, source, flags)
    }

    /// RegExpCreate ( P, F ) (`#sec-regexpcreate`): 1. Let obj be ! RegExpAlloc(%RegExp%), whose
    /// prototype is `%RegExp.prototype%` of the current realm (the "prototype" of `%RegExp%` is
    /// neither writable nor configurable). 2. Return ? RegExpInitialize(obj, P, F).
    pub(crate) fn regexp_create(&mut self, pattern: Value, flags: Value) -> Completion<ObjectRef> {
        let prototype = self.current_intrinsic(Intrinsic::RegExpPrototype)?;
        self.regexp_initialize(prototype, pattern, flags)
    }

    /// RegExpAlloc (`#sec-regexpalloc`) with the prototype `prototype`, read already, and then
    /// RegExpInitialize (`#sec-regexpinitialize`) of `pattern` and `flags`; the object is made
    /// once the program is. Every value it holds across a conversion is kept.
    fn regexp_initialize(
        &mut self,
        prototype: ObjectRef,
        pattern: Value,
        flags: Value,
    ) -> Completion<ObjectRef> {
        let mark = self.keep(Value::Object(prototype));
        let made = self.regexp_initialize_kept(prototype, pattern, flags);
        self.release(mark);
        made
    }

    /// The steps of [`Agent::regexp_initialize`] with the prototype kept.
    fn regexp_initialize_kept(
        &mut self,
        prototype: ObjectRef,
        pattern: Value,
        flags: Value,
    ) -> Completion<ObjectRef> {
        // 1. If pattern is undefined, let P be the empty String. 2. Else, let P be ?
        // ToString(pattern).
        let pattern = match pattern {
            Value::Undefined => self.new_string_str("")?,
            pattern => self.to_string(pattern)?,
        };
        self.keep(Value::String(pattern));
        // 3. If flags is undefined, let F be the empty String. 4. Else, let F be ?
        // ToString(flags).
        let flags = match flags {
            Value::Undefined => self.new_string_str("")?,
            flags => self.to_string(flags)?,
        };
        self.keep(Value::String(flags));
        // 5, 6. A code unit of F that is not a flag, or one that repeats, throws a SyntaxError;
        // ParsePattern's step 1, `u` with `v`, is checked with them. A String longer than the
        // flags there are is charged a step per code unit before it is read.
        let units = if self.string_length(flags)? > MOST_FLAGS {
            self.charged_units(flags)?
        } else {
            self.string_units(flags)?
        };
        let parsed = match Flags::parse(&units) {
            Ok(parsed) => parsed,
            Err(error) => {
                return Err(self.throw_error(ErrorKind::SyntaxError, &error.message()));
            }
        };
        // One step per code unit of the pattern, before ParsePattern (decision 7, "What one
        // step of the matcher is").
        let length = self.string_length(pattern)?;
        self.charge_steps(u64::try_from(length).unwrap_or(u64::MAX))?;
        // 7 to 16, 19 to 22. ParsePattern, then CompilePattern of the tree with the flags' RegExp
        // Record.
        let program = self.compile_regexp(pattern, parsed)?;
        // RegExpAlloc: OrdinaryCreateFromConstructor with the slots set by steps 17 to 22; the
        // program's charge and its box become the bytes the record owns.
        let owned = regexp_owned_bytes(&program);
        if self.heap.accounting.charge(program_box_bytes()).is_err() {
            let mut budget = self.regexp_budget();
            program.release(&mut budget);
            return Err(self.terminate(Termination::MemoryBudget));
        }
        let record = ObjectRecord::new(
            Some(prototype),
            ObjectKind::RegExp(RegExpObject {
                source: pattern,
                flags,
                program: Box::new(program),
            }),
        );
        let object = match self.heap.new_object_charged(record, owned) {
            Ok(object) => object,
            Err(_) => {
                self.heap.accounting.credit(owned);
                return Err(self.terminate(Termination::MemoryBudget));
            }
        };
        // RegExpAlloc, step 2: ! DefinePropertyOrThrow(obj, "lastIndex", PropertyDescriptor {
        // [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false }) of a new
        // object, with no safe point since it was stored.
        let key = self.name_key("lastIndex")?;
        self.add_property(
            object,
            key,
            Property::data(Value::Undefined, true, false, false),
        )?;
        // 23. Perform ? Set(obj, "lastIndex", +0𝔽, true).
        self.set(object, key, Value::Number(0.0), true)?;
        // 24. Return obj.
        Ok(object)
    }

    /// ParsePattern (`#sec-parsepattern`) of the code units of `pattern` with `flags` and the
    /// agent's `parser_nesting`, read in place, then CompilePattern (`#sec-compilepattern`):
    /// the program, or the `SyntaxError` of an error or a refusal (counted), or the end of the
    /// entry on a stop of the budget, with everything charged credited.
    fn compile_regexp(&mut self, pattern: StringRef, flags: Flags) -> Completion<Program> {
        let nesting = self.limits.parser_nesting;
        let limit = self.limits.steps_per_entry;
        let built = {
            let Agent {
                heap,
                steps,
                interrupt,
                ..
            } = self;
            let Heap {
                accounting,
                strings,
                ..
            } = heap;
            let mut budget = AgentBudget::new(accounting, steps, limit, interrupt);
            match strings.get(pattern.handle) {
                Ok(HeapString::Utf16(units)) => {
                    parse_and_compile(units, flags, nesting, &mut budget)
                }
                Ok(HeapString::Latin1(bytes)) => {
                    parse_and_compile_latin1(bytes, flags, nesting, &mut budget)
                }
                Err(_) => Err(Failure::Stale),
            }
        };
        match built {
            Ok(program) => Ok(program),
            Err(Failure::Stale) => Err(self.fault()),
            Err(Failure::Stopped(stop)) => Err(self.regexp_stopped(stop)),
            Err(Failure::Pattern(error)) => {
                let message = error.message();
                if error.is_refusal() {
                    self.count_refusal(&message);
                }
                Err(self.throw_error(ErrorKind::SyntaxError, &message))
            }
            Err(Failure::Refused(error)) => {
                let message = error.message();
                self.count_refusal(&message);
                Err(self.throw_error(ErrorKind::SyntaxError, &message))
            }
        }
    }

    /// The TypeError of `method` called on a `this` that is not an Object.
    fn regexp_this_not_object(&mut self, method: &str) -> crate::host::outcome::Abrupt {
        self.throw_error(
            ErrorKind::TypeError,
            &format!("{method} requires that 'this' be an Object"),
        )
    }

    /// Whether `object` is `%RegExp.prototype%` of the current realm.
    fn is_regexp_prototype(&mut self, object: ObjectRef) -> Completion<bool> {
        let prototype = self.current_intrinsic(Intrinsic::RegExpPrototype)?;
        self.same_value(Value::Object(object), Value::Object(prototype))
    }

    /// The getter of `flag` (22.2.6.3 and the others), RegExpHasFlag (`#sec-regexphasflag`).
    fn regexp_has_flag(&mut self, this: Value, flag: FlagGetter) -> Completion<Value> {
        // 1. If R is not an Object, throw a TypeError exception.
        let Value::Object(object) = this else {
            let method = format!("RegExp.prototype.{}", flag.name());
            return Err(self.regexp_this_not_object(&method));
        };
        match self.regexp_slots(object)? {
            // 3. Let flags be R.[[OriginalFlags]]. 4. If flags contains codeUnit, return true.
            // 5. Return false.
            Some((_, _, flags)) => Ok(Value::Boolean(flag.reads(flags))),
            // 2. If R does not have an [[OriginalFlags]] internal slot, then a. If SameValue(R,
            // %RegExp.prototype%) is true, return undefined. b. Otherwise, throw a TypeError
            // exception.
            None if self.is_regexp_prototype(object)? => Ok(Value::Undefined),
            None => Err(self.throw_error(
                ErrorKind::TypeError,
                &format!(
                    "RegExp.prototype.{} requires that 'this' be a RegExp object",
                    flag.name()
                ),
            )),
        }
    }

    /// `get RegExp.prototype.flags` (`#sec-get-regexp.prototype.flags`).
    fn regexp_prototype_flags(&mut self, this: Value) -> Completion<Value> {
        // 1. Let R be the this value. 2. If R is not an Object, throw a TypeError exception.
        let Value::Object(object) = this else {
            return Err(self.regexp_this_not_object("RegExp.prototype.flags"));
        };
        // 3. Let codeUnits be a new empty List. 4 to 19. Each flag read by ToBoolean(? Get(R,
        // name)), in the order d g i m s u v y, its code unit appended when true.
        let mut units = [0u16; FLAG_PROPERTIES.len()];
        let mut count = 0;
        for (name, letter) in FLAG_PROPERTIES {
            let key = self.name_key(name)?;
            let value = self.get(object, key)?;
            if self.to_boolean(value)?
                && let Some(unit) = units.get_mut(count)
            {
                *unit = u16::from(letter);
                count += 1;
            }
        }
        // 20. Return the String value whose code units are the elements of codeUnits.
        let units = units.get(..count).unwrap_or_default();
        Ok(Value::String(self.new_string_value(units)?))
    }

    /// `get RegExp.prototype.source` (`#sec-get-regexp.prototype.source`).
    fn regexp_prototype_source(&mut self, this: Value) -> Completion<Value> {
        // 1. Let R be the this value. 2. If R is not an Object, throw a TypeError exception.
        let Value::Object(object) = this else {
            return Err(self.regexp_this_not_object("RegExp.prototype.source"));
        };
        let Some((source, _, _)) = self.regexp_slots(object)? else {
            // 3. If R does not have an [[OriginalSource]] internal slot, then a. If SameValue(R,
            // %RegExp.prototype%) is true, return "(?:)". b. Otherwise, throw a TypeError
            // exception.
            if self.is_regexp_prototype(object)? {
                return Ok(Value::String(self.new_string_str("(?:)")?));
            }
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "RegExp.prototype.source requires that 'this' be a RegExp object",
            ));
        };
        // 4. Assert: R has an [[OriginalFlags]] internal slot. 5. Let src be
        // R.[[OriginalSource]]. 6. Let flags be R.[[OriginalFlags]]. 7. Return
        // EscapeRegExpPattern(src, flags).
        Ok(Value::String(self.escape_regexp_pattern(source)?))
    }

    /// EscapeRegExpPattern ( P, F ) (`#sec-escaperegexppattern`): `P` with each `/` outside a
    /// class and each line terminator escaped, so that `/`, the result, `/` and the flags parse
    /// as a RegularExpressionLiteral of the same pattern in each mode; `(?:)` for the empty
    /// pattern. A line terminator after a `\` becomes the letter of its escape. The code units
    /// are read with a step each and the result is built by a [`StringBuilder`].
    fn escape_regexp_pattern(&mut self, pattern: StringRef) -> Completion<StringRef> {
        let units = self.charged_units(pattern)?;
        if units.is_empty() {
            return self.new_string_str("(?:)");
        }
        let mut builder = StringBuilder::new();
        let mut start = 0;
        let mut escaped = false;
        let mut in_class = false;
        for (index, &unit) in units.iter().enumerate() {
            let replacement = if escaped {
                escaped = false;
                match unit {
                    0x0A => Some("n"),
                    0x0D => Some("r"),
                    0x2028 => Some("u2028"),
                    0x2029 => Some("u2029"),
                    _ => None,
                }
            } else {
                match unit {
                    0x5C => {
                        escaped = true;
                        None
                    }
                    0x2F if !in_class => Some("\\/"),
                    0x5B => {
                        in_class = true;
                        None
                    }
                    0x5D => {
                        in_class = false;
                        None
                    }
                    0x0A => Some("\\n"),
                    0x0D => Some("\\r"),
                    0x2028 => Some("\\u2028"),
                    0x2029 => Some("\\u2029"),
                    _ => None,
                }
            };
            if let Some(text) = replacement {
                self.append_units(&mut builder, units.get(start..index).unwrap_or_default())?;
                self.append_str(&mut builder, text)?;
                start = index.saturating_add(1);
            }
        }
        self.append_units(&mut builder, units.get(start..).unwrap_or_default())?;
        self.finish_string(builder)
    }

    /// `RegExp.prototype.toString ( )` (`#sec-regexp.prototype.tostring`).
    fn regexp_prototype_to_string(&mut self, this: Value) -> Completion<Value> {
        // 1. Let R be the this value. 2. If R is not an Object, throw a TypeError exception.
        let Value::Object(object) = this else {
            return Err(self.regexp_this_not_object("RegExp.prototype.toString"));
        };
        let mark = self.kept.len();
        let made = self.regexp_to_string_kept(object);
        self.release(mark);
        made
    }

    /// Steps 3 to 5 of `RegExp.prototype.toString`, the pattern kept across the read of the
    /// flags.
    fn regexp_to_string_kept(&mut self, object: ObjectRef) -> Completion<Value> {
        // 3. Let pattern be ? ToString(? Get(R, "source")).
        let key = self.name_key("source")?;
        let source = self.get(object, key)?;
        let pattern = self.to_string(source)?;
        self.keep(Value::String(pattern));
        // 4. Let flags be ? ToString(? Get(R, "flags")).
        let key = self.name_key("flags")?;
        let flags = self.get(object, key)?;
        let flags = self.to_string(flags)?;
        self.keep(Value::String(flags));
        // 5. Let result be the string-concatenation of "/", pattern, "/", and flags. 6. Return
        // result.
        let mut builder = StringBuilder::new();
        self.append_str(&mut builder, "/")?;
        self.append_string(&mut builder, pattern)?;
        self.append_str(&mut builder, "/")?;
        self.append_string(&mut builder, flags)?;
        Ok(Value::String(self.finish_string(builder)?))
    }
}

/// An accessor property of `getter` whose set accessor function is undefined, { [[Enumerable]]:
/// false, [[Configurable]]: true } (clause 18).
fn getter_property(getter: ObjectRef) -> Property {
    Property {
        slot: Slot::Accessor {
            get: Some(getter),
            set: None,
        },
        enumerable: false,
        configurable: true,
    }
}
