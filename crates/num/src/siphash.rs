//! SipHash-2-4 (Aumasson and Bernstein, "SipHash: a fast short-input PRF", 2012), the keyed hash
//! of design decision 4: every hash map of the engine hashes with it, keyed by 16 bytes the host's
//! randomness gives at agent creation, so that a script cannot choose keys that collide, and no
//! hash map reads the operating system's random source as `std`'s `RandomState` does.
//!
//! The key is read as two little-endian 64-bit words, the message in little-endian 64-bit words
//! with the final word carrying the length modulo 256 in its top byte, as the reference
//! implementation does.

use std::hash::{BuildHasher, Hasher};

/// One SipHash-2-4 computation: [`Hasher::write`] the message in any number of pieces, then
/// [`Hasher::finish`].
#[derive(Clone)]
pub struct SipHasher24 {
    v0: u64,
    v1: u64,
    v2: u64,
    v3: u64,
    /// The bytes written since the last full word, in the low bytes.
    tail: u64,
    /// How many bytes `tail` holds, 0 to 7.
    tail_len: u32,
    /// How many bytes were written, modulo 2^64.
    length: u64,
}

/// The 16 bytes of `key` as the two words the algorithm keys with.
fn key_words(key: &[u8; 16]) -> (u64, u64) {
    let [
        a0,
        a1,
        a2,
        a3,
        a4,
        a5,
        a6,
        a7,
        b0,
        b1,
        b2,
        b3,
        b4,
        b5,
        b6,
        b7,
    ] = *key;
    (
        u64::from_le_bytes([a0, a1, a2, a3, a4, a5, a6, a7]),
        u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]),
    )
}

impl SipHasher24 {
    /// A hasher keyed by `key`.
    pub fn new(key: &[u8; 16]) -> SipHasher24 {
        let (k0, k1) = key_words(key);
        SipHasher24 {
            v0: k0 ^ 0x736f_6d65_7073_6575,
            v1: k1 ^ 0x646f_7261_6e64_6f6d,
            v2: k0 ^ 0x6c79_6765_6e65_7261,
            v3: k1 ^ 0x7465_6462_7974_6573,
            tail: 0,
            tail_len: 0,
            length: 0,
        }
    }

    fn round(&mut self) {
        self.v0 = self.v0.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(13);
        self.v1 ^= self.v0;
        self.v0 = self.v0.rotate_left(32);
        self.v2 = self.v2.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(16);
        self.v3 ^= self.v2;
        self.v0 = self.v0.wrapping_add(self.v3);
        self.v3 = self.v3.rotate_left(21);
        self.v3 ^= self.v0;
        self.v2 = self.v2.wrapping_add(self.v1);
        self.v1 = self.v1.rotate_left(17);
        self.v1 ^= self.v2;
        self.v2 = self.v2.rotate_left(32);
    }

    /// The compression of one message word: two rounds.
    fn compress(&mut self, word: u64) {
        self.v3 ^= word;
        self.round();
        self.round();
        self.v0 ^= word;
    }
}

impl std::fmt::Debug for SipHasher24 {
    /// Never prints the state, which is derived from the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SipHasher24")
    }
}

impl Hasher for SipHasher24 {
    fn write(&mut self, bytes: &[u8]) {
        self.length = self.length.wrapping_add(bytes.len() as u64);
        let mut rest = bytes;
        // Fill the pending word first.
        while self.tail_len != 0 {
            let Some((&byte, after)) = rest.split_first() else {
                return;
            };
            self.tail |= u64::from(byte) << (8 * self.tail_len);
            self.tail_len += 1;
            rest = after;
            if self.tail_len == 8 {
                let word = self.tail;
                self.compress(word);
                self.tail = 0;
                self.tail_len = 0;
            }
        }
        let (words, remainder) = rest.as_chunks::<8>();
        for word in words {
            self.compress(u64::from_le_bytes(*word));
        }
        for (i, &byte) in remainder.iter().enumerate() {
            self.tail |= u64::from(byte) << (8 * i);
        }
        self.tail_len = remainder.len() as u32;
    }

    fn finish(&self) -> u64 {
        let mut state = self.clone();
        let last = state.tail | ((state.length & 0xff) << 56);
        state.compress(last);
        state.v2 ^= 0xff;
        for _ in 0..4 {
            state.round();
        }
        state.v0 ^ state.v1 ^ state.v2 ^ state.v3
    }
}

/// SipHash-2-4 of `message` under `key`, in one call.
pub fn siphash24(key: &[u8; 16], message: &[u8]) -> u64 {
    let mut hasher = SipHasher24::new(key);
    hasher.write(message);
    hasher.finish()
}

/// The [`BuildHasher`] for the engine's hash maps: every hasher it builds has the same key.
#[derive(Clone, Copy)]
pub struct SipBuildHasher {
    key: [u8; 16],
}

impl SipBuildHasher {
    /// A builder whose hashers are keyed by `key`.
    pub fn new(key: &[u8; 16]) -> SipBuildHasher {
        SipBuildHasher { key: *key }
    }
}

impl std::fmt::Debug for SipBuildHasher {
    /// Never prints the key.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SipBuildHasher")
    }
}

impl BuildHasher for SipBuildHasher {
    type Hasher = SipHasher24;

    fn build_hasher(&self) -> SipHasher24 {
        SipHasher24::new(&self.key)
    }
}
