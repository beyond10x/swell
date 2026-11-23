//! The flags of a pattern from their code units (`#sec-regexpinitialize`, steps 5 and 6: each of
//! `d g i m s u v y` at most once, no other code unit) and the one combination ParsePattern
//! refuses (`#sec-parsepattern`, step 1: `u` and `v` together).

use crate::error::{ErrorKind, PatternError};

/// The flags of a pattern, each at most once, never `u` with `v`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flags {
    bits: u8,
}

/// The flags in the order of `#sec-regexpinitialize`, step 5, each with its bit.
const LETTERS: [(u8, u8); 8] = [
    (b'd', 1 << 0),
    (b'g', 1 << 1),
    (b'i', 1 << 2),
    (b'm', 1 << 3),
    (b's', 1 << 4),
    (b'u', 1 << 5),
    (b'v', 1 << 6),
    (b'y', 1 << 7),
];

const HAS_INDICES: u8 = 1 << 0;
const GLOBAL: u8 = 1 << 1;
const IGNORE_CASE: u8 = 1 << 2;
const MULTILINE: u8 = 1 << 3;
const DOT_ALL: u8 = 1 << 4;
const UNICODE: u8 = 1 << 5;
const UNICODE_SETS: u8 = 1 << 6;
const STICKY: u8 = 1 << 7;

impl Flags {
    /// The flags of `units`, checked in the order of the clauses: an error at the first code
    /// unit that is not a flag, else at the first that repeats one, else at the second of `u`
    /// and `v` when both are there.
    pub fn parse(units: &[u16]) -> Result<Flags, PatternError> {
        let bit_of = |unit: u16| {
            LETTERS
                .iter()
                .find(|&&(letter, _)| u16::from(letter) == unit)
                .map(|&(_, bit)| bit)
        };
        if let Some(offset) = units.iter().position(|&unit| bit_of(unit).is_none()) {
            return Err(PatternError::new(ErrorKind::UnknownFlag, offset));
        }
        let mut bits = 0u8;
        for (offset, &unit) in units.iter().enumerate() {
            let bit = bit_of(unit).unwrap_or(0);
            if bits & bit != 0 {
                return Err(PatternError::new(ErrorKind::RepeatedFlag, offset));
            }
            bits |= bit;
        }
        if bits & (UNICODE | UNICODE_SETS) == UNICODE | UNICODE_SETS {
            let offset = units
                .iter()
                .rposition(|&unit| unit == u16::from(b'u') || unit == u16::from(b'v'))
                .unwrap_or(0);
            return Err(PatternError::new(ErrorKind::UnicodeWithSets, offset));
        }
        Ok(Flags { bits })
    }

    /// `d`: [[HasIndices]].
    pub fn has_indices(self) -> bool {
        self.bits & HAS_INDICES != 0
    }

    /// `g`: global.
    pub fn global(self) -> bool {
        self.bits & GLOBAL != 0
    }

    /// `i`: [[IgnoreCase]].
    pub fn ignore_case(self) -> bool {
        self.bits & IGNORE_CASE != 0
    }

    /// `m`: [[Multiline]].
    pub fn multiline(self) -> bool {
        self.bits & MULTILINE != 0
    }

    /// `s`: [[DotAll]].
    pub fn dot_all(self) -> bool {
        self.bits & DOT_ALL != 0
    }

    /// `u`: [[Unicode]].
    pub fn unicode(self) -> bool {
        self.bits & UNICODE != 0
    }

    /// `v`: [[UnicodeSets]].
    pub fn unicode_sets(self) -> bool {
        self.bits & UNICODE_SETS != 0
    }

    /// `y`: sticky.
    pub fn sticky(self) -> bool {
        self.bits & STICKY != 0
    }

    /// `u` or `v`: the pattern is read as code points, in the grammar with [UnicodeMode]
    /// (HasEitherUnicodeFlag, `#sec-haseitherunicodeflag`).
    pub fn either_unicode(self) -> bool {
        self.bits & (UNICODE | UNICODE_SETS) != 0
    }
}
