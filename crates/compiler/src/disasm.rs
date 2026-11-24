//! A text form of a [`Code`], one instruction per line, for the tests and for reading.
//!
//! ```text
//! script locals <n>[ strict]
//!   var "<name>" ...                     the declaredVarNames, when there are any
//!   declare <function> ...               the functionsToInitialize, when there are any
//!   lexical let|const "<name>"[ local <n>|env <n>] ...   the top level's lexical names
//!   template <k> "<cooked>"|undefined ... raw "<raw>" ...   each tagged template site
//!   <index> <line>:<column> <instruction> <operands>
//!   catch|finally <start>..<end> -> <target> items <n> envs <n>
//!
//! function <i> <kind> "<name>" length <n> locals <n>[ strict][ self][ eval-environment]
//!         [ binds-parameters][ parameter-environment]
//!   params local <n>|env <n>, ...         when there are parameters
//!   captured "<name>" ...                 the function environment's slots, when there are any
//!   arguments mapped|unmapped local <n>|env <n>   the arguments object a call makes, if any
//!   ...
//! ```
//!
//! A function's `<kind>` is `declaration`, `expression`, `getter`, `setter`, `method` or `arrow`.
//!
//! A string constant is written in double quotes, with `"`, `\` and every code unit outside
//! printable ASCII escaped (`\u{XXXX}`); a Number by `Number::toString`. Instruction and jump
//! indices have four digits at least. The position `0:0` is the end of a script, which has no
//! source position. `PushNamedEnv <t>`, `PushLexicalEnv <t>` and `PushVarEnv <t>` are followed by
//! the names of the slots of their table entry, a `const` one after `const`.

use std::fmt::Write;

use swell_num::to_string::number_to_string;
use swell_syntax::ast::{FunctionKind, Span};

use crate::bytecode::{Code, Handler, HandlerKind, Instruction, Slot, SlotName};

/// The text form of `code`.
pub fn disassemble(code: &Code) -> String {
    let mut out = String::new();
    out.push_str("script locals ");
    out.push_str(&code.locals.to_string());
    if code.strict {
        out.push_str(" strict");
    }
    out.push('\n');
    if !code.var_names.is_empty() {
        out.push_str("  var");
        for k in &code.var_names {
            out.push(' ');
            out.push_str(&string(code, *k));
        }
        out.push('\n');
    }
    if !code.function_declarations.is_empty() {
        out.push_str("  declare");
        for f in &code.function_declarations {
            let _ = write!(out, " {f}");
        }
        out.push('\n');
    }
    if !code.lexical.is_empty() {
        out.push_str("  lexical");
        for l in &code.lexical {
            let kind = if l.constant { "const" } else { "let" };
            let _ = write!(out, " {kind} {}", string(code, l.name));
            if let Some(at) = l.slot {
                let _ = write!(out, " {}", slot(at));
            }
        }
        out.push('\n');
    }
    for (i, site) in code.template_sites.iter().enumerate() {
        let _ = write!(out, "  template {i}");
        for k in &site.cooked {
            match k {
                Some(k) => {
                    let _ = write!(out, " {}", string(code, *k));
                }
                None => out.push_str(" undefined"),
            }
        }
        out.push_str(" raw");
        for k in &site.raw {
            let _ = write!(out, " {}", string(code, *k));
        }
        out.push('\n');
    }
    body(
        &mut out,
        code,
        &code.scopes,
        &code.instructions,
        &code.positions,
        &code.handlers,
    );
    for (i, f) in code.functions.iter().enumerate() {
        let kind = match f.kind {
            FunctionKind::Declaration => "declaration",
            FunctionKind::Expression => "expression",
            FunctionKind::Getter => "getter",
            FunctionKind::Setter => "setter",
            FunctionKind::Method => "method",
            FunctionKind::Arrow => "arrow",
        };
        let _ = write!(
            out,
            "\nfunction {i} {kind} {} length {} locals {}",
            quote(&f.name),
            f.length,
            f.locals
        );
        if f.strict {
            out.push_str(" strict");
        }
        if f.generator {
            out.push_str(" generator");
        }
        if f.asynchronous {
            out.push_str(" async");
        }
        if f.self_binding {
            out.push_str(" self");
        }
        if f.eval_environment {
            out.push_str(" eval-environment");
        }
        if f.binds_parameters {
            out.push_str(" binds-parameters");
        }
        if f.parameter_environment {
            out.push_str(" parameter-environment");
        }
        out.push('\n');
        if !f.params.is_empty() {
            let params: Vec<String> = f.params.iter().map(|p| slot(*p)).collect();
            let _ = writeln!(out, "  params {}", params.join(", "));
        }
        if !f.captured.is_empty() {
            out.push_str("  captured");
            for k in &f.captured {
                out.push(' ');
                out.push_str(&string(code, *k));
            }
            out.push('\n');
        }
        if let Some(arguments) = f.arguments {
            let kind = if arguments.mapped {
                "mapped"
            } else {
                "unmapped"
            };
            let _ = writeln!(out, "  arguments {kind} {}", slot(arguments.slot));
        }
        body(
            &mut out,
            code,
            &f.scopes,
            &f.instructions,
            &f.positions,
            &f.handlers,
        );
    }
    out
}

