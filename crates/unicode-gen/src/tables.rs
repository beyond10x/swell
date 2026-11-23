//! The renderers of the tables, by the name a line of `crates/unicode-gen/tables.txt` gives.
//! A renderer gets the sources of its line, in the order of the line, and returns the whole
//! text of its table file.

use crate::render::{self, Source};
use crate::ucd::Ucd;

/// A renderer: the text of a table from its sources.
pub(crate) type Renderer = fn(&mut Ucd, &[Source]) -> Result<String, String>;

/// The renderer named `name`, if there is one.
pub(crate) fn renderer(name: &str) -> Option<Renderer> {
    match name {
        "zs" => Some(zs),
        _ => None,
    }
}

/// General_Category `Zs` from `ucd/extracted/DerivedGeneralCategory.txt`, as the constant `ZS`.
fn zs(ucd: &mut Ucd, sources: &[Source]) -> Result<String, String> {
    let [source] = sources else {
        return Err(format!(
            "the renderer zs reads one source, not {}",
            sources.len()
        ));
    };
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
