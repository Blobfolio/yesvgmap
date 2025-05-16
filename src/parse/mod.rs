/*!
# Yesvgmap: Parsing.
*/

mod spec;

use crate::{
	ContentWarnings,
	SvgErrorKind,
};
pub(super) use spec::normalize_attr_case;
use spec::normalize_tag_case;
use svg::{
	node::{
		Attributes,
		element::{
			Element,
			Symbol,
			tag::Type,
		},
		Node,
	},
	parser::{
		Event,
		Parser,
	},
};



#[derive(Debug, Clone, PartialEq)]
/// # SVG Part.
///
/// This is essentially a stripped down version of `svg::Event`, containing
/// only the conditions we're interested in.
pub(crate) enum SvgPart<'a> {
	/// # Opening/Closing Tag.
	///
	/// Unlike the corresponding `Event::Tag`, both the tag and attribute keys
	/// have their casing normalized during initialization.
	Tag(&'a str, Type, Attributes),

	/// # Text.
	Text(&'a str),

	/// # Error.
	Error(SvgErrorKind),
}



/// # (Inline) Style Splitter.
///
/// This iterator is used to naively split a complete style attribute value
/// into separate rule chunks. It essentially just splits on `;`, but not when
/// between quotes or after a backslash.
struct StyleSplitter<'a>(&'a str);

impl<'a> Iterator for StyleSplitter<'a> {
	type Item = &'a str;

	fn next(&mut self) -> Option<Self::Item> {
		// We're done.
		if self.0.is_empty() { return None; }

		let mut quote = None;
		let mut stop = None;
		let mut chars = self.0.char_indices();
		while let Some((pos, c)) = chars.next() {
			// If there's a backslash, ignore it and whatever follows.
			if c == '\\' {
				let _ = chars.next();
				continue;
			}

			// We're inside a quote.
			if let Some(q) = quote {
				if q == c { quote = None; }
				continue;
			}

			// Look for quotes or semi-colons.
			match c {
				'\'' | '"' => {
					quote.replace(c);
				},
				';' => {
					stop = Some(pos + 1);
					break;
				},
				_ => {},
			}
		}

		// If we have a stop, split and return the bit we found.
		if let Some(stop) = stop {
			let (a, b) = self.0.split_at_checked(stop)?;
			let a = a.trim_matches(trim_style);
			self.0 = b.trim_matches(trim_style);

			// If the chunk we just found is empty, recurse!
			if a.is_empty() { return self.next(); }

			// We found it.
			return Some(a);
		}

		// We're returning the whole thing.
		let out = std::mem::take(&mut self.0).trim_matches(trim_style);
		if out.is_empty() { None }
		else { Some(out) }
	}
}



/// # SVG Parser Wrapper.
///
/// This struct provides a thin wrapper around `svg::Parser`, ensuring tags
/// and attributes have the correct case for the spec.
///
/// This also silently suppresses comments, declarations, and instructions,
/// since they're not needed for sprite purposes.
pub(crate) struct SvgParser<'a> {
	/// # Found Opening Tag.
	first: bool,

	/// # Found Closing Tag.
	last: bool,

	/// # Warnings.
	///
	/// Note whether or not the parsed image contains potentially problematic
	/// properties like scripts and styles.
	warn: ContentWarnings,

	/// # Viewport Dimensions.
	///
	/// The width and height used for the `viewBox`. Note this is only
	/// populated when parsing is run through `SvgParser::symbol`.
	viewport: (f32, f32),

	/// # Parser.
	parser: Parser<'a>,
}

impl<'a> SvgParser<'a> {
	/// # New.
	///
	/// Return a new parser for `raw`.
	pub(crate) fn new(raw: &'a str) -> Self {
		Self {
			first: false,
			last: false,
			warn: ContentWarnings::None,
			viewport: (f32::INFINITY, f32::INFINITY),
			parser: Parser::new(raw.trim()),
		}
	}