/// Where a binding lives.
fn slot(slot: Slot) -> String {
    match slot {
        Slot::Local(n) => format!("local {n}"),
        Slot::Env(n) => format!("env {n}"),
    }
}

fn body(
    out: &mut String,
    code: &Code,
    scopes: &[Vec<SlotName>],
    instructions: &[Instruction],
    positions: &[Span],
    handlers: &[Handler],
) {
    for (i, instruction) in instructions.iter().enumerate() {
        let span = positions.get(i).copied().unwrap_or_default();
        let _ = writeln!(
            out,
            "  {i:04} {}:{} {}",
            span.line,
            span.column,
            text(code, scopes, instruction)
        );
    }
    for h in handlers {
        let kind = match h.kind {
            HandlerKind::Catch => "catch",
            HandlerKind::Finally => "finally",
            HandlerKind::Pattern => "pattern",
        };
        let _ = writeln!(
            out,
            "  {kind} {:04}..{:04} -> {:04} items {} envs {}",
            h.start, h.end, h.target, h.items, h.envs
        );
    }
}

/// A string constant, quoted.
fn string(code: &Code, k: u32) -> String {
    match code.strings.get(k as usize) {
        Some(units) => quote(units),
        None => format!("<no string {k}>"),
    }
}

fn quote(units: &[u16]) -> String {
    let mut s = String::from("\"");
    for &u in units {
        match u {
            0x22 => s.push_str("\\\""),
            0x5C => s.push_str("\\\\"),
            0x20..=0x7E => s.push(char::from(u as u8)),
            _ => {
                let _ = write!(s, "\\u{{{u:04X}}}");
            }
        }
    }
    s.push('"');
    s
}

fn number(code: &Code, k: u32) -> String {
    match code.numbers.get(k as usize) {
        Some(n) => number_to_string(*n),
        None => format!("<no number {k}>"),
    }
}

