//! The renderers of the tables, by the name a line of `crates/unicode-gen/tables.txt` gives.
//! A renderer gets the sources of its line, in the order of the line, and returns the whole
//! text of its table file.

use std::collections::BTreeMap;

use crate::case::{self, Simple};
use crate::emoji;
use crate::normalize;
use crate::property;
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
        "case_folding" => Some(case_folding),
        "cased" => Some(cased),
        "case_ignorable" => Some(case_ignorable),
        "decompositions" => Some(decompositions),
        "combining_class" => Some(combining_class),
        "compositions" => Some(compositions),
        "general_category" => Some(general_category),
        "script" => Some(script),
        "script_extensions" => Some(script_extensions),
        "binary_properties" => Some(binary_properties),
        "property_names" => Some(property_names),
        "properties_of_strings" => Some(properties_of_strings),
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

/// The simple case folding of `ucd/CaseFolding.txt` (status `C` and `S`), as the constants
/// `FOLDING`, `BY_FOLDING` and `LARGEST_GROUP`.
fn case_folding(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("case_folding", sources)?;
    let pairs = crate::case_folding::simple_case_folding(ucd, &source.path)?;
    let (grouped, largest) = crate::case_folding::by_folding(&pairs);
    let mut text = render::head(sources);
    text.push_str(
        "//! The simple case folding of `CaseFolding.txt`, its lines of status `C` and `S`, which\n\
         //! Canonicalize reads under the flags `u` and `v` with `i`\n\
         //! (`#sec-runtime-semantics-canonicalize-ch`). The lines of status `F` and `T` are not\n\
         //! written. A folding is its own folding and has as many UTF-16 code units as its code\n\
         //! point.\n\
         \n\
         /// Each code point with a simple case folding and that folding, sorted by code point.\n",
    );
    text.push_str(&render::pairs("FOLDING", &pairs)?);
    text.push_str(
        "\n\
         /// The pairs of `FOLDING` as `(folding, code point)`, sorted by folding and then by code\n\
         /// point: the code points of one folding next to each other, the folding not among them.\n",
    );
    let rows: Vec<String> = grouped
        .iter()
        .map(|(folding, point)| {
            format!(
                "    ({}, {}),\n",
                normalize::hex(*folding),
                normalize::hex(*point)
            )
        })
        .collect();
    text.push_str(&normalize::constant("BY_FOLDING", "(u32, u32)", &rows));
    text.push_str(&format!(
        "\n\
         /// The most code points of one folding, the folding itself counted.\n\
         pub(crate) const LARGEST_GROUP: usize = {largest};\n"
    ));
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

/// The values of General_Category of `ucd/extracted/DerivedGeneralCategory.txt`, one constant of
/// ranges each, and the constant `VALUES` of all of them in the order of the bits of a mask.
fn general_category(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let source = one_source("general_category", sources)?;
    let categories = property::categories(ucd, &source.path)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! The values of General_Category of `DerivedGeneralCategory.txt`, which the property\n\
         //! escapes read (`#sec-runtime-semantics-unicodematchpropertyvalue-p-v`). Every code point\n\
         //! has one of them, `Cn` where no other; the groups (`L`, `LC`, `M`, `N`, `P`, `S`, `Z`\n\
         //! and `C`) are unions of these, made at run time.\n",
    );
    for (name, ranges) in &categories {
        text.push_str(&format!(
            "\n/// The code points of `{name}`: sorted, disjoint inclusive ranges.\n"
        ));
        text.push_str(&property::ranges(&name.to_ascii_uppercase(), ranges));
    }
    let items: Vec<String> = categories
        .iter()
        .map(|(name, _)| name.to_ascii_uppercase())
        .collect();
    text.push_str(
        "\n/// Every value, sorted by its short name: bit `i` of a mask of values of\n\
         /// `property_names::CATEGORIES` is `VALUES[i]`.\n",
    );
    text.push_str(&property::short_items("VALUES", "&[(u32, u32)]", &items));
    Ok(text)
}

