//! The methods of `String.prototype` that match a regular expression (ECMA-262 22.1.3,
//! `#sec-properties-of-the-string-prototype-object`), the group of
//! `story:regexp-symbol-methods-and-string-match-search`: `match`
//! (`#sec-string.prototype.match`) and `search` (`#sec-string.prototype.search`); and
//! `matchAll` (`#sec-string.prototype.matchall`), `story:regexp-match-all-and-escape`. The group
//! defines nothing on `%String%`.
//!
//! An Object given as the pattern is handed to its method keyed by `%Symbol.match%`,
//! `%Symbol.matchAll%` or `%Symbol.search%` when it has one, as the first steps of the clauses
//! say; a primitive is never looked up, so a method on its prototype is not read. Before
//! `matchAll` reads the method of a RegExp-like Object (IsRegExp, `#sec-isregexp`), it reads its
//! `flags` by Get and throws a `TypeError` when they are undefined or null or their String holds
//! no "g", each code unit read with a step. Otherwise the this value is converted by ToString,
//! the pattern made a RegExp by RegExpCreate (`#sec-regexpcreate`) with undefined flags (with
//! "g" for `matchAll`), and the method of the RegExp keyed by the symbol invoked with the String
//! (`#sec-invoke`): `%RegExp.prototype%`'s, `builtins/regexp/symbol.rs` and
//! `builtins/regexp/match_all.rs`, unless a script replaced it. RegExpCreate charges the
//! pattern a step per code unit before it parses it; the method keyed by the symbol is a call of
//! the agent's call path, so each is a re-entry. The String, the flags and the RegExp are kept
//! while the method runs.

use crate::builtins::object::argument;
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::property::Key;
use crate::value::{ObjectRef, SymbolRef, Value};
use alloc::format;

/// The code unit 0x0067 (LATIN SMALL LETTER G).
const FLAG_G: u16 = 0x0067;

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternBuiltin {
    /// `String.prototype.match ( regexp )` (22.1.3.13).
    Match,
    /// `String.prototype.matchAll ( regexpOrPattern )` (22.1.3.14).
    MatchAll,
    /// `String.prototype.search ( regexp )` (22.1.3.21).
    Search,
}

/// The methods this group defines on `%String%`, in the order of their clauses: none.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<PatternBuiltin>] = &[];

/// The methods this group defines on `%String.prototype%`, in the order of their clauses, each
/// with the "length" its clause heading gives (the required parameters, clause 18).
pub(crate) const PROTOTYPE_METHODS: &[Method<PatternBuiltin>] = &[
    ("match", 1, PatternBuiltin::Match),
    ("matchAll", 1, PatternBuiltin::MatchAll),
    ("search", 1, PatternBuiltin::Search),
];

/// The data properties keyed by a well-known symbol this group defines on
/// `%String.prototype%`, after its methods: none.
pub(crate) fn prototype_symbol_data<H: Host>() -> [SymbolData<H>; 0] {
    []
}

