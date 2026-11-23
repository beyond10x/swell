//! The four normalization forms of UAX #15 (Unicode Normalization Forms): NFD, NFC, NFKD and
//! NFKC, which `String.prototype.normalize` makes (`#sec-string.prototype.normalize`) and by
//! whose NFC `localeCompare` orders two Strings (`docs/design/engine.md`, decision 5). A form is
//! made in two passes over a sequence of code points:
//!
//! 1. [`decompose`]: the full decomposition of each code point, canonical for NFD and NFC,
//!    compatibility for NFKD and NFKC (Unicode chapter 3, D68 and D65): the one-level
//!    decompositions of the tables, expanded by a loop over an explicit stack of
//!    `tables::decompositions::DEPTH` levels, so a decomposition costs no call per level; a
//!    Hangul syllable by the arithmetic of chapter 3, section 3.12.
//! 2. [`reorder_and_compose`]: the canonical ordering of each run of non-starters (D108 and
//!    D109: a stable sort by canonical combining class), and for NFC and NFKC the canonical
//!    composition (D117) by the primary composites of the tables and the Hangul arithmetic. The
//!    ordering moves nothing: a run is read in the order of its classes, once per class it
//!    holds, and each code point is handed on in that order.
//!
//! The caller owns the buffers and pays for them: every code point either pass writes goes
//! through [`Buffer::push`], which the caller charges before it writes, and a composite
//! replaces the first of its pair in place through [`FormBuffer::replace`]. Nothing here
//! allocates or panics, and nothing indexes unchecked. A lone surrogate is a code point with no
//! decomposition and the class 0, so it stays in place. Any `u32` may be given; a value above
//! U+10FFFF has no decomposition and the class 0.

use std::cmp::Ordering;
use std::convert::Infallible;

use crate::tables::{combining_class, compositions, decompositions};

/// A normalization form of UAX #15.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// Canonical decomposition, then canonical composition.
    Nfc,
    /// Canonical decomposition.
    Nfd,
    /// Compatibility decomposition, then canonical composition.
    Nfkc,
    /// Compatibility decomposition.
    Nfkd,
}

impl Form {
    /// Whether the form decomposes by the compatibility decompositions.
    fn compatibility(self) -> bool {
        matches!(self, Form::Nfkc | Form::Nfkd)
    }

    /// Whether the form composes after the ordering.
    fn composes(self) -> bool {
        matches!(self, Form::Nfc | Form::Nfkc)
    }
}

/// A buffer of the caller that takes the code points of a pass, one at a time.
pub trait Buffer {
    /// What the buffer answers when it does not take a code point (a budget the caller
    /// charges), which ends the pass.
    type Error;

    /// Append `point`.
    fn push(&mut self, point: u32) -> Result<(), Self::Error>;
}

/// The buffer of a form: [`Buffer`], and the replacement of a starter it holds by the composite
/// a later code point makes with it.
pub trait FormBuffer: Buffer {
    /// Where the next code point pushed will be, in the buffer's own units.
    fn position(&self) -> usize;

    /// Replace `old`, pushed at `position`, by `new`, a primary composite of `old` and a later
    /// code point; `new` has as many UTF-16 code units as `old` (the generator holds the tables
    /// to that), so a buffer of code units keeps its length.
    fn replace(&mut self, position: usize, old: u32, new: u32) -> Result<(), Self::Error>;
}

/// A buffer of code points that takes every one; it grows as a `Vec` grows, which no budget
/// bounds.
impl Buffer for Vec<u32> {
    type Error = Infallible;

    fn push(&mut self, point: u32) -> Result<(), Infallible> {
        Vec::push(self, point);
        Ok(())
    }
}

impl FormBuffer for Vec<u32> {
    fn position(&self) -> usize {
        self.len()
    }

    fn replace(&mut self, position: usize, _old: u32, new: u32) -> Result<(), Infallible> {
        if let Some(slot) = self.get_mut(position) {
            *slot = new;
        }
        Ok(())
    }
}

/// Why a pass stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop<E> {
    /// The caller's buffer did not take a code point.
    Buffer(E),
    /// A decomposition went deeper than the explicit stack holds. The generator writes `DEPTH`
    /// as the most levels any decomposition of the tables takes, so this is not reached.
    TooDeep,
}

/// The Hangul syllables and their jamo (Unicode chapter 3, section 3.12).
const S_BASE: u32 = 0xAC00;
const L_BASE: u32 = 0x1100;
const V_BASE: u32 = 0x1161;
const T_BASE: u32 = 0x11A7;
const L_COUNT: u32 = 19;
const V_COUNT: u32 = 21;
const T_COUNT: u32 = 28;
const N_COUNT: u32 = V_COUNT * T_COUNT;
const S_COUNT: u32 = L_COUNT * N_COUNT;

