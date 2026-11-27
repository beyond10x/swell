//! The generator of `Math.random` (`docs/design/engine.md`, decision 6, "`Math.random`";
//! `#sec-math.random`): xoshiro256** (Blackman and Vigna, 2018) over four `u64` words, the
//! Number of a word, and the state of realm n derived from the agent's `random_seed` by
//! SipHash-2-4 (decision 4).
//!
//! Where the numbers come from: the three words from the state (1, 2, 3, 4) were worked by hand
//! from the step of the design on 2026-11-24 (the design's "What a test asserts"), and the ten
//! words from that state are the reference implementation's output, as `rand_xoshiro` 0.7.0
//! holds them in its test `reference`; the sixteen
//! derived words were printed by `from_seed` of this crate and are checked here against
//! `siphash24`, which `tests/siphash.rs` holds to the reference vectors, over the message the
//! design writes, built byte by byte in [`message`].

use std::time::Instant;

use swell_num::random::{Xoshiro256, fix_zero, word_to_number};
use swell_num::siphash::siphash24;

/// 2^-53, the weight of the lowest of the 53 bits a Number is made of.
const ULP: f64 = 1.0 / 9_007_199_254_740_992.0;

/// The design's step (decision 6, "`Math.random`", "What a test asserts"): from (1, 2, 3, 4)
/// the first three words are 11,520, 0 and 1,509,978,240, worked by hand, and the first ten
/// are the output of the reference implementation (`xoshiro256starstar.c`, Blackman and Vigna),
/// as the crate `rand_xoshiro` 0.7.0 holds them in its test `reference`
/// (`src/xoshiro256starstar.rs`). The three hand-worked words never read `s3` after its
/// rotation, so a wrong rotation constant passes them; the fourth word and after do not.
#[test]
fn the_first_ten_words_from_one_two_three_four() {
    const REFERENCE: [u64; 10] = [
        11_520,
        0,
        1_509_978_240,
        1_215_971_899_390_074_240,
        1_216_172_134_540_287_360,
        607_988_272_756_665_600,
        16_172_922_978_634_559_625,
        8_476_171_486_693_032_832,
        10_595_114_339_597_558_777,
        2_904_607_092_377_533_576,
    ];
    let mut generator = Xoshiro256::from_state([1, 2, 3, 4]);
    let words: Vec<u64> = (0..10).map(|_| generator.next_u64()).collect();
    assert_eq!(words, REFERENCE);
}

/// The Number of a word is its top 53 bits times 2^-53: 11,520 is 5 × 2^-53 (11,520 >> 11 is
/// 5), 0 is +0 and not -0, and the largest word is 1 - 2^-53, below 1. `next_f64` answers the
/// Number of the word `next_u64` would.
#[test]
fn the_number_of_a_word_is_its_top_53_bits_times_2_to_the_minus_53() {
    assert_eq!(ULP.to_bits(), 0x3ca0_0000_0000_0000);
    assert_eq!(word_to_number(11_520).to_bits(), (5.0 * ULP).to_bits());
    assert_eq!(word_to_number(0).to_bits(), 0.0_f64.to_bits());
    assert_eq!(word_to_number(u64::MAX).to_bits(), (1.0 - ULP).to_bits());
    assert!(word_to_number(u64::MAX) < 1.0);
    let mut words = Xoshiro256::from_state([1, 2, 3, 4]);
    let mut numbers = Xoshiro256::from_state([1, 2, 3, 4]);
    for _ in 0..3 {
        let word = words.next_u64();
        assert_eq!(numbers.next_f64().to_bits(), word_to_number(word).to_bits());
    }
}

/// The message of word `i` of realm `index`, as the design writes it: `random_seed[16..32]`,
/// then `index` as 8 bytes little-endian, then the byte `i`; 25 bytes.
fn message(seed: &[u8; 32], index: u64, i: u8) -> Vec<u8> {
    let mut message = seed[16..32].to_vec();
    message.extend_from_slice(&index.to_le_bytes());
    message.push(i);
    assert_eq!(message.len(), 25);
    message
}

/// The four words of realm `index` by the design's sentence, through `siphash24` alone.
fn by_hand(seed: &[u8; 32], index: u64) -> [u64; 4] {
    let mut key = [0_u8; 16];
    key.copy_from_slice(&seed[0..16]);
    [0, 1, 2, 3].map(|i| siphash24(&key, &message(seed, index, i)))
}

/// The derived states of the all-zero seed and the all-0xff seed at n = 0 and n = 1, as
/// `from_seed` printed them, each word equal to its SipHash-2-4 over the design's message.
#[test]
fn the_derived_states_of_two_seeds_at_index_0_and_1() {
    let table: [([u8; 32], u64, [u64; 4]); 4] = [
        ([0; 32], 0, DERIVED[0]),
        ([0; 32], 1, DERIVED[1]),
        ([0xff; 32], 0, DERIVED[2]),
        ([0xff; 32], 1, DERIVED[3]),
    ];
    let states = table.map(|(seed, index, _)| Xoshiro256::from_seed(&seed, index).state());
    for ((seed, index, _), state) in table.iter().zip(&states) {
        println!(
            "derived seed {:02x} index {index}: {:#018x} {:#018x} {:#018x} {:#018x}",
            seed[0], state[0], state[1], state[2], state[3]
        );
    }
    for ((seed, index, expected), state) in table.iter().zip(states) {
        assert_eq!(
            state,
            by_hand(seed, *index),
            "seed {:02x} n {index}",
            seed[0]
        );
        assert_eq!(state, *expected, "seed {:02x} n {index}", seed[0]);
    }
}