/// The values of Script of `ucd/Scripts.txt`, in the order of the `sc` lines of
/// `ucd/PropertyValueAliases.txt`, one constant of ranges each, and the constant `VALUES` of all
/// of them in that order.
fn script(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let [scripts, aliases] = sources else {
        return Err(format!(
            "the renderer script reads two sources (Scripts.txt, PropertyValueAliases.txt), not {}",
            sources.len()
        ));
    };
    let values = property::values(ucd, &aliases.path, "sc")?;
    let ranges = property::scripts(ucd, &scripts.path, &values)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! The values of Script of `Scripts.txt`, which the property escapes read\n\
         //! (`#sec-runtime-semantics-unicodematchpropertyvalue-p-v`), in the order of the lines of\n\
         //! Script in `PropertyValueAliases.txt`. `Zzzz` (`Unknown`), which `Scripts.txt` does not\n\
         //! list, is every code point it gives no script; `Hrkt` (`Katakana_Or_Hiragana`) has no\n\
         //! code point.\n",
    );
    for (value, ranges) in values.iter().zip(&ranges) {
        text.push_str(&format!(
            "\n/// The code points of `{}` (`{}`): sorted, disjoint inclusive ranges.\n",
            value.short(),
            value.long()
        ));
        text.push_str(&property::ranges(
            &value.short().to_ascii_uppercase(),
            ranges,
        ));
    }
    let items: Vec<String> = values
        .iter()
        .map(|value| value.short().to_ascii_uppercase())
        .collect();
    text.push_str(
        "\n/// Every value, in the order of `PropertyValueAliases.txt`: the number of a script in\n\
         /// `property_names::SCRIPTS` and `script_extensions::LISTS` is its index here.\n",
    );
    text.push_str(&property::short_items("VALUES", "&[(u32, u32)]", &items));
    Ok(text)
}

/// The sets of Script_Extensions of `ucd/ScriptExtensions.txt`, as the constants `RANGES` and
/// `LISTS`, a script by its number in the order of the `sc` lines of
/// `ucd/PropertyValueAliases.txt`.
fn script_extensions(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let [extensions, aliases] = sources else {
        return Err(format!(
            "the renderer script_extensions reads two sources (ScriptExtensions.txt, \
             PropertyValueAliases.txt), not {}",
            sources.len()
        ));
    };
    let values = property::values(ucd, &aliases.path, "sc")?;
    let rows = property::script_extensions(ucd, &extensions.path, &values)?;
    let mut lists: Vec<u8> = Vec::new();
    let mut starts: BTreeMap<Vec<u8>, usize> = BTreeMap::new();
    let mut lines = Vec::with_capacity(rows.len());
    for (first, last, list) in &rows {
        let start = match starts.get(list) {
            Some(start) => *start,
            None => {
                let start = lists.len();
                lists.extend_from_slice(list);
                starts.insert(list.clone(), start);
                start
            }
        };
        lines.push(format!(
            "    ({}, {}, {start}, {}),\n",
            normalize::hex(*first),
            normalize::hex(*last),
            list.len()
        ));
    }
    let mut text = render::head(sources);
    text.push_str(
        "//! The values of Script_Extensions of `ScriptExtensions.txt`, which the property escapes\n\
         //! read (`#sec-runtime-semantics-unicodematchpropertyvalue-p-v`). A code point this file\n\
         //! does not list has the extensions `{Script}`, its own script alone; the rule is applied\n\
         //! at run time.\n\
         \n\
         /// The code points the file lists: sorted, disjoint inclusive ranges\n\
         /// `(first, last, start, len)`, the scripts of each the `len` numbers from `start` of\n\
         /// `LISTS`.\n",
    );
    text.push_str(&normalize::constant(
        "RANGES",
        "(u32, u32, u16, u8)",
        &lines,
    ));
    let items: Vec<String> = lists.iter().map(u8::to_string).collect();
    text.push_str(
        "\n/// The lists of scripts of `RANGES`, each sorted, a script by its index in\n\
         /// `script::VALUES`.\n",
    );
    text.push_str(&property::short_items("LISTS", "u8", &items));
    Ok(text)
}