/// The jamo of the Hangul syllable `point` (its full canonical decomposition, chapter 3,
/// "Hangul Syllable Decomposition"): L and V, and T when there is one. `None` for any other
/// code point.
fn hangul_jamo(point: u32) -> Option<([u32; 3], usize)> {
    let index = point.checked_sub(S_BASE).filter(|index| *index < S_COUNT)?;
    let l = L_BASE + index / N_COUNT;
    let v = V_BASE + (index % N_COUNT) / T_COUNT;
    let t = index % T_COUNT;
    if t == 0 {
        Some(([l, v, 0], 2))
    } else {
        Some(([l, v, T_BASE + t], 3))
    }
}

/// The canonical combining class of `point` (field 3 of `UnicodeData.txt`); 0, a starter, for
/// every code point the table does not name, a lone surrogate and a value above U+10FFFF
/// among them.
pub fn canonical_combining_class(point: u32) -> u8 {
    let ranges = combining_class::COMBINING_CLASS;
    let at = ranges.partition_point(|&(_, last, _)| last < point);
    ranges
        .get(at)
        .filter(|&&(first, _, _)| first <= point)
        .map_or(0, |&(_, _, class)| class)
}

/// The code points of `key` in `mappings`, sorted by key.
fn row(mappings: &'static [(u32, &'static [u32])], key: u32) -> Option<&'static [u32]> {
    mappings
        .binary_search_by_key(&key, |&(k, _)| k)
        .ok()
        .and_then(|at| mappings.get(at))
        .map(|&(_, points)| points)
}

/// The decomposition of one level of `point` in the tables (field 5 of `UnicodeData.txt`):
/// for `compatibility` its compatibility decomposition where it has one, else its canonical
/// one. `None` where it has none; the Hangul syllables, decomposed by arithmetic, have none
/// here.
pub fn decomposition(point: u32, compatibility: bool) -> Option<&'static [u32]> {
    if compatibility && let Some(points) = row(decompositions::COMPATIBILITY, point) {
        return Some(points);
    }
    row(decompositions::CANONICAL, point)
}

/// The primary composite of `first` and `second` (D114): by the table of the canonical
/// decompositions of two code points not excluded from composition, or a Hangul syllable of
/// an L and a V jamo, or of a syllable with no T and a T jamo. `None` where there is none.
pub fn primary_composite(first: u32, second: u32) -> Option<u32> {
    if let (Some(l), Some(v)) = (
        first.checked_sub(L_BASE).filter(|l| *l < L_COUNT),
        second.checked_sub(V_BASE).filter(|v| *v < V_COUNT),
    ) {
        return Some(S_BASE + (l * V_COUNT + v) * T_COUNT);
    }
    if let (Some(_), Some(t)) = (
        first
            .checked_sub(S_BASE)
            .filter(|index| *index < S_COUNT && index % T_COUNT == 0),
        second
            .checked_sub(T_BASE)
            .filter(|t| (1..T_COUNT).contains(t)),
    ) {
        return Some(first + t);
    }
    let table = compositions::COMPOSITIONS;
    table
        .binary_search_by(|&(a, b, _)| match a.cmp(&first) {
            Ordering::Equal => b.cmp(&second),
            other => other,
        })
        .ok()
        .and_then(|at| table.get(at))
        .map(|&(_, _, composite)| composite)
}