impl<H: Host> Agent<H> {
    /// The steps of the builtins of this group; not inlined, so that the frame of
    /// `run_string_builtin` does not grow with its arms.
    #[inline(never)]
    pub(crate) fn run_string_pattern_builtin(
        &mut self,
        id: PatternBuiltin,
        _function: ObjectRef,
        this: Value,
        args: &[Value],
        _new_target: Option<ObjectRef>,
    ) -> Completion<Value> {
        let Some(symbols) = self.well_known() else {
            return Err(self.fault());
        };
        match id {
            PatternBuiltin::Match => {
                self.string_match_or_search(this, args, symbols.r#match, "String.prototype.match")
            }
            PatternBuiltin::MatchAll => self.string_match_all(this, args, symbols.match_all),
            PatternBuiltin::Search => {
                self.string_match_or_search(this, args, symbols.search, "String.prototype.search")
            }
        }
    }

    /// `String.prototype.match ( regexp )` (`#sec-string.prototype.match`) and
    /// `String.prototype.search ( regexp )` (`#sec-string.prototype.search`), whose steps are
    /// the same but for the symbol `symbol` (`%Symbol.match%` or `%Symbol.search%`).
    #[inline(never)]
    fn string_match_or_search(
        &mut self,
        this: Value,
        args: &[Value],
        symbol: SymbolRef,
        method: &str,
    ) -> Completion<Value> {
        let pattern = argument(args, 0);
        // 1. Let O be the this value. 2. Perform ? RequireObjectCoercible(O).
        if matches!(this, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                &format!("{method} called on undefined or null"),
            ));
        }
        // 3. If regexp is an Object, then a. Let matcher be ? GetMethod(regexp, symbol). b. If
        // matcher is not undefined, then i. Return ? Call(matcher, regexp, « O »).
        if let Value::Object(_) = pattern
            && let Some(matcher) = self.get_method(pattern, Key::Symbol(symbol))?
        {
            return self.call_value(matcher, pattern, &[this]);
        }
        // 4. Let S be ? ToString(O).
        let string = self.to_string(this)?;
        let mark = self.keep(Value::String(string));
        // 5. Let rx be ? RegExpCreate(regexp, undefined).
        let answer = self
            .regexp_create(pattern, Value::Undefined)
            .and_then(|rx| {
                self.keep(Value::Object(rx));
                // 6. Return ? Invoke(rx, symbol, « S »).
                self.invoke(
                    Value::Object(rx),
                    Key::Symbol(symbol),
                    &[Value::String(string)],
                )
            });
        self.release(mark);
        answer
    }

    /// `String.prototype.matchAll ( regexpOrPattern )` (`#sec-string.prototype.matchall`);
    /// `symbol` is `%Symbol.matchAll%`.
    #[inline(never)]
    fn string_match_all(
        &mut self,
        this: Value,
        args: &[Value],
        symbol: SymbolRef,
    ) -> Completion<Value> {
        let pattern = argument(args, 0);
        // 1. Let thisValue be the this value. 2. Perform ? RequireObjectCoercible(thisValue).
        if matches!(this, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "String.prototype.matchAll called on undefined or null",
            ));
        }
        // 3. If regexpOrPattern is an Object, then
        if let Value::Object(object) = pattern {
            // a. Let isRegExp be ? IsRegExp(regexpOrPattern). b. If isRegExp is true, then the
            // check of its flags.
            if self.is_reg_exp(pattern)? {
                self.match_all_require_global(object)?;
            }
            // c. Let matcher be ? GetMethod(regexpOrPattern, %Symbol.matchAll%). d. If matcher
            // is not undefined, then i. Return ? Call(matcher, regexpOrPattern, « thisValue »).
            if let Some(matcher) = self.get_method(pattern, Key::Symbol(symbol))? {
                return self.call_value(matcher, pattern, &[this]);
            }
        }
        // 4. Let string be ? ToString(thisValue).
        let string = self.to_string(this)?;
        let mark = self.keep(Value::String(string));
        // 5. Let regexp be ? RegExpCreate(regexpOrPattern, "g"): the flags are kept while the
        // pattern's ToString may run code.
        let answer = self.new_string_str("g").and_then(|flags| {
            self.keep(Value::String(flags));
            let rx = self.regexp_create(pattern, Value::String(flags))?;
            self.keep(Value::Object(rx));
            // 6. Return ? Invoke(regexp, %Symbol.matchAll%, « string »).
            self.invoke(
                Value::Object(rx),
                Key::Symbol(symbol),
                &[Value::String(string)],
            )
        });
        self.release(mark);
        answer
    }

    /// Step 3.b of `String.prototype.matchAll` for a RegExp-like `regexp`: "i. Let flags be ?
    /// Get(regexpOrPattern, "flags"). ii. Perform ? RequireObjectCoercible(flags). iii. If ?
    /// ToString(flags) does not contain "g", throw a TypeError exception." Each code unit of the
    /// String is read with a step.
    fn match_all_require_global(&mut self, regexp: ObjectRef) -> Completion<()> {
        let key = self.name_key("flags")?;
        let flags = self.get(regexp, key)?;
        if matches!(flags, Value::Undefined | Value::Null) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "String.prototype.matchAll requires that the flags of a RegExp be neither undefined nor null",
            ));
        }
        let mark = self.keep(flags);
        let flags = self.to_string(flags);
        self.release(mark);
        let flags = flags?;
        let length = self.string_length(flags)?;
        if !self.read_units(flags, 0, length)?.contains(&FLAG_G) {
            return Err(self.throw_error(
                ErrorKind::TypeError,
                "String.prototype.matchAll requires a RegExp with the flag g",
            ));
        }
        Ok(())
    }
}