/// The binary properties of ECMA-262's table that the data files hold, one constant of ranges
/// each from the first source that has it (after `ucd/PropertyAliases.txt`, against which every
/// name and alias of the table is checked), and the constant `VALUES` of all of them in the
/// order of the table.
fn binary_properties(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let [aliases, data @ ..] = sources else {
        return Err(
            "the renderer binary_properties reads PropertyAliases.txt and the files of \
                    the properties"
                .to_string(),
        );
    };
    if data.is_empty() {
        return Err("the renderer binary_properties reads no file of properties".to_string());
    }
    property::check_property_aliases(ucd, &aliases.path)?;
    let paths: Vec<&str> = data.iter().map(|source| source.path.as_str()).collect();
    let properties = property::binary_properties(ucd, &paths)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! The binary properties of ECMA-262's table (`#table-binary-unicode-properties`) that\n\
         //! the data files hold, which the property escapes read\n\
         //! (`#sec-runtime-semantics-unicodematchproperty-p`). `Any`, `ASCII` and `Assigned` are\n\
         //! made at run time; `Cased` and `Case_Ignorable` are the tables of the case mappings.\n",
    );
    for (name, ranges) in &properties {
        text.push_str(&format!(
            "\n/// The code points of `{name}`: sorted, disjoint inclusive ranges.\n"
        ));
        text.push_str(&property::ranges(&name.to_ascii_uppercase(), ranges));
    }
    let rows: Vec<String> = property::binary_order()
        .iter()
        .map(|name| format!("    {},\n", property::binary_constant(name)))
        .collect();
    text.push_str(
        "\n/// Every binary property of the table that a data file holds, in the order of the\n\
         /// table: the number of a property in `property_names::BINARY` is its index here.\n",
    );
    text.push_str(&normalize::constant("VALUES", "&[(u32, u32)]", &rows));
    Ok(text)
}

