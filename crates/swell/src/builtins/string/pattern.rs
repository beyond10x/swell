//! The methods of `String.prototype` that match a regular expression (ECMA-262 22.1.3,
//! `#sec-properties-of-the-string-prototype-object`), the group of
//! `story:regexp-symbol-methods-and-string-match-search`: `match`
//! (`#sec-string.prototype.match`) and `search` (`#sec-string.prototype.search`). The group
//! defines nothing on `%String%`.
//!
//! An Object given as the pattern is handed to its method keyed by `%Symbol.match%` or
//! `%Symbol.search%` when it has one, as the first steps of the clauses say; a primitive is never
//! looked up, so a method on its prototype is not read. Otherwise the this value is converted
//! by ToString, the pattern made a RegExp by RegExpCreate (`#sec-regexpcreate`) with undefined
//! flags, and the method of the RegExp keyed by the symbol invoked with the String
//! (`#sec-invoke`): `%RegExp.prototype%`'s, `builtins/regexp/symbol.rs`, unless a script replaced
//! it. RegExpCreate charges the pattern a step per code unit before it parses it; the method
//! keyed by the symbol is a call of the agent's call path, so each is a re-entry. The String
//! and the RegExp are kept while the method runs.

use crate::builtins::object::argument;
use crate::builtins::{Method, SymbolData};
use crate::host::agent::Agent;
use crate::host::outcome::Completion;
use crate::host::{ErrorKind, Host};
use crate::object::property::Key;
use crate::value::{ObjectRef, SymbolRef, Value};

/// The builtins of this group, by their steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PatternBuiltin {
    /// `String.prototype.match ( regexp )` (22.1.3.13).
    Match,
    /// `String.prototype.search ( regexp )` (22.1.3.21).
    Search,
}

/// The methods this group defines on `%String%`, in the order of their clauses: none.
pub(crate) const CONSTRUCTOR_METHODS: &[Method<PatternBuiltin>] = &[];

/// The methods this group defines on `%String.prototype%`, in the order of their clauses, each
/// with the "length" its clause heading gives (the required parameters, clause 18).
pub(crate) const PROTOTYPE_METHODS: &[Method<PatternBuiltin>] = &[
    ("match", 1, PatternBuiltin::Match),
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
}
