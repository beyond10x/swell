//! The tables `swell-unicode-gen` writes, one module per table. Do not edit them by hand: change
//! the generator and run `cargo run -p swell-unicode-gen`.

pub(crate) mod binary_properties;
pub(crate) mod case_folding;
pub(crate) mod case_ignorable;
pub(crate) mod cased;
pub(crate) mod combining_class;
pub(crate) mod compositions;
pub(crate) mod decompositions;
pub(crate) mod general_category;
pub(crate) mod lowercase;
pub(crate) mod property_names;
pub(crate) mod script;
pub(crate) mod script_extensions;
pub(crate) mod special_casing;
pub(crate) mod uppercase;
pub(crate) mod zs;