/// The names and aliases of the properties of ECMA-262's two tables and of the values of
/// General_Category and Script, as the constants `PROPERTIES`, `CATEGORIES`, `SCRIPTS` and
/// `BINARY`, each sorted by name, from `ucd/PropertyAliases.txt` (the check of the tables'
/// aliases), `ucd/PropertyValueAliases.txt` and `ucd/extracted/DerivedGeneralCategory.txt` (the
/// order of the values of General_Category).
fn property_names(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let [aliases, value_aliases, general] = sources else {
        return Err(format!(
            "the renderer property_names reads three sources (PropertyAliases.txt, \
             PropertyValueAliases.txt, DerivedGeneralCategory.txt), not {}",
            sources.len()
        ));
    };
    property::check_property_aliases(ucd, &aliases.path)?;
    let categories: Vec<String> = property::categories(ucd, &general.path)?
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    let gc = property::values(ucd, &value_aliases.path, "gc")?;
    let sc = property::values(ucd, &value_aliases.path, "sc")?;

    let mut properties: Vec<(String, String)> = Vec::new();
    for (number, (name, names)) in property::NON_BINARY.iter().enumerate() {
        for name in std::iter::once(name).chain(names.iter()) {
            properties.push(((*name).to_owned(), number.to_string()));
        }
    }
    let categories: Vec<(String, String)> = property::category_masks(&categories, &gc)?
        .into_iter()
        .map(|(name, mask)| (name, format!("{mask:#010X}")))
        .collect();
    let mut scripts: Vec<(String, String)> = Vec::new();
    for (number, value) in sc.iter().enumerate() {
        for name in &value.names {
            scripts.push((name.clone(), number.to_string()));
        }
    }
    let order = property::binary_order();
    let mut binary: Vec<(String, String)> = Vec::new();
    for (name, names) in property::BINARY {
        let number = match order.iter().position(|other| other == name) {
            Some(at) => at,
            None => {
                order.len()
                    + property::RUN_TIME
                        .iter()
                        .position(|other| other == name)
                        .unwrap_or_default()
            }
        };
        for name in std::iter::once(name).chain(names.iter()) {
            binary.push(((*name).to_owned(), number.to_string()));
        }
    }
    for (name, _) in &binary {
        if categories.iter().any(|(other, _)| other == name) {
            return Err(format!(
                "{name} is both a binary property and a value of General_Category"
            ));
        }
    }
    let constant =
        |name: &str, row: &str, mut list: Vec<(String, String)>| -> Result<String, String> {
            // A line may give one name twice (`sc ; Ahom ; Ahom`): one row; one name for two values
            // is an error.
            list.sort();
            list.dedup();
            if let Some(pair) = list
                .windows(2)
                .find(|pair| matches!(pair, [a, b] if a.0 == b.0))
            {
                return Err(format!(
                    "{name}: the name {} twice",
                    pair.first()
                        .map(|(name, _)| name.as_str())
                        .unwrap_or_default()
                ));
            }
            let rows: Vec<String> = list
                .iter()
                .map(|(name, value)| format!("    ({name:?}, {value}),\n"))
                .collect();
            Ok(normalize::constant(name, row, &rows))
        };
    let mut text = render::head(sources);
    text.push_str(
        "//! The names and aliases a property escape may give\n\
         //! (`#sec-patterns-static-semantics-early-errors`,\n\
         //! `#sec-runtime-semantics-unicodematchproperty-p`,\n\
         //! `#sec-runtime-semantics-unicodematchpropertyvalue-p-v`): those of ECMA-262's tables of\n\
         //! properties (`#table-nonbinary-unicode-properties`, `#table-binary-unicode-properties`),\n\
         //! and every value and alias of General_Category and Script in `PropertyValueAliases.txt`.\n\
         //! Each list is sorted by name, as bytes, for a binary search; a name matches only as\n\
         //! written (no loose matching).\n\
         \n\
         /// The non-binary properties and the number of each: `0` General_Category, `1` Script,\n\
         /// `2` Script_Extensions.\n",
    );
    text.push_str(&constant("PROPERTIES", "(&str, u8)", properties)?);
    text.push_str(
        "\n/// The values of General_Category and their aliases, each with its mask: bit `i` is the\n\
         /// value `general_category::VALUES[i]`, and a group has the bits of its members.\n",
    );
    text.push_str(&constant("CATEGORIES", "(&str, u32)", categories)?);
    text.push_str(
        "\n/// The values of Script and Script_Extensions and their aliases, each with the index of\n\
         /// its script in `script::VALUES`.\n",
    );
    text.push_str(&constant("SCRIPTS", "(&str, u8)", scripts)?);
    text.push_str(
        "\n/// The binary properties and their aliases, each with its number: the index of its\n\
         /// ranges in `binary_properties::VALUES`, or one of [`ANY`], [`ASCII`] and [`ASSIGNED`].\n",
    );
    text.push_str(&constant("BINARY", "(&str, u8)", binary)?);
    for (offset, name) in property::RUN_TIME.iter().enumerate() {
        text.push_str(&format!(
            "\n/// The number of `{name}`, made at run time.\npub(crate) const {}: u8 = {};\n",
            name.to_ascii_uppercase(),
            order.len() + offset
        ));
    }
    Ok(text)
}

