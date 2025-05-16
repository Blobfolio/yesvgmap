/*!
# Yesvgmap: Sprite.
*/

use crate::{
	ContentWarnings,
	SvgError,
	SvgErrorKind,
	SvgParser,
};
use dowser::Dowser;
use fyi_ansi::{
	ansi,
	csi,
	dim,
};
use std::{
	collections::{
		btree_map::Entry,
		BTreeMap,
		BTreeSet,
	},
	ffi::OsStr,
	fmt,
	path::{
		Path,
		PathBuf,
	},
};
use svg::node::{
	element::SVG,
	Node,
};



#[derive(Debug, Clone)]
/// # Runtime Settings.
///
/// This struct holds the `Sprite`-related settings. (It is essentially a
/// builder without the usual builder-style methods.)
pub(crate) struct SpriteOptions {
	/// # SVG ID.
	attributes: BTreeMap<String, String>,

	/// # Paths.
	paths: Dowser,

	/// # SVG Prefix.
	prefix: String,
}

impl Default for SpriteOptions {
	#[inline]
	fn default() -> Self {
		Self {
			attributes: BTreeMap::new(),
			paths: Dowser::default(),
			prefix: String::from("i"),
		}
	}
}

impl SpriteOptions {
	/// # Add Attribute.
	///
	/// Append an arbitrary key/value attribute to the sprite's outer `<svg>`
	/// tag, such as an `id` or `class`.
	///
	/// ## Errors
	///
	/// This will return an error if the key is invalid or already defined.
	pub(crate) fn set_attribute(&mut self, key: &str, value: Option<&str>)
	-> Result<(), SvgError> {
		// Normalize the key.
		let key = match crate::normalize_attr_case(key) {
			Ok(k) => k,
			Err(k) =>
				if crate::valid_attr(k) { k }
				else {
					return Err((SvgErrorKind::InvalidAttribute, k).into());
				},
		};

		// Normalize the value.
		let mut value = value.map_or_else(
			||
				// Hidden and disable are the only two boolean attributes that
				// make any sense to add; we can default them to themselves.
				if key == "hidden" || key == "disabled" { key }
				// Everything else should just be blank.
				else { "" },
			|v| v.trim(),
		);

		// Strip quotes if any, but only if they appear on both ends.
		if match value.as_bytes() {
			[b'"', b'"'] | [b'\'', b'\''] => true,
			[b'"', .., m, b'"'] | [b'\'', .., m, b'\''] => *m != b'\\',
			_ => false,
		} {
			value = value.get(1..value.len() - 1).unwrap_or("");
		}

		// If the attribute is already set, return an error.
		if self.attributes.insert(key.to_owned(), value.to_owned()).is_some() {
			return Err((SvgErrorKind::DupeAttribute, key).into());
		}

		Ok(())
	}

	/// # Add Crawl Path.
	///
	/// Add, but do not (yet) a crawl, a new file or directory path.
	///
	/// Note that no particular validation is performed here. It'll either
	/// yield SVGs later on or it won't.
	pub(crate) fn set_path(&mut self, path: PathBuf, list: bool)
	-> Result<(), SvgError> {
		if list {
			self.paths.read_paths_from_file(&path)
				.map_err(|_| (SvgErrorKind::Read, path).into())
		}
		else {
			self.paths.push_path(path);
			Ok(())
		}
	}

	/// # Set Prefix.
	///
	/// Customize the `id` prefix used for sprite symbols, minus the trailing
	/// `'-'`, which gets added automatically.
	///
	/// The default prefix is simply `'i'`.
	///
	/// ## Errors
	///
	/// This will return an error if the prefix would be invalid as an HTML
	/// `id`.
	pub(crate) fn set_prefix(&mut self, mut prefix: String) -> Result<(), SvgError> {
		if crate::valid_id(prefix.as_str()) {
			prefix.make_ascii_lowercase();
			prefix.clone_into(&mut self.prefix);
			Ok(())
		}
		else {
			Err((SvgErrorKind::InvalidPrefix, prefix).into())
		}
	}
}



/// # SVG Sprite.
///
/// This struct holds a (completed) SVG sprite and details about the symbols
/// that were incorporated into it.
pub(crate) struct Sprite {
	/// # The SVG element.
	el: SVG,

	/// # Symbol IDs and Dimensions.
	symbols: SpriteSymbols,

	/// # Warnings.
	warnings: BTreeMap<PathBuf, ContentWarnings>,
}

impl fmt::Display for Sprite {
	#[inline]
	/// # Print as XML!
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		<SVG as fmt::Display>::fmt(&self.el, f)
	}
}

impl TryFrom<SpriteOptions> for Sprite {
	type Error = SvgError;

	fn try_from(opts: SpriteOptions) -> Result<Self, Self::Error> {
		// Break apart the settings.
		let SpriteOptions { attributes, paths, prefix } = opts;
		let paths = paths.filter(crate::valid_extension);

		// Initialize an (outer) `<svg>` wrapper to hold all the icon
		// `<symbol>`s we'll be creating.
		let mut el = SVG::new()
			.set("xmlns", "http://www.w3.org/2000/svg")
			.set("aria-hidden", "true")
			.set("style", "display:none");
		for (k, v) in attributes { el = el.set(k, v); }

		// Collect the paths by stem, ensuring uniqueness/sanity as we go.
		let mut src = BTreeMap::<String, PathBuf>::new();
		for p in paths {
			let k = make_symbol_id(&prefix, &p).ok_or((SvgErrorKind::InvalidFileName, &p))?;
			match src.entry(k) {
				Entry::Vacant(e) => { e.insert(p); },
				Entry::Occupied(_) => return Err((SvgErrorKind::DupeFileName, p).into()),
			}
		}

		// Nothing?
		if src.is_empty() { return Err(SvgErrorKind::NoSvgs.into()); }

		// Parse each file!
		let mut symbols = Vec::new();
		let mut warnings = BTreeMap::new();
		for (id, path) in src {
			let raw = std::fs::read_to_string(&path)
				.map_err(|_| (SvgErrorKind::Read, &path))?;

			let mut parser = SvgParser::new(&raw);
			el.append(parser.symbol(&id).map_err(|e| (e, &path))?);
			let (w, h) = parser.viewport();
			symbols.push((id, w, h));

			// Note the warnings, if any.
			if let Some(warn) = parser.warnings() {
				warnings.insert(path, warn);
			}
		}

		// And return!
		Ok(Self {
			el,
			symbols: SpriteSymbols(symbols),
			warnings,
		})
	}
}

