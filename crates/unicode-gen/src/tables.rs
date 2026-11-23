//! The renderers of the tables, by the name a line of `crates/unicode-gen/tables.txt` gives.
//! A renderer gets the sources of its line, in the order of the line, and returns the whole
//! text of its table file.

use crate::case::{self, Simple};
use crate::normalize;
use crate::render::{self, Source};
use crate::ucd::Ucd;

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
        "decompositions" => Some(decompositions),
        "combining_class" => Some(combining_class),
        "compositions" => Some(compositions),
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

/// The decompositions of one level of field 5 of `ucd/UnicodeData.txt`, as the constants
/// `DEPTH`, `CANONICAL` and `COMPATIBILITY`.
fn decompositions(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("decompositions", sources)?;
    let decompositions = normalize::decompositions(ucd, &source.path)?;
    let depth = decompositions.depth()?;
    let rows = |list: &[(u32, Vec<u32>)]| -> Vec<String> {
        list.iter()
            .map(|(key, points)| normalize::mapping_row(*key, points))
            .collect()
    };
    let mut text = render::head(sources);
    text.push_str(&format!(
        "//! The decompositions of field 5 of `UnicodeData.txt`, one level each. The full\n\
         //! decomposition of a code point, which the four forms of UAX #15 start from\n\
         //! (`#sec-string.prototype.normalize`), is expanded at run time by a loop over an\n\
         //! explicit stack of `DEPTH` levels. The Hangul syllables are decomposed by the\n\
         //! arithmetic of Unicode chapter 3, section 3.12, and are not written.\n\
         \n\
         /// The most levels a full decomposition takes, its compatibility decompositions\n\
         /// included: the depth of the stack that expands one.\n\
         pub(crate) const DEPTH: usize = {depth};\n\
         \n\
         /// The canonical decompositions: a code point and its one or two code points, sorted by\n\
         /// code point.\n"
    ));
    text.push_str(&normalize::constant(
        "CANONICAL",
        "(u32, &[u32])",
        &rows(&decompositions.canonical),
    ));
    text.push_str(
        "\n\
         /// The compatibility decompositions, those with a tag (`<compat>`, `<font>` and the\n\
         /// others, not written): a code point and its one to 18 code points, sorted by code\n\
         /// point. NFKD and NFKC take one where it exists, else the canonical one.\n",
    );
    text.push_str(&normalize::constant(
        "COMPATIBILITY",
        "(u32, &[u32])",
        &rows(&decompositions.compatibility),
    ));
    Ok(text)
}

/// The canonical combining classes other than 0 (field 3 of `ucd/UnicodeData.txt`), as the
/// constant `COMBINING_CLASS`.
fn combining_class(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("combining_class", sources)?;
    let ranges = normalize::combining_classes(ucd, &source.path)?;
    let rows: Vec<String> = ranges
        .iter()
        .map(|(first, last, class)| {
            format!(
                "    ({}, {}, {class}),\n",
                normalize::hex(*first),
                normalize::hex(*last)
            )
        })
        .collect();
    let mut text = render::head(sources);
    text.push_str(
        "//! The canonical combining classes of field 3 of `UnicodeData.txt`, which the canonical\n\
         //! ordering and the canonical composition of UAX #15 read\n\
         //! (`#sec-string.prototype.normalize`). A code point no range holds has the class 0: a\n\
         //! starter.\n\
         \n\
         /// The code points whose class is not 0: sorted, disjoint inclusive ranges\n\
         /// `(first, last, class)` of one class each.\n",
    );
    text.push_str(&normalize::constant(
        "COMBINING_CLASS",
        "(u32, u32, u8)",
        &rows,
    ));
    Ok(text)
}

/// The primary composites, from the canonical decompositions of `ucd/UnicodeData.txt`, the
/// exclusions of `ucd/CompositionExclusions.txt` and `Full_Composition_Exclusion` of
/// `ucd/DerivedNormalizationProps.txt`, as the constant `COMPOSITIONS`.
fn compositions(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let [unicode_data, exclusions, derived] = sources else {
        return Err(format!(
            "the renderer compositions reads three sources (UnicodeData.txt, \
             CompositionExclusions.txt, DerivedNormalizationProps.txt), not {}",
            sources.len()
        ));
    };
    let composites =
        normalize::compositions(ucd, &unicode_data.path, &exclusions.path, &derived.path)?;
    let rows: Vec<String> = composites
        .iter()
        .map(|(first, second, composite)| {
            format!(
                "    ({}, {}, {}),\n",
                normalize::hex(*first),
                normalize::hex(*second),
                normalize::hex(*composite)
            )
        })
        .collect();
    let mut text = render::head(sources);
    text.push_str(
        "//! The primary composites of UAX #15 (Unicode chapter 3, D114), which the canonical\n\
         //! composition of NFC and NFKC makes (`#sec-string.prototype.normalize`): every canonical\n\
         //! decomposition of two code points of `UnicodeData.txt` whose code point\n\
         //! `Full_Composition_Exclusion` does not name, that property being the exclusions of\n\
         //! `CompositionExclusions.txt`, the singletons and the non-starter decompositions. The\n\
         //! Hangul syllables are composed by the arithmetic of Unicode chapter 3, section 3.12,\n\
         //! and are not written.\n\
         \n\
         /// The pairs and their composites: `(first, second, composite)`, sorted by the pair; a\n\
         /// composite has as many UTF-16 code units as the first of its pair.\n",
    );
    text.push_str(&normalize::constant(
        "COMPOSITIONS",
        "(u32, u32, u32)",
        &rows,
    ));
    Ok(text)
}
