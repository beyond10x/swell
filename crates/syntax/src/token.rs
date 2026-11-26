//! The tokens of the lexical grammar (`#sec-ecmascript-language-lexical-grammar`) as the lexer
//! hands them to the parser: an `IdentifierName` that is no `ReservedWord`, a `ReservedWord`
//! (`#sec-keywords-and-reserved-words`), a `Punctuator` (`#sec-punctuators`), one of the
//! literals of `#sec-ecmascript-language-lexical-grammar-literals` or a template token
//! (`#sec-template-literal-lexical-components`).

use crate::ast::{NumberLiteral, RegExpLiteral, StringLiteral, TemplateString};
use crate::ledger::HeapBytes;
use alloc::boxed::Box;
use alloc::string::String;

/// A token with its source position: `start` and `end` are code unit offsets, `line` and
/// `column` count from 1. `newline_before` says whether a `LineTerminator`, or a
/// `MultiLineComment` holding one, comes between the previous token and this one
/// (`#sec-rules-of-automatic-semicolon-insertion`). `escaped` says whether the token is an
/// `IdentifierName` written with a `\u` escape (`#sec-identifier-names`): such a name is never a
/// keyword (`#sec-keywords-and-reserved-words`, Note 1).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub start: u32,
    pub end: u32,
    pub line: u32,
    pub column: u32,
    pub newline_before: bool,
    pub escaped: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum TokenKind {
    Eof,
    /// An `IdentifierName` that is not a `ReservedWord` written without escapes, or any
    /// `IdentifierName` written with one (its StringValue, which may then be a reserved word);
    /// ASCII only: `let`, `yield`, `await`, `async`, `get`, `set`, `of`, `static` and the other
    /// contextual words are names.
    Name(String),
    Keyword(Keyword),
    Punct(Punct),
    Number(NumberLiteral),
    String(StringLiteral),
    /// Produced only when the parser asks for the `InputElementRegExp` goal.
    RegExp(RegExpLiteral),
    /// `` `...` `` (`#sec-template-literal-lexical-components`). The parts of a template token
    /// are boxed, so a token is no larger than before templates: the parser's frames hold
    /// tokens.
    NoSubstitutionTemplate(Box<TemplatePart>),
    /// `` `...${ ``.
    TemplateHead(Box<TemplatePart>),
    /// `}...${`, produced only when the parser asks for the `InputElementTemplateTail` goal at
    /// the `}` that ends a substitution.
    TemplateMiddle(Box<TemplatePart>),
    /// `` }...` ``, likewise.
    TemplateTail(Box<TemplatePart>),
}

impl HeapBytes for TokenKind {
    /// What the token's payload owns, which the parser credits when it drops the token and
    /// does not when it moved the payload into the tree.
    fn heap_bytes(&self) -> usize {
        match self {
            TokenKind::Eof | TokenKind::Keyword(_) | TokenKind::Punct(_) => 0,
            TokenKind::Name(name) => name.heap_bytes(),
            TokenKind::Number(n) => n.heap_bytes(),
            TokenKind::String(s) => s.heap_bytes(),
            TokenKind::RegExp(r) => r.heap_bytes(),
            TokenKind::NoSubstitutionTemplate(part)
            | TokenKind::TemplateHead(part)
            | TokenKind::TemplateMiddle(part)
            | TokenKind::TemplateTail(part) => {
                size_of::<TemplatePart>().saturating_add(part.string.heap_bytes())
            }
        }
    }
}

/// The characters of a template token: its TV and TRV, and where its first NotEscapeSequence
/// starts (line and column of its `\`), if it has one: then its TV is undefined
/// (`#sec-static-semantics-tv`), which only a tagged template may have.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TemplatePart {
    pub string: TemplateString,
    pub not_escape: Option<(u32, u32)>,
}

/// The `ReservedWord`s but `await` and `yield`, which the grammar allows as identifiers in
/// scripts and non-generator code (`#sec-keywords-and-reserved-words`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyword {
    Break,
    Case,
    Catch,
    Class,
    Const,
    Continue,
    Debugger,
    Default,
    Delete,
    Do,
    Else,
    Enum,
    Export,
    Extends,
    False,
    Finally,
    For,
    Function,
    If,
    Import,
    In,
    Instanceof,
    New,
    Null,
    Return,
    Super,
    Switch,
    This,
    Throw,
    True,
    Try,
    Typeof,
    Var,
    Void,
    While,
    With,
}