impl Sprite {
	/// # Check IDs.
	///
	/// Rescan the entire `<svg>` tree to collect `id` attributes, returning
	/// an error if any are duplicated.
	pub(crate) fn check_ids(&self) -> Result<(), SvgError> {
		use svg::node::{Attributes, Children};

		/// # Recurse Children.
		fn check_tree<'a>(
			attr: Option<&'a Attributes>,
			children: Option<&'a Children>,
			all: &mut BTreeSet<&'a str>
		) -> Result<(), SvgError> {
			// If there's an ID, try to add it.
			if let Some(id) = attr.and_then(|v| v.get("id")) {
				let id: &str = id;
				if ! all.insert(id) {
					return Err((SvgErrorKind::DupeId, id).into());
				}
			}

			// If there are children, check them too.
			if let Some(children) = children {
				for child in children {
					check_tree(
						child.get_attributes(),
						child.get_children(),
						all,
					)?;
				}
			}

			Ok(())
		}

		let mut all = BTreeSet::new();
		check_tree(self.el.get_attributes(), self.el.get_children(), &mut all)
	}

	/// # Symbols.
	///
	/// Return the symbol IDs.
	pub(crate) const fn symbols(&self) -> &SpriteSymbols { &self.symbols }

	/// # Warnings.
	///
	/// Return the content warnings encountered during parsing, if any.
	pub(crate) const fn warnings(&self) -> &BTreeMap<PathBuf, ContentWarnings> {
		&self.warnings
	}
}



#[derive(Debug, Clone)]
/// # Sprite Symbols.
///
/// This struct contains the symbol IDs and dimensions that were incorporated
/// into a [`Sprite`]. It is only really used for display purposes.
pub(crate) struct SpriteSymbols(Vec<(String, f32, f32)>);

impl SpriteSymbols {
	/// # Length.
	///
	/// Note this should never be zero.
	pub(crate) const fn len(&self) -> usize { self.0.len() }
}

impl fmt::Display for SpriteSymbols {
	#[inline]
	/// # Prettyprint the IDs and dimensions.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Find the maximum ID width to for alignment.
		let Some(max) = self.0.iter().map(|v| v.0.len()).max() else { return Ok(()); };

		for (id, w, h) in &self.0 {
			writeln!(
				f,
				concat!(
					ansi!((bold, light_green) "  ↳ "), "{id:max$}    ",
					dim!(
						"{{ ",
						csi!(light_cyan), "aspect-ratio",
						csi!(dark_orange), ": ",
						csi!(light_yellow), "{w}",
						csi!(dark_orange), " / ",
						csi!(light_yellow), "{h}",
						csi!(dark_orange), ";",
						csi!(!fg), " }}",
					),
				),
				id=id,
				w=w,
				h=h,
				max=max,
			)?;
		}

		Ok(())
	}
}



/// # Symbol ID From Path and Prefix.
///
/// Icon IDs are formatted `[prefix]-[stem]`, with UPPER ASCII lowercased,
/// contiguous whitespace and underscores converted to `-`, and anything
/// other than dashes and digits removed.
fn make_symbol_id(prefix: &str, path: &Path) -> Option<String> {
	let stem = path.file_stem().and_then(OsStr::to_str)?.trim();
	let mut out = String::with_capacity(prefix.len() + 1 + stem.len());
	out.push_str(prefix);
	out.push('-');

	// Add the stem, converting/dropping unusable characters as we go.
	let before = out.len();
	let mut ws = true;
	for c in stem.chars() {
		match c {
			// Passthrough.
			'a'..='z' | '0'..='9' | '-' => {
				out.push(c);
				ws = false;
			},
			// Fix case.
			'A'..='Z' => {
				out.push(c.to_ascii_lowercase());
				ws = false;
			},
			// Convert whitespace and underscores to dashes for
			// consistency, but only once per region.
			_ => if ! ws && (c.is_whitespace() || c == '_') {
				out.push('-');
				ws = true;
			},
		}
	}

	// Strip trailing dashes.
	let trimmed = out.trim_end_matches('-');
	if trimmed.len() < out.len() { out.truncate(trimmed.len()); }

	// Make sure the ID is valid-looking and contains at least _something_
	// from the file stem.
	if before < out.len() && crate::valid_id(&out) { Some(out) }
	else { None }
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_symbol_id() {
		for (prefix, path, expected) in [
			("i", "/root/image.svg", Some("i-image")),
			("i", "./image.svg", Some("i-image")),
			("i", "image-.svg", Some("i-image")),
			("foo", "b _ (A) r.svg", Some("foo-b-a-r")),
			("i", "Image_Name33", Some("i-image-name33"))
		] {
			assert_eq!(
				make_symbol_id(prefix, path.as_ref()).as_deref(),
				expected,
			);
		}
	}
}