	/// # Into Symbol.
	///
	/// Parse an SVG as if it were a SYMBOL, returning it if successful.
	pub(crate) fn symbol(&mut self, id: &str) -> Result<Symbol, SvgErrorKind> {
		// Make sure the iterator hasn't started yet.
		if self.first { return Err(SvgErrorKind::Parse); }

		// The first bit will be the opening SVG tag. All we want to port over
		// from that is the viewbox (and the ID passed in).
		let mut out = Symbol::new().set("id", id);
		match self.next() {
			Some(SvgPart::Tag("svg", Type::Start, attr)) => {
				self.viewport = parse_viewport(&attr)?;
				out.assign(
					"viewBox",
					format!("0 0 {} {}", self.viewport.0, self.viewport.1),
				);
			},
			Some(SvgPart::Error(e)) => return Err(e),
			_ => return Err(SvgErrorKind::ParseStart),
		}

		// Bring over all the child elements.
		while ! self.last {
			if let Some(next) = self.next_element() {
				if ! is_empty(&next) { out.append(next); }
			}
			else if ! self.last { return Err(SvgErrorKind::Parse); }
		}

		// It's gotta have children.
		if (*out).get_children().is_empty() { Err(SvgErrorKind::Parse) }
		// And it can't have scripting elements or attributes.
		else if self.warn.contains_any(ContentWarnings::Scripts).is_some() {
			Err(SvgErrorKind::ParseScript)
		}
		// Otherwise it's probably fine.
		else { Ok(out) }
	}

	/// # Viewport.
	///
	/// Return the width and height used for the `viewBox`.
	pub(crate) const fn viewport(&self) -> (f32, f32) { self.viewport }

	/// # Content Warnings.
	///
	/// Return the content warnings found during parsing, if any.
	pub(crate) const fn warnings(&self) -> Option<ContentWarnings> {
		if self.warn.is_none() { None }
		else { Some(self.warn) }
	}

	/// # Next Element.
	///
	/// Like `next`, except it keeps going until the entire element has been
	/// built up.
	///
	/// For this to work, the next thing has to either be an opening tag or an
	/// empty one. It will return `None` if something else is encountered
	/// instead.
	fn next_element(&mut self) -> Option<Element> {
		match self.next()? {
			// Empty elements are easy!
			SvgPart::Tag(tag, Type::Empty, attr) => {
				let mut next = Element::new(tag);
				for (k, v) in attr { next.assign(k, v); }
				self.check_element(&next);
				Some(next)
			},

			// The start of something requires recursion.
			SvgPart::Tag(tag, Type::Start, attr) => {
				self.next_element_recurse(tag, attr)
			},

			// Nothing else should appear here.
			_ => None,
		}
	}

	/// # Next Element (Recursive).
	///
	/// This method takes over from `next_element` when it encounters a
	/// non-empty start tag, allowing for the collection of all of its
	/// children.
	///
	/// `None` will be returned if no closing tag is ever found.
	fn next_element_recurse(&mut self, root_tag: &'a str, attr: Attributes) -> Option<Element> {
		let mut out = Element::new(root_tag);
		for (k, v) in attr { out.assign(k, v); }

		let mut closed = false;
		while let Some(next) = self.next() {
			match next {
				// The end!
				SvgPart::Tag(tag, Type::End, _) if tag == root_tag => {
					closed = true;
					break;
				},

				// Text gets added if non-empty.
				SvgPart::Text(v) => if ! v.is_empty() {
					out.append(svg::node::Text::new(v));
				},

				// Empty tags can be added straight too.
				SvgPart::Tag(tag, Type::Empty, attr) => {
					let mut next = Element::new(tag);
					for (k, v) in attr { next.assign(k, v); }
					if ! is_empty(&next) {
						self.check_element(&next);
						out.append(next);
					}
				},

				// Tags with a new start require recursion.
				SvgPart::Tag(tag, Type::Start, attr) =>
					if let Some(next) = self.next_element_recurse(tag, attr) {
						if ! is_empty(&next) { out.append(next); }
					},

				// Whatever else there might be, we're not interested.
				_ => {},
			}
		}

		// If it closed, we're good!
		if closed {
			self.check_element(&out);
			Some(out)
		}
		else { None }
	}