impl Keyword {
    pub(crate) fn from_name(name: &str) -> Option<Keyword> {
        Some(match name {
            "break" => Keyword::Break,
            "case" => Keyword::Case,
            "catch" => Keyword::Catch,
            "class" => Keyword::Class,
            "const" => Keyword::Const,
            "continue" => Keyword::Continue,
            "debugger" => Keyword::Debugger,
            "default" => Keyword::Default,
            "delete" => Keyword::Delete,
            "do" => Keyword::Do,
            "else" => Keyword::Else,
            "enum" => Keyword::Enum,
            "export" => Keyword::Export,
            "extends" => Keyword::Extends,
            "false" => Keyword::False,
            "finally" => Keyword::Finally,
            "for" => Keyword::For,
            "function" => Keyword::Function,
            "if" => Keyword::If,
            "import" => Keyword::Import,
            "in" => Keyword::In,
            "instanceof" => Keyword::Instanceof,
            "new" => Keyword::New,
            "null" => Keyword::Null,
            "return" => Keyword::Return,
            "super" => Keyword::Super,
            "switch" => Keyword::Switch,
            "this" => Keyword::This,
            "throw" => Keyword::Throw,
            "true" => Keyword::True,
            "try" => Keyword::Try,
            "typeof" => Keyword::Typeof,
            "var" => Keyword::Var,
            "void" => Keyword::Void,
            "while" => Keyword::While,
            "with" => Keyword::With,
            _ => return None,
        })
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Keyword::Break => "break",
            Keyword::Case => "case",
            Keyword::Catch => "catch",
            Keyword::Class => "class",
            Keyword::Const => "const",
            Keyword::Continue => "continue",
            Keyword::Debugger => "debugger",
            Keyword::Default => "default",
            Keyword::Delete => "delete",
            Keyword::Do => "do",
            Keyword::Else => "else",
            Keyword::Enum => "enum",
            Keyword::Export => "export",
            Keyword::Extends => "extends",
            Keyword::False => "false",
            Keyword::Finally => "finally",
            Keyword::For => "for",
            Keyword::Function => "function",
            Keyword::If => "if",
            Keyword::Import => "import",
            Keyword::In => "in",
            Keyword::Instanceof => "instanceof",
            Keyword::New => "new",
            Keyword::Null => "null",
            Keyword::Return => "return",
            Keyword::Super => "super",
            Keyword::Switch => "switch",
            Keyword::This => "this",
            Keyword::Throw => "throw",
            Keyword::True => "true",
            Keyword::Try => "try",
            Keyword::Typeof => "typeof",
            Keyword::Var => "var",
            Keyword::Void => "void",
            Keyword::While => "while",
            Keyword::With => "with",
        }
    }
}

/// Every `Punctuator`, `DivPunctuator` and `RightBracePunctuator` of `#sec-punctuators`. The
/// lexer reads the later ones too (`**`, `??`, `?.`, `=>`, `...`, the logical assignments), so the
/// parser can refuse them by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Punct {
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Dot,
    Ellipsis,
    Semicolon,
    Comma,
    Lt,
    Gt,
    LtEq,
    GtEq,
    EqEq,
    NotEq,
    EqEqEq,
    NotEqEq,
    Plus,
    Minus,
    Star,
    Percent,
    StarStar,
    PlusPlus,
    MinusMinus,
    Shl,
    Shr,
    UShr,
    Amp,
    Pipe,
    Caret,
    Bang,
    Tilde,
    AmpAmp,
    PipePipe,
    QuestionQuestion,
    Question,
    QuestionDot,
    Colon,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    PercentEq,
    StarStarEq,
    ShlEq,
    ShrEq,
    UShrEq,
    AmpEq,
    PipeEq,
    CaretEq,
    AmpAmpEq,
    PipePipeEq,
    QuestionQuestionEq,
    Arrow,
    Slash,
    SlashEq,
}

impl Punct {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Punct::LBrace => "{",
            Punct::RBrace => "}",
            Punct::LParen => "(",
            Punct::RParen => ")",
            Punct::LBracket => "[",
            Punct::RBracket => "]",
            Punct::Dot => ".",
            Punct::Ellipsis => "...",
            Punct::Semicolon => ";",
            Punct::Comma => ",",
            Punct::Lt => "<",
            Punct::Gt => ">",
            Punct::LtEq => "<=",
            Punct::GtEq => ">=",
            Punct::EqEq => "==",
            Punct::NotEq => "!=",
            Punct::EqEqEq => "===",
            Punct::NotEqEq => "!==",
            Punct::Plus => "+",
            Punct::Minus => "-",
            Punct::Star => "*",
            Punct::Percent => "%",
            Punct::StarStar => "**",
            Punct::PlusPlus => "++",
            Punct::MinusMinus => "--",
            Punct::Shl => "<<",
            Punct::Shr => ">>",
            Punct::UShr => ">>>",
            Punct::Amp => "&",
            Punct::Pipe => "|",
            Punct::Caret => "^",
            Punct::Bang => "!",
            Punct::Tilde => "~",
            Punct::AmpAmp => "&&",
            Punct::PipePipe => "||",
            Punct::QuestionQuestion => "??",
            Punct::Question => "?",
            Punct::QuestionDot => "?.",
            Punct::Colon => ":",
            Punct::Eq => "=",
            Punct::PlusEq => "+=",
            Punct::MinusEq => "-=",
            Punct::StarEq => "*=",
            Punct::PercentEq => "%=",
            Punct::StarStarEq => "**=",
            Punct::ShlEq => "<<=",
            Punct::ShrEq => ">>=",
            Punct::UShrEq => ">>>=",
            Punct::AmpEq => "&=",
            Punct::PipeEq => "|=",
            Punct::CaretEq => "^=",
            Punct::AmpAmpEq => "&&=",
            Punct::PipePipeEq => "||=",
            Punct::QuestionQuestionEq => "??=",
            Punct::Arrow => "=>",
            Punct::Slash => "/",
            Punct::SlashEq => "/=",
        }
    }
}
