//! The renderers of the tables, by the name a line of `crates/unicode-gen/tables.txt` gives.
//! A renderer gets the sources of its line, in the order of the line, and returns the whole
//! text of its table file.

// The case data the case renderers read; a module of the generator beside this file.
#[path = "case.rs"]
mod case;

use crate::render::{self, Source};
use crate::ucd::Ucd;
use case::Simple;

/// A renderer: the text of a table from its sources.
pub(crate) type Renderer = fn(&mut Ucd, &[Source]) -> Result<String, String>;

/// The renderer named `name`, if there is one.
pub(crate) fn renderer(name: &str) -> Option<Renderer> {
    match name {
        "zs" => Some(zs),
        "lowercase" => Some(lowercase),
        "uppercase" => Some(uppercase),
        "special_casing" => Some(special_casing),
        "cased" => Some(cased),
        "case_ignorable" => Some(case_ignorable),
        _ => None,
    }
}

/// The one source of the renderer `renderer`.
fn one_source<'a>(renderer: &str, sources: &'a [Source]) -> Result<&'a Source, String> {
    match sources {
        [source] => Ok(source),
        _ => Err(format!(
            "the renderer {renderer} reads one source, not {}",
            sources.len()
        )),
    }
}

/// General_Category `Zs` from `ucd/extracted/DerivedGeneralCategory.txt`, as the constant `ZS`.
fn zs(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("zs", sources)?;
    let ranges = ucd
        .ranges(&source.path, &["Zs"])?
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut text = render::head(sources);
    text.push_str(
        "//! General_Category `Space_Separator` (`Zs`), which `WhiteSpace` admits through `USP`\n\
         //! (`#sec-white-space`).\n\
         \n\
         /// The code points of `Zs`: sorted, disjoint inclusive ranges `(first, last)`.\n",
    );
    text.push_str(&render::ranges("ZS", &ranges));
    Ok(text)
}

/// `Simple_Lowercase_Mapping` (field 13 of `ucd/UnicodeData.txt`), as the constant `LOWERCASE`.
fn lowercase(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("lowercase", sources)?;
    let pairs = case::simple_mappings(ucd, &source.path, Simple::Lowercase)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! `Simple_Lowercase_Mapping`, field 13 of `UnicodeData.txt`: the mapping `toLowercase`\n\
         //! (`#sec-string.prototype.tolowercase`) takes where `SpecialCasing.txt` has none.\n\
         \n\
         /// Each code point with a simple lowercase mapping and that mapping, sorted by code point.\n",
    );
    text.push_str(&render::pairs("LOWERCASE", &pairs)?);
    Ok(text)
}

/// `Simple_Uppercase_Mapping` (field 12 of `ucd/UnicodeData.txt`), as the constant `UPPERCASE`.
fn uppercase(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("uppercase", sources)?;
    let pairs = case::simple_mappings(ucd, &source.path, Simple::Uppercase)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! `Simple_Uppercase_Mapping`, field 12 of `UnicodeData.txt`: the mapping `toUppercase`\n\
         //! (`#sec-string.prototype.touppercase`) takes where `SpecialCasing.txt` has none.\n\
         \n\
         /// Each code point with a simple uppercase mapping and that mapping, sorted by code point.\n",
    );
    text.push_str(&render::pairs("UPPERCASE", &pairs)?);
    Ok(text)
}

/// The full case mappings of `ucd/SpecialCasing.txt` that need no language, as the constants
/// `LOWERCASE`, `UPPERCASE` and `FINAL_SIGMA`.
fn special_casing(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("special_casing", sources)?;
    let special = case::special_casing(ucd, &source.path)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! The full case mappings of `SpecialCasing.txt` that need no language: the mappings\n\
         //! `toLowercase` and `toUppercase` take before the simple ones\n\
         //! (`#sec-string.prototype.tolowercase`). The lines of a language (`lt`, `tr`, `az`) are\n\
         //! not written: the engine has no host locale.\n\
         \n\
         /// The unconditional full lowercase mappings: a code point and its one to three code\n\
         /// points, sorted by code point.\n",
    );
    text.push_str(&render::mappings("LOWERCASE", &special.lowercase)?);
    text.push_str(
        "\n\
         /// The unconditional full uppercase mappings: a code point and its one to three code\n\
         /// points, sorted by code point.\n",
    );
    text.push_str(&render::mappings("UPPERCASE", &special.uppercase)?);
    text.push_str(
        "\n\
         /// The lowercase mappings where the context `Final_Sigma` holds (Unicode chapter 3,\n\
         /// table 3-17), sorted by code point.\n",
    );
    text.push_str(&render::mappings("FINAL_SIGMA", &special.final_sigma)?);
    Ok(text)
}

/// The property `Cased` of `ucd/DerivedCoreProperties.txt`, as the constant `CASED`.
fn cased(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("cased", sources)?;
    let ranges = ucd
        .ranges(&source.path, &["Cased"])?
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut text = render::head(sources);
    text.push_str(
        "//! The property `Cased` of `DerivedCoreProperties.txt`, which the context `Final_Sigma`\n\
         //! reads (Unicode chapter 3, table 3-17).\n\
         \n\
         /// The code points of `Cased`: sorted, disjoint inclusive ranges `(first, last)`.\n",
    );
    text.push_str(&render::ranges("CASED", &ranges));
    Ok(text)
}

/// The property `Case_Ignorable` of `ucd/DerivedCoreProperties.txt`, as the constant
/// `CASE_IGNORABLE`.
fn case_ignorable(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("case_ignorable", sources)?;
    let ranges = ucd
        .ranges(&source.path, &["Case_Ignorable"])?
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut text = render::head(sources);
    text.push_str(
        "//! The property `Case_Ignorable` of `DerivedCoreProperties.txt`, which the context\n\
         //! `Final_Sigma` reads (Unicode chapter 3, table 3-17).\n\
         \n\
         /// The code points of `Case_Ignorable`: sorted, disjoint inclusive ranges\n\
         /// `(first, last)`.\n",
    );
    text.push_str(&render::ranges("CASE_IGNORABLE", &ranges));
    Ok(text)
}