/// The sixteen words of [`the_derived_states_of_two_seeds_at_index_0_and_1`]: the all-zero seed
/// at n = 0 and 1, then the all-0xff seed at n = 0 and 1.
const DERIVED: [[u64; 4]; 4] = [
    [
        0x2ac0_288c_62cb_c269,
        0xa834_8008_445c_3936,
        0x20f1_0705_1b45_58bf,
        0xf5ba_f16d_6c06_1331,
    ],
    [
        0xb643_c7cb_95aa_3fe4,
        0x52ff_7caa_1671_8244,
        0xf53e_37ad_3fdb_c980,
        0xc19a_37dd_3111_9cc4,
    ],
    [
        0x9a75_16c8_d612_1372,
        0xa24c_2ef8_29f2_88fb,
        0x69e8_8625_2a30_0eb8,
        0xd524_d947_b9ea_342f,
    ],
    [
        0x891a_05a0_992f_283f,
        0x2edd_6fc2_826b_cd09,
        0x2427_5227_2e34_7fb3,
        0x10ab_405f_5dda_d644,
    ],
];

/// A state of four zero words would give zeros for ever, so `s0` becomes 1; no seed can be
/// built whose four derived words are zero, so the rule is held through `fix_zero`, which
/// `from_seed` and `from_state` call. A state with any word set is kept.
#[test]
fn an_all_zero_state_becomes_one_zero_zero_zero() {
    assert_eq!(fix_zero([0; 4]), [1, 0, 0, 0]);
    for state in [[0, 0, 0, 1], [0, 2, 0, 0], [5, 6, 7, 8], [u64::MAX; 4]] {
        assert_eq!(fix_zero(state), state);
    }
    let mut fixed = Xoshiro256::from_state([0; 4]);
    assert_eq!(fixed.state(), [1, 0, 0, 0]);
    assert!((0..8).any(|_| fixed.next_u64() != 0));
}

/// Two seeds that differ in one bit give different states, for each of the 256 bits, at n = 0;
/// and one seed gives different states at n = 0, 1 and 2.
#[test]
fn seeds_that_differ_in_one_bit_give_different_states() {
    let base: [u8; 32] = core::array::from_fn(|i| u8::try_from(i).unwrap().wrapping_mul(37));
    let state = Xoshiro256::from_seed(&base, 0).state();
    for byte in 0..32 {
        for bit in 0..8 {
            let mut seed = base;
            seed[byte] ^= 1 << bit;
            assert_ne!(
                Xoshiro256::from_seed(&seed, 0).state(),
                state,
                "byte {byte} bit {bit}"
            );
        }
    }
    let states = [0, 1, 2].map(|n| Xoshiro256::from_seed(&base, n).state());
    assert_ne!(states[0], states[1]);
    assert_ne!(states[0], states[2]);
    assert_ne!(states[1], states[2]);
}

/// A smoke test of the generator, not of its quality: over 100,000 words from one seed each of
/// the 64 bits is set in 49 % to 51 % of them.
#[test]
fn each_bit_is_set_in_49_to_51_percent_of_100000_words() {
    const WORDS: u32 = 100_000;
    let started = Instant::now();
    let mut generator = Xoshiro256::from_seed(&[0x5a; 32], 0);
    let mut set = [0_u32; 64];
    for _ in 0..WORDS {
        let word = generator.next_u64();
        for (bit, count) in set.iter_mut().enumerate() {
            *count += u32::from((word >> bit) & 1 == 1);
        }
    }
    for (bit, &count) in set.iter().enumerate() {
        assert!(
            (WORDS / 100 * 49..=WORDS / 100 * 51).contains(&count),
            "bit {bit} set in {count} of {WORDS} words"
        );
    }
    println!(
        "{WORDS} words, bits set {}..{} times, {} ms",
        set.iter().min().unwrap(),
        set.iter().max().unwrap(),
        started.elapsed().as_millis()
    );
}

/// The design's derivation (decision 6, "`Math.random`"): word i of realm n is SipHash-2-4 over
/// `random_seed[16..32]`, n as 8 bytes little-endian and the byte i. At indices whose high bytes
/// are set (255, 256, 2^16, 2^32, 2^56, 2^64 - 2 and 2^64 - 1) the state equals that message
/// built by hand, so a derivation that reads fewer than the 8 bytes of the index fails; the
/// table of `the_derived_states_of_two_seeds_at_index_0_and_1` holds only indices whose bytes
/// but the lowest are zero.
#[test]
fn the_derived_state_at_large_indices_is_the_designs_message() {
    let seed: [u8; 32] = core::array::from_fn(|i| u8::try_from(i).unwrap().wrapping_mul(29) ^ 0x5c);
    for index in [255, 256, 65_536, 1 << 32, 1 << 56, u64::MAX - 1, u64::MAX] {
        assert_eq!(
            Xoshiro256::from_seed(&seed, index).state(),
            by_hand(&seed, index),
            "index {index}"
        );
    }
    assert_ne!(
        Xoshiro256::from_seed(&seed, 256).state(),
        Xoshiro256::from_seed(&seed, 0).state()
    );
}