/// Push the full decomposition of `point` into `out`, canonical or for `compatibility`: each
/// decomposition entered is a level of the explicit stack, read from its first code point; a
/// code point with no decomposition is pushed, a Hangul syllable as its jamo.
fn push_full<B: Buffer>(
    point: u32,
    compatibility: bool,
    out: &mut B,
) -> Result<(), Stop<B::Error>> {
    let empty: &'static [u32] = &[];
    // Each open level: its code points and the next one to read.
    let mut levels: [(&'static [u32], usize); decompositions::DEPTH] =
        [(empty, 0); decompositions::DEPTH];
    let mut open = 0;
    let mut pending = Some(point);
    loop {
        if let Some(point) = pending.take() {
            if let Some((jamo, count)) = hangul_jamo(point) {
                for &jamo in jamo.get(..count).unwrap_or(&[]) {
                    out.push(jamo).map_err(Stop::Buffer)?;
                }
            } else if let Some(points) = decomposition(point, compatibility) {
                let Some(level) = levels.get_mut(open) else {
                    return Err(Stop::TooDeep);
                };
                *level = (points, 0);
                open += 1;
            } else {
                out.push(point).map_err(Stop::Buffer)?;
            }
        }
        let Some(level) = open.checked_sub(1).and_then(|top| levels.get_mut(top)) else {
            return Ok(());
        };
        match level.0.get(level.1) {
            Some(&next) => {
                level.1 += 1;
                pending = Some(next);
            }
            None => open -= 1,
        }
    }
}

/// The first pass of `form`: the full decomposition of each code point of `input`, canonical
/// for NFD and NFC, compatibility for NFKD and NFKC, pushed into `out` in the order of
/// `input`, not yet ordered.
pub fn decompose<B: Buffer>(
    form: Form,
    input: impl IntoIterator<Item = u32>,
    out: &mut B,
) -> Result<(), Stop<B::Error>> {
    let compatibility = form.compatibility();
    for point in input {
        push_full(point, compatibility, out)?;
    }
    Ok(())
}

/// The canonical composition (D117) of the code points handed to it in canonical order: the
/// last starter pushed and where it is, and the class of the last code point pushed after it.
struct Composer {
    starter: Option<(usize, u32)>,
    last_class: Option<u8>,
}

impl Composer {
    /// Take `point` of class `class`: where a starter is before it and nothing between them
    /// blocks it (D115: a code point between them of class 0 or of a class not below its own),
    /// and the two have a primary composite, the composite replaces the starter and `point` is
    /// dropped; else `point` is pushed, and a starter is the last starter from then.
    fn take<B: FormBuffer>(&mut self, point: u32, class: u8, out: &mut B) -> Result<(), B::Error> {
        if let Some((at, starter)) = self.starter {
            let blocked = self
                .last_class
                .is_some_and(|last| last == 0 || last >= class);
            if !blocked && let Some(composite) = primary_composite(starter, point) {
                out.replace(at, starter, composite)?;
                self.starter = Some((at, composite));
                return Ok(());
            }
        }
        if class == 0 {
            self.starter = Some((out.position(), point));
            self.last_class = None;
        } else {
            self.last_class = Some(class);
        }
        out.push(point)
    }
}

/// Hand each code point of `points`, a full decomposition, on to `take` with its class, in
/// canonical order: a starter as it comes, and each run of non-starters by class, a class's
/// code points in their order (D109). A run already in order is read once; any other once per
/// class it holds, so a run costs at most as many reads per code point as there are classes.
fn in_canonical_order<E>(
    points: &[u32],
    mut take: impl FnMut(u32, u8) -> Result<(), E>,
) -> Result<(), E> {
    let mut start = 0;
    while let Some(&first) = points.get(start) {
        let class = canonical_combining_class(first);
        if class == 0 {
            take(first, 0)?;
            start += 1;
            continue;
        }
        // The run of non-starters from `start`, the classes it holds, and whether in order.
        let mut end = start;
        let mut previous = 0;
        let mut in_order = true;
        let mut held = [0_u64; 4];
        while let Some(&point) = points.get(end) {
            let class = canonical_combining_class(point);
            if class == 0 {
                break;
            }
            in_order &= class >= previous;
            previous = class;
            if let Some(word) = held.get_mut(usize::from(class / 64)) {
                *word |= 1 << (class % 64);
            }
            end += 1;
        }
        let run = points.get(start..end).unwrap_or(&[]);
        if in_order {
            for &point in run {
                take(point, canonical_combining_class(point))?;
            }
        } else {
            for (word_index, &word) in (0_u8..).zip(held.iter()) {
                let mut bits = word;
                while bits != 0 {
                    // Below 64, so the class fits a u8: word_index is below 4.
                    let bit = u8::try_from(bits.trailing_zeros()).unwrap_or(0);
                    bits &= bits - 1;
                    let wanted = word_index * 64 + bit;
                    for &point in run {
                        if canonical_combining_class(point) == wanted {
                            take(point, wanted)?;
                        }
                    }
                }
            }
        }
        start = end;
    }
    Ok(())
}

/// The second pass of `form` on `decomposition`, the first pass's code points: each run of
/// non-starters in canonical order, and for NFC and NFKC composed; pushed into `out`.
pub fn reorder_and_compose<B: FormBuffer>(
    form: Form,
    decomposition: &[u32],
    out: &mut B,
) -> Result<(), Stop<B::Error>> {
    if form.composes() {
        let mut composer = Composer {
            starter: None,
            last_class: None,
        };
        in_canonical_order(decomposition, |point, class| {
            composer.take(point, class, out)
        })
    } else {
        in_canonical_order(decomposition, |point, _| out.push(point))
    }
    .map_err(Stop::Buffer)
}

/// `input` in `form`: [`decompose`] into `decomposition`, then [`reorder_and_compose`] of what
/// it holds into `out`. The two buffers are the caller's; each code point either takes is
/// pushed.
pub fn normalize<D, O, E>(
    form: Form,
    input: impl IntoIterator<Item = u32>,
    decomposition: &mut D,
    out: &mut O,
) -> Result<(), Stop<E>>
where
    D: Buffer<Error = E> + AsRef<[u32]>,
    O: FormBuffer<Error = E>,
{
    decompose(form, input, decomposition)?;
    reorder_and_compose(form, decomposition.as_ref(), out)
}
