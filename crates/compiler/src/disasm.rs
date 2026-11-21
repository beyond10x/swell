//! A text form of a [`Code`], one instruction per line, for the tests and for reading.
//!
//! ```text
//! script locals <n>[ strict]
//!   var "<name>" ...                     the declaredVarNames, when there are any
//!   declare <function> ...               the functionsToInitialize, when there are any
//!   <index> <line>:<column> <instruction> <operands>
//!   catch|finally <start>..<end> -> <target> items <n> envs <n>
//!
//! function <i> declaration|expression|getter|setter "<name>" length <n> locals <n>[ strict][ self]
//!   params local <n>|env <n>, ...         when there are parameters
//!   captured "<name>" ...                 the function environment's slots, when there are any
//!   ...
//! ```
//!
//! A string constant is written in double quotes, with `"`, `\` and every code unit outside
//! printable ASCII escaped (`\u{XXXX}`); a Number by `Number::toString`. Instruction and jump
//! indices have four digits at least. The position `0:0` is the end of a script, which has no
//! source position.

use std::fmt::Write;

use swell_num::to_string::number_to_string;
use swell_syntax::ast::{FunctionKind, Span};

use crate::bytecode::{Code, Handler, HandlerKind, Instruction, Slot};

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
    body(
        &mut out,
        code,
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
        if f.self_binding {
            out.push_str(" self");
        }
        out.push('\n');
        if !f.params.is_empty() {
            let params: Vec<String> = f
                .params
                .iter()
                .map(|p| match p {
                    Slot::Local(n) => format!("local {n}"),
                    Slot::Env(n) => format!("env {n}"),
                })
                .collect();
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
        body(&mut out, code, &f.instructions, &f.positions, &f.handlers);
    }
    out
}

fn body(
    out: &mut String,
    code: &Code,
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
            text(code, instruction)
        );
    }
    for h in handlers {
        let kind = match h.kind {
            HandlerKind::Catch => "catch",
            HandlerKind::Finally => "finally",
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
fn text(code: &Code, instruction: &Instruction) -> String {
    use Instruction as I;
    let s = |k: u32| string(code, k);
    match *instruction {
        I::Number(k) => format!("Number {}", number(code, k)),
        I::String(k) => format!("String {}", s(k)),
        I::Closure(f) => format!("Closure {f}"),
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
        I::GetNamed(k) => format!("GetNamed {}", s(k)),
        I::PutNamed(k) => format!("PutNamed {}", s(k)),
        I::DeleteNamed(k) => format!("DeleteNamed {}", s(k)),
        I::DefineField(k) => format!("DefineField {}", s(k)),
        I::DefineGetter(k) => format!("DefineGetter {}", s(k)),
        I::DefineSetter(k) => format!("DefineSetter {}", s(k)),
        I::DefineIndex(n) => format!("DefineIndex {n}"),
        I::ArrayLength(n) => format!("ArrayLength {n}"),
        I::Call(n) => format!("Call {n}"),
        I::New(n) => format!("New {n}"),
        I::Jump(t) => format!("Jump {t:04}"),
        I::JumpIfTrue(t) => format!("JumpIfTrue {t:04}"),
        I::JumpIfFalse(t) => format!("JumpIfFalse {t:04}"),
        I::LoopJump(t) => format!("LoopJump {t:04}"),
        I::CaseJump(t) => format!("CaseJump {t:04}"),
        I::ForInNext(t) => format!("ForInNext {t:04}"),
        I::ThrowSyntaxError(k) => format!("ThrowSyntaxError {}", s(k)),
        I::PushEnv(n) => format!("PushEnv {n}"),
        I::Tag(n) => format!("Tag {n}"),
        I::JumpIfTag { tag, target } => format!("JumpIfTag {tag} {target:04}"),
        other => format!("{other:?}"),
    }
}