/// One instruction with its operands.
fn text(code: &Code, scopes: &[Vec<SlotName>], instruction: &Instruction) -> String {
    use Instruction as I;
    let s = |k: u32| string(code, k);
    // The names of the slots of a table entry, `const` ones marked.
    let names = |t: u32| -> String {
        let names: Vec<String> = scopes
            .get(t as usize)
            .map(|names| {
                names
                    .iter()
                    .map(|n| {
                        let mark = if n.is_constant() { "const " } else { "" };
                        format!("{mark}{}", s(n.name()))
                    })
                    .collect()
            })
            .unwrap_or_default();
        names.join(" ")
    };
    match *instruction {
        I::Number(k) => format!("Number {}", number(code, k)),
        I::String(k) => format!("String {}", s(k)),
        I::Closure(f) => format!("Closure {f}"),
        I::ArrowClosure(f) => format!("ArrowClosure {f}"),
        I::Bury(n) => format!("Bury {n}"),
        I::Nip(n) => format!("Nip {n}"),
        I::GetLocal(n) => format!("GetLocal {n}"),
        I::SetLocal(n) => format!("SetLocal {n}"),
        I::GetEnv { hops, slot } => format!("GetEnv {hops} {slot}"),
        I::SetEnv { hops, slot } => format!("SetEnv {hops} {slot}"),
        I::AssignImmutable(k) => format!("AssignImmutable {}", s(k)),
        I::GetGlobal(k) => format!("GetGlobal {}", s(k)),
        I::ResolveGlobal(k) => format!("ResolveGlobal {}", s(k)),
        I::GetGlobalRef(k) => format!("GetGlobalRef {}", s(k)),
        I::PutGlobalRef(k) => format!("PutGlobalRef {}", s(k)),
        I::TypeofGlobal(k) => format!("TypeofGlobal {}", s(k)),
        I::DeleteGlobal(k) => format!("DeleteGlobal {}", s(k)),
        I::GetName(k) => format!("GetName {}", s(k)),
        I::ResolveName(k) => format!("ResolveName {}", s(k)),
        I::GetNameRef(k) => format!("GetNameRef {}", s(k)),
        I::PutNameRef(k) => format!("PutNameRef {}", s(k)),
        I::TypeofName(k) => format!("TypeofName {}", s(k)),
        I::DeleteName(k) => format!("DeleteName {}", s(k)),
        I::GetNameForCall(k) => format!("GetNameForCall {}", s(k)),
        I::GetNamed(k) => format!("GetNamed {}", s(k)),
        I::PutNamed(k) => format!("PutNamed {}", s(k)),
        I::DeleteNamed(k) => format!("DeleteNamed {}", s(k)),
        I::DefineField(k) => format!("DefineField {}", s(k)),
        I::DefineGetter(k) => format!("DefineGetter {}", s(k)),
        I::DefineSetter(k) => format!("DefineSetter {}", s(k)),
        I::DefineMethod(k) => format!("DefineMethod {}", s(k)),
        I::DefineIndex(n) => format!("DefineIndex {n}"),
        I::ArrayLength(n) => format!("ArrayLength {n}"),
        I::Call(n) => format!("Call {n}"),
        I::CallEval(n) => format!("CallEval {n}"),
        I::New(n) => format!("New {n}"),
        I::Jump(t) => format!("Jump {t:04}"),
        I::JumpIfTrue(t) => format!("JumpIfTrue {t:04}"),
        I::JumpIfFalse(t) => format!("JumpIfFalse {t:04}"),
        I::LoopJump(t) => format!("LoopJump {t:04}"),
        I::CaseJump(t) => format!("CaseJump {t:04}"),
        I::ForInNext(t) => format!("ForInNext {t:04}"),
        I::IteratorStep(t) => format!("IteratorStep {t:04}"),
        I::AsyncIteratorStep(t) => format!("AsyncIteratorStep {t:04}"),
        I::AsyncIteratorClose(t) => format!("AsyncIteratorClose {t:04}"),
        I::ThrowSyntaxError(k) => format!("ThrowSyntaxError {}", s(k)),
        I::NewRegExp { body, flags } => format!("NewRegExp {} {}", s(body), s(flags)),
        I::PushEnv(n) => format!("PushEnv {n}"),
        I::PushNamedEnv(t) => format!("PushNamedEnv {t} {}", names(t)),
        I::Tag(n) => format!("Tag {n}"),
        I::JumpIfTag { tag, target } => format!("JumpIfTag {tag} {target:04}"),
        I::Uninitialize(n) => format!("Uninitialize {n}"),
        I::PushLexicalEnv(t) => format!("PushLexicalEnv {t} {}", names(t)),
        I::GetLocalChecked { slot, name } => format!("GetLocalChecked {slot} {}", s(name)),
        I::SetLocalChecked { slot, name } => format!("SetLocalChecked {slot} {}", s(name)),
        I::GetEnvChecked { hops, slot } => format!("GetEnvChecked {hops} {slot}"),
        I::SetEnvChecked { hops, slot } => format!("SetEnvChecked {hops} {slot}"),
        I::AssignConstLocal { slot, name } => format!("AssignConstLocal {slot} {}", s(name)),
        I::AssignConstEnv { hops, slot } => format!("AssignConstEnv {hops} {slot}"),
        I::InitGlobal(k) => format!("InitGlobal {}", s(k)),
        I::ToString => "ToString".to_string(),
        I::Argument(n) => format!("Argument {n}"),
        I::JumpIfNotUndefined(t) => format!("JumpIfNotUndefined {t:04}"),
        I::RestArguments(n) => format!("RestArguments {n}"),
        I::PushVarEnv(t) => format!("PushVarEnv {t} {}", names(t)),
        I::PatternGet { depth, name } => format!("PatternGet {depth} {}", s(name)),
        I::PatternGetKey { depth, kept } => format!("PatternGetKey {depth} {kept}"),
        I::ObjectRest { depth, kept } => format!("ObjectRest {depth} {kept}"),
        I::ArrayGrow(n) => format!("ArrayGrow {n}"),
        I::PatternStep(n) => format!("PatternStep {n}"),
        I::PatternRest(n) => format!("PatternRest {n}"),
        I::TemplateObject(k) => format!("TemplateObject {k}"),
        I::GeneratorStart(n) => format!("GeneratorStart {n}"),
        I::Yield(n) => format!("Yield {n}"),
        I::YieldReturn(n) => format!("YieldReturn {n}"),
        I::GeneratorReturn(n) => format!("GeneratorReturn {n}"),
        I::GeneratorDone(n) => format!("GeneratorDone {n}"),
        I::DelegateStep(t) => format!("DelegateStep {t:04}"),
        I::JumpIfDelegateDone(t) => format!("JumpIfDelegateDone {t:04}"),
        I::DelegateYield(n) => format!("DelegateYield {n}"),
        I::AsyncStart(n) => format!("AsyncStart {n}"),
        I::Await(n) => format!("Await {n}"),
        I::AsyncReturn(n) => format!("AsyncReturn {n}"),
        I::AsyncReject(n) => format!("AsyncReject {n}"),
        other => format!("{other:?}"),
    }
}