	/// # Calculate Element Warnings.
	///
	/// This method is used to check an individual element for problematic
	/// properties, whether the tag itself or the attributes attached to it.
	///
	/// Empty elements, even if scripts or tags, are ignored since they'll
	/// ultimately be excluded from the map.
	fn check_element(&mut self, el: &Element) {
		if ! is_empty(el) {
			// Check the name.
			match el.get_name() {
				"script" => { self.warn.set(ContentWarnings::ScriptTag); },
				"style" => { self.warn.set(ContentWarnings::StyleTag); },
				_ => {},
			}

			// Check the attributes.
			for k in el.get_attributes().keys() {
				match k.as_str() {
					"id" => { self.warn.set(ContentWarnings::IdAttr); },
					"class" => { self.warn.set(ContentWarnings::ClassAttr); },
					"style" => { self.warn.set(ContentWarnings::StyleAttr); },
					k => if spec::ATTR_EVENT.contains(&k) {
						self.warn.set(ContentWarnings::OnAttr);
					},
				}
			}
		}
	}
}

impl<'a> Iterator for SvgParser<'a> {
	type Item = SvgPart<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		// The first thing we return must be an SVG tag.
		if ! self.first {
			loop {
				match self.parser.next() {
					// Bingo!
					Some(Event::Tag(tag, Type::Start, mut attr)) if tag.eq_ignore_ascii_case("svg") => {
						normalize_attributes(&mut attr);
						self.first = true;
						return Some(SvgPart::Tag("svg", Type::Start, attr));
					},

					// Not a good start!
					Some(Event::Error(_)) => return Some(SvgPart::Error(SvgErrorKind::Parse)),

					// Ignore and try again.
					Some(Event::Comment(_) | Event::Declaration(_) | Event::Instruction(_)) => {},

					// Everything else is bad.
					_ => return Some(SvgPart::Error(SvgErrorKind::ParseStart)),
				}
			}
		}

		// See what turns up!
		while let Some(e) = self.parser.next() {
			match e {
				Event::Error(_) => return Some(SvgPart::Error(SvgErrorKind::Parse)),
				Event::Text(v) => return Some(SvgPart::Text(v)),
				Event::Tag(tag, ty, mut attr) => {
					// Clean up the data.
					let tag = match normalize_tag_case(tag) {
						Ok(v) => v,
						Err(v) =>
							// Tags basically share the same formatting
							// requirements as attributes…
							if crate::valid_attr(v) { v }
							else {
								return Some(SvgPart::Error(SvgErrorKind::Parse));
							},
					};
					normalize_attributes(&mut attr);

					// Styles don't need a type.
					if tag == "style" { let _ = attr.remove("type"); }

					// Handle SVG tags specifically.
					else if tag == "svg" {
						// The end!
						if matches!(ty, Type::End) {
							self.last = true;

							// Make sure there are no trailing elements before
							// we call it!
							if self.parser.any(|next| matches!(next, Event::Text(_) | Event::Tag(_, _, _))) {
								return Some(SvgPart::Error(SvgErrorKind::ParseEnd));
							}
						}
						// There can be only one…
						else { return Some(SvgPart::Error(SvgErrorKind::ParseSvgSvg)); }
					}

					// It's okay, folks!
					return Some(SvgPart::Tag(tag, ty, attr));
				},
				// Skip anything else.
				_ => {},
			}
		}

		// We're out of tags!
		if self.last { None }
		// But didn't finish!
		else {
			self.last = true; // Force an abort if called again.
			Some(SvgPart::Error(SvgErrorKind::ParseEnd))
		}
	}
}



