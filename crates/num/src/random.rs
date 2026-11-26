//! The generator of `Math.random` (`#sec-math.random`; `docs/design/engine.md`, decision 6,
//! "`Math.random`"): xoshiro256** (Blackman and Vigna, "Scrambled Linear Pseudorandom Number
//! Generators", 2018), four `u64` words, never all zero.
//!
//! Each realm holds its own state, derived from the agent's `random_seed` (32 bytes, the host's
//! `Seeds`) and the realm's creation index n by SipHash-2-4 ([`crate::siphash`], decision 4):
//! word i, for i = 0 to 3, is `siphash24(key = random_seed[0..16], message = random_seed[16..32]
//! followed by n as 8 bytes little-endian followed by the byte i)`, read as the little-endian
//! `u64` the hash is. The state is a function of the seed and the index alone, and a step is
//! integer arithmetic whose result Rust fixes on every platform (`wrapping_mul`, `rotate_left`,
//! shifts and xor), so a stream is the same bytes everywhere.
//!
//! The Number of a word is its top 53 bits times 2^-53 ([`word_to_number`]): `as f64` of an
//! integer below 2^53 is exact, and a product by a power of two is exact, so every Number is a
//! multiple of 2^-53 in [0, 1 - 2^-53] with positive sign. No `f64` method is called.

use crate::siphash::siphash24;

/// 2^-53, the weight of the lowest of the 53 bits a Number of the generator has.
const TWO_TO_THE_MINUS_53: f64 = 1.0 / 9_007_199_254_740_992.0;

/// The bytes of a derivation message: 16 of the seed, 8 of the index, 1 of the word's number.
const MESSAGE_BYTES: usize = 25;

/// xoshiro256**: the state `s0..s3`, never all zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Xoshiro256 {
    s: [u64; 4],
}

/// A state of four zero words stays zero for ever: `s0` becomes 1 (the design's rule). A state
/// with any word set is answered unchanged.
pub fn fix_zero(state: [u64; 4]) -> [u64; 4] {
    if state == [0; 4] { [1, 0, 0, 0] } else { state }
}

/// The Number of `word`: its top 53 bits, `word >> 11`, as an integer below 2^53 (exact in
/// `f64`), times 2^-53 (exact): a multiple of 2^-53 in [0, 1 - 2^-53], +0 for a word below 2^11.
pub fn word_to_number(word: u64) -> f64 {
    (word >> 11) as f64 * TWO_TO_THE_MINUS_53
}

impl Xoshiro256 {
    /// The state of the realm of creation index `index` of an agent seeded with `random_seed`:
    /// the four SipHash-2-4 words of the module's documentation, through [`fix_zero`].
    pub fn from_seed(random_seed: &[u8; 32], index: u64) -> Xoshiro256 {
        // The key, random_seed[0..16], and the message's first 16 bytes, random_seed[16..32].
        let key: [u8; 16] = core::array::from_fn(|i| random_seed.get(i).copied().unwrap_or(0));
        let tail = random_seed.iter().skip(16);
        // The message on the stack: the 16 bytes, the index, then the byte i, set per word.
        let index_bytes = index.to_le_bytes();
        let mut message = [0_u8; MESSAGE_BYTES];
        for (slot, &byte) in message.iter_mut().zip(tail.chain(&index_bytes)) {
            *slot = byte;
        }
        let mut s = [0_u64; 4];
        for (i, word) in (0_u8..).zip(s.iter_mut()) {
            if let Some(last) = message.last_mut() {
                *last = i;
            }
            *word = siphash24(&key, &message);
        }
        Xoshiro256::from_state(s)
    }

    /// A generator at the state `state`, through [`fix_zero`].
    pub fn from_state(state: [u64; 4]) -> Xoshiro256 {
        Xoshiro256 { s: fix_zero(state) }
    }

    /// The four words of the state, `s0` first.
    pub fn state(&self) -> [u64; 4] {
        self.s
    }

    /// The next word: `rotl(s1 × 5, 7) × 9` (wrapping), then the step of the state.
    pub fn next_u64(&mut self) -> u64 {
        let [s0, s1, s2, s3] = self.s;
        let result = s1.wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = s1 << 17;
        let s2 = s2 ^ s0;
        let s3 = s3 ^ s1;
        let s1 = s1 ^ s2;
        let s0 = s0 ^ s3;
        let s2 = s2 ^ t;
        let s3 = s3.rotate_left(45);
        self.s = [s0, s1, s2, s3];
        result
    }

    /// The Number of the next word ([`word_to_number`]).
    pub fn next_f64(&mut self) -> f64 {
        word_to_number(self.next_u64())
    }
}