/// The design's Number of a word (decision 6, "`Math.random`": "a multiple of 2^-53 in
/// [0, 1 - 2^-53], with positive sign (0 gives +0)"), at the ends of the word range: a word below
/// 2^11 has no bit in its top 53 and is +0, never -0; 2^11 is 2^-53; a word whose top 53 bits are
/// all set is 1 - 2^-53, below 1.
#[test]
fn the_numbers_at_the_ends_of_the_word_range() {
    assert_eq!(word_to_number(2_047).to_bits(), 0);
    assert_eq!(word_to_number(2_048).to_bits(), ULP.to_bits());
    assert_eq!(
        word_to_number(u64::MAX - 2_047).to_bits(),
        (1.0 - ULP).to_bits()
    );
    assert!(word_to_number(u64::MAX - 2_047) < 1.0);
}

/// SipHash-2-4 of the Rust standard library (`std::hash::SipHasher`, 2 compression rounds and 4
/// finalization rounds, keyed by two words), an implementation independent of
/// `swell_num::siphash`: the key's 16 bytes as two little-endian words, as the reference
/// implementation reads them.
#[expect(
    deprecated,
    reason = "the standard library's SipHash-2-4 is the independent implementation the test compares with"
)]
fn std_siphash24(key: &[u8; 16], message: &[u8]) -> u64 {
    use std::hash::{Hasher, SipHasher};
    let mut k0 = [0_u8; 8];
    let mut k1 = [0_u8; 8];
    k0.copy_from_slice(&key[0..8]);
    k1.copy_from_slice(&key[8..16]);
    let mut hasher = SipHasher::new_with_keys(u64::from_le_bytes(k0), u64::from_le_bytes(k1));
    hasher.write(message);
    hasher.finish()
}

/// The four words of the design's sentence (decision 6, "`Math.random`"; HOST-213's section 3
/// of `host-interface.md`), computed with [`std_siphash24`] and nothing of this
/// crate: key `random_seed[0..16]`, message `random_seed[16..32]`, `index` as 8 bytes
/// little-endian, the byte i.
fn state_by_std_siphash(seed: &[u8; 32], index: u64) -> [u64; 4] {
    let mut key = [0_u8; 16];
    key.copy_from_slice(&seed[0..16]);
    [0_u8, 1, 2, 3].map(|i| {
        let mut message = [0_u8; 25];
        message[..16].copy_from_slice(&seed[16..32]);
        message[16..24].copy_from_slice(&index.to_le_bytes());
        message[24] = i;
        std_siphash24(&key, &message)
    })
}

/// The derivation against an independent SipHash-2-4. The oracle first
/// reproduces the published algorithm (the paper's example, key `00..0f` and message `00..0e`,
/// is `a129ca6149be45e5`; the empty message is `726fdb47dd0e0e31`, the first of the reference
/// vectors). Then `from_seed` equals the oracle for the all-zero seed, the all-0xff seed and the
/// seed `00..1f` (whose key is the reference key) at indices 0, 1, 0x0807060504030201 and
/// 2^64 - 1, and the sixteen committed words of `DERIVED` equal the oracle too.
#[test]
fn the_derived_states_agree_with_the_standard_librarys_siphash_2_4() {
    let reference_key: [u8; 16] = core::array::from_fn(|i| u8::try_from(i).unwrap());
    let fifteen: Vec<u8> = (0_u8..15).collect();
    assert_eq!(
        std_siphash24(&reference_key, &fifteen),
        0xa129_ca61_49be_45e5
    );
    assert_eq!(std_siphash24(&reference_key, &[]), 0x726f_db47_dd0e_0e31);
    let counting: [u8; 32] = core::array::from_fn(|i| u8::try_from(i).unwrap());
    for seed in [[0_u8; 32], [0xff; 32], counting] {
        for index in [0, 1, 0x0807_0605_0403_0201, u64::MAX] {
            assert_eq!(
                Xoshiro256::from_seed(&seed, index).state(),
                state_by_std_siphash(&seed, index),
                "seed {:02x}{:02x} index {index:#x}",
                seed[0],
                seed[1]
            );
        }
    }
    let table = [
        ([0_u8; 32], 0, DERIVED[0]),
        ([0; 32], 1, DERIVED[1]),
        ([0xff; 32], 0, DERIVED[2]),
        ([0xff; 32], 1, DERIVED[3]),
    ];
    for (seed, index, words) in table {
        assert_eq!(
            words,
            state_by_std_siphash(&seed, index),
            "DERIVED seed {:02x} n {index}",
            seed[0]
        );
    }
}