/// # Is Empty Element?
///
/// Returns `true` for certain tags if they have no children or attributes.
fn is_empty(src: &Element) -> bool {
	/// # Safe Tag List.
	///
	/// These elements can be _safely_ considered empty provided they have no
	/// children and no attributes.
	static EMPTY_IF_NO_ATTR: [&str; 8] = [
		"a", "g", "glyph", "marker", "mask", "missing-glyph", "pattern", "switch",
	];

	// A lack of children is the first requirement to being empty!
	if src.get_children().is_empty() {
		let tag = src.get_name();
		let attr = src.get_attributes();

		// For some elements, children are everything.
		tag == "defs" ||
		tag == "desc" ||
		tag == "style" ||
		tag == "title" ||

		// For the rest, it's safer to require they have NO attributes of
		// any kind before calling them "empty".
		(attr.is_empty() && EMPTY_IF_NO_ATTR.contains(&tag))
	}
	else { false }
}

/// # Normalize Attributes.
///
/// This normalizes attribute key casing and moves inline styles to tag-level
/// attributes (where valid).
fn normalize_attributes(attr: &mut Attributes) {
	// Start with what's already there.
	let mut fixed = Attributes::new();
	attr.retain(|k, v|
		match normalize_attr_case(k) {
			Ok(k2) if k != k2 => {
				fixed.insert(k2.to_owned(), v.clone());
				false
			},
			// This should never happen.
			Err(k2) if ! crate::valid_attr(k2) => false,
			_ => true,
		}
	);
	attr.extend(fixed.drain());

	// Try to move styles over to attributes.
	if let Some((tag, raw)) = attr.remove_entry("style") {
		let mut chunks: Vec<&str> = StyleSplitter(&raw).collect();
		chunks.retain(|v| {
			if let Some((k2, v2)) = v.split_once(':') {
				// If the style key matches a known attribute (other than
				// display) convert it to an attribute.
				if let Ok(k3) = normalize_attr_case(k2) {
					if k3 != "display" {
						attr.insert(k3.to_owned(), v2.trim().into());
						return false;
					}
				}
			}
			true
		});

		// Add back the style(s) we couldn't move.
		if ! chunks.is_empty() {
			attr.insert(tag, chunks.join(";").into());
		}
	}
}

/// # Normalize Viewbox.
///
/// This method attempts to parse the size of the viewport — to be used when
/// constructing the `viewBox` — either from an existing `viewBox` or
/// separate `width`/`height` attributes.
///
/// ## Errors
///
/// This will return an error if there is an existing `viewBox` that has
/// offsets or non-positive width/height, or if it doesn't and the `width`
/// and `height` attributes are missing or have non-positive values.
fn parse_viewport(attr: &Attributes) -> Result<(f32, f32), SvgErrorKind> {
	// If there's a viewBox, we have to use it!
	if let Some(vb) = attr.get("viewBox") {
		let mut split = vb.split(|c: char| c.is_whitespace() || c == ',')
			.filter_map(|v| {
				let v = v.trim();
				if v.is_empty() { None }
				else { Some(v) }
			});

		// There should be exactly four numbers.
		let a = split.next()
			.and_then(|n| n.parse::<f32>().ok())
			.ok_or(SvgErrorKind::ParseViewBox)?;
		let b = split.next()
			.and_then(|n| n.parse::<f32>().ok())
			.ok_or(SvgErrorKind::ParseViewBox)?;
		let w = split.next()
			.and_then(|n| n.parse::<f32>().ok())
			.ok_or(SvgErrorKind::ParseViewBox)?;
		let h = split.next()
			.and_then(|n| n.parse::<f32>().ok())
			.ok_or(SvgErrorKind::ParseViewBox)?;

		// That's it!
		if split.next().is_some() { return Err(SvgErrorKind::ParseViewBox); }

		// The first two must be zero.
		if a != 0.0 || b != 0.0 { return Err(SvgErrorKind::ParseViewBoxOffset); }

		// The second two must be non-zero.
		if
			w.is_nan() || w.is_infinite() || w <= 0.0 ||
			h.is_nan() || h.is_infinite() || h <= 0.0
		{
			return Err(SvgErrorKind::ParseViewportSize);
		}

		// We're good!
		return Ok((w, h));
	}

	// Otherwise we'll have to try to build one manually from the width and
	// height attributes.
	let w = attr.get("width")
		.ok_or(SvgErrorKind::ParseViewportAttr)?
		.trim_start()
		.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c.is_whitespace())
		.parse::<f32>()
		.map_err(|_| SvgErrorKind::ParseViewportSize)?;
	let h = attr.get("height")
		.ok_or(SvgErrorKind::ParseViewportAttr)?
		.trim_start()
		.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c.is_whitespace())
		.parse::<f32>()
		.map_err(|_| SvgErrorKind::ParseViewportSize)?;

	// Same as with viewBox, they have to be normal and positive.
	if
		w.is_nan() || w.is_infinite() || w <= 0.0 ||
		h.is_nan() || h.is_infinite() || h <= 0.0
	{
		Err(SvgErrorKind::ParseViewportSize)
	}
	// We're good!
	else { Ok((w, h)) }
}