/// The properties of strings of ECMA-262's table from `emoji/emoji-sequences.txt` and
/// `emoji/emoji-zwj-sequences.txt`: per property its single code points, its sequences and the
/// most code points of one of them; the lists of all of them in the order of the table, the
/// bound over all, and the names.
fn properties_of_strings(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    if sources.is_empty() {
        return Err("the renderer properties_of_strings reads no file of sequences".to_string());
    }
    let paths: Vec<&str> = sources.iter().map(|source| source.path.as_str()).collect();
    let properties = emoji::properties(ucd, &paths)?;
    let mut text = render::head(sources);
    text.push_str(
        "//! The properties of strings of ECMA-262's table\n\
         //! (`#table-binary-unicode-properties-of-strings`), which `\\p{...}` names under the flag\n\
         //! `v` (`#sec-runtime-semantics-unicodematchproperty-p`): each property's single code\n\
         //! points and its sequences of two code points or more, from the emoji sequence files of\n\
         //! UTS #51. `RGI_Emoji`, the union of the others (UTS #51, ED-27), is made at run time.\n",
    );
    let names: Vec<String> = emoji::PROPERTIES
        .iter()
        .map(|property| emoji::constant(property))
        .collect();
    for ((property, name), strings) in emoji::PROPERTIES.iter().zip(&names).zip(&properties) {
        text.push_str(&format!(
            "\n/// The single code points of `{property}`: sorted, disjoint inclusive ranges.\n"
        ));
        text.push_str(&render::ranges(
            &format!("{name}_SINGLES"),
            &strings.singles,
        ));
        text.push_str(&format!(
            "\n/// The sequences of `{property}` of two code points or more, sorted by their code\n\
             /// points.\n"
        ));
        text.push_str(&render::sequences(name, &strings.sequences)?);
        text.push_str(&format!(
            "\n/// The most code points of a sequence of `{property}`.\n\
             pub(crate) const {name}_LONGEST: usize = {};\n",
            strings.longest
        ));
    }
    let rows = |suffix: &str| -> Vec<String> {
        names
            .iter()
            .map(|name| format!("    {name}{suffix},\n"))
            .collect()
    };
    text.push_str(
        "\n/// The single code points of each property of the files, in the order of ECMA-262's\n\
         /// table: the index of a property is its number in `NAMES`.\n",
    );
    text.push_str(&normalize::constant(
        "SINGLES",
        "&[(u32, u32)]",
        &rows("_SINGLES"),
    ));
    text.push_str("\n/// The sequences of each property of the files, in the same order.\n");
    text.push_str(&normalize::constant("SEQUENCES", "&[&[u32]]", &rows("")));
    text.push_str(
        "\n/// The most code points of a sequence of each property, in the same order.\n",
    );
    text.push_str(&normalize::constant(
        "LONGEST_OF",
        "usize",
        &rows("_LONGEST"),
    ));
    let longest = properties
        .iter()
        .map(|strings| strings.longest)
        .max()
        .unwrap_or(1);
    text.push_str(&format!(
        "\n/// The most code points of a sequence of any property of strings: the bound of the\n\
         /// lookups of sequences at one position of a match.\n\
         pub(crate) const LONGEST: usize = {longest};\n"
    ));
    let union = emoji::PROPERTIES.len();
    let mut named: Vec<(&str, usize)> = emoji::PROPERTIES
        .iter()
        .copied()
        .enumerate()
        .map(|(index, property)| (property, index))
        .chain(std::iter::once((emoji::UNION, union)))
        .collect();
    named.sort_unstable();
    let rows: Vec<String> = named
        .iter()
        .map(|(name, number)| format!("    ({name:?}, {number}),\n"))
        .collect();
    text.push_str(
        "\n/// The names of the seven properties of strings, which ECMA-262's table gives no alias,\n\
         /// sorted as bytes for a binary search, each with its index in `SINGLES`, or [`UNION`].\n",
    );
    text.push_str(&normalize::constant("NAMES", "(&str, u8)", &rows));
    text.push_str(&format!(
        "\n/// The number of `{}`, the union of the others, made at run time.\n\
         pub(crate) const UNION: u8 = {union};\n",
        emoji::UNION
    ));
    Ok(text)
}
