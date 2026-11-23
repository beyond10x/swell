//! The tables `swell-unicode-gen` writes, one module per table. Do not edit them by hand: change
//! the generator and run `cargo run -p swell-unicode-gen`.

pub(crate) mod case_ignorable;
pub(crate) mod cased;
pub(crate) mod lowercase;
pub(crate) mod special_casing;
pub(crate) mod uppercase;
pub(crate) mod zs;