/// # Trim Style (Callback).
///
/// Get rid of whitespace and semi-colons.
const fn trim_style(c: char) -> bool { c.is_whitespace() || c == ';' }



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	/// # Test `svg` Parser.
	///
	/// The `svg` parser does not normalize casing.
	fn t_parse_a() {
		let mut parser = Parser::new(r#"<SVG WIDTH="5" HEIGHT="4" style="fill : red ; foo:bar;"></SVG>"#);
		let Some(Event::Tag("SVG", Type::Start, attr)) = parser.next() else {
			panic!("Failed to parse SVG.");
		};
		assert_eq!(attr.len(), 3);
		assert_eq!(attr.get("WIDTH"), Some(&"5".into()));
		assert_eq!(attr.get("HEIGHT"), Some(&"4".into()));
		assert_eq!(attr.get("style"), Some(&"fill : red ; foo:bar;".into()));

		// All that's left is the closing tag.
		assert!(matches!(parser.next(), Some(Event::Tag("SVG", Type::End, _))));
		assert!(parser.next().is_none());
	}

	#[test]
	/// # Test Our Parser.
	///
	/// Our parser normalizes casing and inlines known CSS definitions.
	fn t_parse_b() {
		let mut parser = SvgParser::new(r#"<SVG WIDTH="5" HEIGHT="4" style="fill : red ; foo:bar;"></SVG>"#);
		let Some(SvgPart::Tag("svg", Type::Start, attr)) = parser.next() else {
			panic!("Failed to parse SVG.");
		};
		assert_eq!(attr.len(), 4);
		assert_eq!(attr.get("width"), Some(&"5".into()));
		assert_eq!(attr.get("height"), Some(&"4".into()));
		assert_eq!(attr.get("fill"), Some(&"red".into()));
		assert_eq!(attr.get("style"), Some(&"foo:bar".into()));

		// All that's left is the closing tag.
		assert!(matches!(parser.next(), Some(SvgPart::Tag("svg", Type::End, _))));
		assert!(parser.next().is_none());
	}

	#[test]
	/// # Viewbox Formatting.
	fn t_parse_viewport() {
		let mut attr = Attributes::new();

		// Test viewbox on its own.
		for (raw, expected) in [
			("0 0 3 4", Ok((3.0, 4.0))),
			(".0 0. 3 4", Ok((3.0, 4.0))),
			(" 0.0 ,  0 3.333, 4\n", Ok((3.333, 4.0))),
			(" -0.0 ,  0 3.333,\t 4.444\n", Ok((3.333, 4.444))),
			("1 0 3 4", Err(SvgErrorKind::ParseViewBoxOffset)), // Offset X.
			("0 1 3 4", Err(SvgErrorKind::ParseViewBoxOffset)), // Offset Y.
			("0 1 3 ", Err(SvgErrorKind::ParseViewBox)),        // Too few numbers.
			("", Err(SvgErrorKind::ParseViewBox)),
			("0 1 3 4 5", Err(SvgErrorKind::ParseViewBox)),     // Too many numbers.
			("0 0 3 n", Err(SvgErrorKind::ParseViewBox)),       // Invalid char.
			(". . 3 4", Err(SvgErrorKind::ParseViewBox)),
			("0 0 3 4 n", Err(SvgErrorKind::ParseViewBox)),
			("0 0 0 1", Err(SvgErrorKind::ParseViewportSize)),  // Bad width.
			("0 0 -1 1", Err(SvgErrorKind::ParseViewportSize)),
			("0 0 1 0", Err(SvgErrorKind::ParseViewportSize)),  // Bad height.
			("0 0 1 -1", Err(SvgErrorKind::ParseViewportSize)),
		] {
			attr.clear();
			attr.insert("viewBox".to_owned(), raw.into());
			assert_eq!(
				parse_viewport(&attr),
				expected,
			);

			// Width and height shouldn't change anything for a fucked-up
			// viewbox.
			attr.insert("width".to_owned(), "666".into());
			attr.insert("height".to_owned(), "666".into());
			assert_eq!(
				parse_viewport(&attr),
				expected,
			);
		}

		// Now let's test width/height.
		for (w, h, expected) in [
			("1", "2", Ok((1.0, 2.0))),
			(" 1", " 2px ", Ok((1.0, 2.0))),
			("1.3", "2.44", Ok((1.3, 2.44))),
			("0", "2", Err(SvgErrorKind::ParseViewportSize)), // Bad width.
			("-0", "2", Err(SvgErrorKind::ParseViewportSize)),
			("-3", "2", Err(SvgErrorKind::ParseViewportSize)),
			("2", "0", Err(SvgErrorKind::ParseViewportSize)), // Bad height.
			("2", "-0", Err(SvgErrorKind::ParseViewportSize)),
			("2", "-3", Err(SvgErrorKind::ParseViewportSize)),
			("n1", " 2px ", Err(SvgErrorKind::ParseViewportSize)), // Invalid char.
			("", "", Err(SvgErrorKind::ParseViewportSize)),        // Empty.
			("", "", Err(SvgErrorKind::ParseViewportSize)),
		] {
			attr.clear();
			attr.insert("width".to_owned(), w.into());
			attr.insert("height".to_owned(), h.into());
			assert_eq!(
				parse_viewport(&attr),
				expected,
			);
		}

		// Let's also double-check what happens when we're missing too much.
		attr.clear();
		attr.insert("width".to_owned(), "666".into());
		assert_eq!(
			parse_viewport(&attr),
			Err(SvgErrorKind::ParseViewportAttr),
		);
		attr.clear();
		attr.insert("height".to_owned(), "666".into());
		assert_eq!(
			parse_viewport(&attr),
			Err(SvgErrorKind::ParseViewportAttr),
		);
	}

	#[test]
	fn t_symbol_content_warnings() {
		for (raw, expected) in [
			(
				include_str!("../../test-assets/arrow-1.svg"),
				Some(ContentWarnings::StyleTag | ContentWarnings::ClassAttr),
			),
			(
				include_str!("../../test-assets/arrow-2.svg"),
				Some(ContentWarnings::ClassAttr),
			),
			(
				include_str!("../../test-assets/arrow-3.svg"),
				Some(ContentWarnings::IdAttr),
			),
			(
				include_str!("../../test-assets/bitcoin.svg"),
				None,
			),
		] {
			let mut parser = SvgParser::new(raw);
			assert!(
				parser.symbol("foo").is_ok(),
				"Failed to parse symbol: {raw}",
			);
			assert_eq!(parser.warnings(), expected);
		}

		// The last two have scripts so should fail.
		for raw in [
			include_str!("../../test-assets/arrow-4.svg"),
			include_str!("../../test-assets/arrow-5.svg"),
		] {
			let mut parser = SvgParser::new(raw);
			assert!(
				matches!(parser.symbol("foo"), Err(SvgErrorKind::ParseScript)),
				"Scripts were not detected: {raw}",
			);
		}
	}
}
