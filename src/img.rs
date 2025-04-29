/*!
# Yesvgmap: Boundaries
*/

use crate::SvgError;
use fyi_ansi::dim;
use fyi_msg::Msg;
use std::{
	borrow::Cow,
	collections::BTreeMap,
	fmt,
	path::{
		Path,
		PathBuf,
	},
};
use svg::{
	node::{
		Attributes,
		element::{
			Element,
			Symbol,
			tag::Type,
			SVG,
		},
		Node,
		Value,
	},
	parser::{
		Event,
		Parser,
	},
};



include!(concat!(env!("OUT_DIR"), "/content-warnings.rs"));

impl fmt::Display for ContentWarnings {
	/// # To String.
	///
	/// This emits SVG code, slightly compressed.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.is_none() { return Ok(()); }

		// Pre-collect the warnings to compact the formatting.
		let mut what = Vec::with_capacity(3);

		match (self.contains(Self::ScriptTag), self.contains(Self::StyleTag)) {
			(true, true) => { what.push("<script>/<style> tags"); },
			(true, false) => { what.push("<script> tags"); },
			(false, true) => { what.push("<style> tags"); },
			(false, false) => {},
		}

		match (self.contains(Self::ClassAttr), self.contains(Self::IdAttr)) {
			(true, true) => { what.push("class/id attributes"); },
			(true, false) => { what.push("classes"); },
			(false, true) => { what.push("IDs"); },
			(false, false) => {},
		}

		match (self.contains(Self::InlineScript), self.contains(Self::InlineStyle)) {
			(true, true) => { what.push("inline scripts/styles"); },
			(true, false) => { what.push("inline scripts"); },
			(false, true) => { what.push("inline styles"); },
			(false, false) => {},
		}

		match what.as_slice() {
			[a, b, c] => write!(f, "{a}, {b}, and {c}"),
			[a, b] => write!(f, "{a} and {b}"),
			[c] => f.write_str(c),
			_ => Ok(()),
		}
	}
}



#[derive(Debug, Clone)]
/// # SVG Map.
///
/// This holds an `svg` element with some number of `symbol` children.
pub(super) struct Map {
	/// # SVG.
	inner: SVG,

	/// # Hide Type.
	hide: HideType,

	/// # Length.
	len: usize,
}

impl fmt::Display for Map {
	/// # To String.
	///
	/// This emits SVG code, slightly compressed.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Stringify the SVG.
		let mut raw = self.inner.to_string();

		// The hidden attribute shouldn't have a "true" attached to it.
		if matches!(self.hide, HideType::Hidden) {
			if let Some(pos) = raw.find(r#" hidden="true""#) {
				raw.replace_range(pos+7..pos+14, "");
			}
		}

		// Clean up whitespace a bit.
		let mut out = String::with_capacity(raw.len());
		let mut last = '?';
		let mut iter = raw.chars().peekable();
		while let Some(c) = iter.next() {
			if  c == '\n' && (last == '>' || iter.peek() == Some(&'<')) {
				continue;
			}

			last = c;
			out.push(c);
		}

		// Print it!
		f.write_str(&out)
	}
}

impl Map {
	/// # New.
	pub(super) fn new(
		id: Option<&str>,
		class: Option<&str>,
		hide: HideType,
		prefix: &str,
		paths: &[PathBuf],
	) -> Result<Self, SvgError> {
		// There have to be paths.
		if paths.is_empty() {
			return Err(SvgError::NoSvgs);
		}

		// Start the map!
		let mut map = SVG::new()
			.set("xmlns", "http://www.w3.org/2000/svg")
			.set("aria-hidden", "true");

		// Add an ID?
		if let Some(id) = id { map = map.set("id", id); }

		// Add a class?
		if let Some(class) = class { map = map.set("class", class); }

		// Hide it in some way?
		match hide {
			HideType::Hidden => {
				map = map.set("hidden", "true");
			},
			HideType::Offscreen => {
				map = map.set("style", "position:fixed;top:0;left:-100px;width:1px;height:1px;overflow:hidden");
			},
			HideType::None => {},
		}

		// Handle the paths!
		let len: usize = paths.len();
		let mut nice_paths: BTreeMap<Cow<str>, Symbol> = BTreeMap::default();
		for path in paths {
			// The symbol ID is built from the alphanumeric (and dash)
			// characters in the file name.
			let stem = parse_stem_id(path)
				.ok_or_else(|| SvgError::FileName(path.clone()))?;

			// Build up the symbol.
			let (s, warn) = parse_as_symbol(path, &stem, prefix)?;

			// Push it to temporary storage.
			if nice_paths.insert(stem.clone(), s).is_some() {
				return Err(SvgError::Duplicate(stem.into_owned()));
			}

			// Note if this has styles or other issues.
			if ! warn.is_none() {
				if let Some(name) = path.file_name() {
					Msg::warning(format!(
						concat!(dim!("{file}"), " contains {contents}."),
						file=name.to_string_lossy(),
						contents=warn,
					)).eprint();
				}
			}
		}

		// Done!
		Ok(Self {
			// We can add the children on-the-fly.
			inner: nice_paths.into_iter().fold(map, |m, (_, s)| m.add(s)),
			hide,
			len,
		})
	}

	/// # Length.
	///
	/// Return the number of children (`symbol` elements).
	pub(super) const fn len(&self) -> usize { self.len }
}



#[derive(Debug, Clone, Copy)]
/// # Map Hiding Strategy.
///
/// SVG maps aren't generally intended for direct display. This enum holds the
/// different strategies for keeping it that way.
pub(super) enum HideType {
	/// # Don't Hide.
	None,

	/// # Hide with `hidden` Attribute.
	Hidden,

	/// # Position Offscreen.
	Offscreen,
}



/// # Is Empty Element?
fn is_empty(src: &Element) -> bool {
	src.get_attributes().is_empty() &&
	src.get_children().is_empty() &&
	matches!(
		src.get_name(),
		"a" | "defs" | "glyph" | "g" | "marker" | "mask" | "missing-glyph" |
		"pattern" | "script" | "style" | "switch"
	)
}

/// # Check for Styles, Classes, IDs.
///
/// Styles, classes, and IDs inside of SVGs have a habit of colliding with one
/// another, particularly in map contexts. This method looks to see if there
/// are any so we can issue a warning.
fn has_styles(src: &[Event]) -> ContentWarnings {
	let mut warnings = ContentWarnings::None;
	for e in src {
		if let Event::Tag(name, _, attrs) = e {
			if name.eq_ignore_ascii_case("style") {
				warnings.set(ContentWarnings::StyleTag);
			}
			else if name.eq_ignore_ascii_case("script") {
				warnings.set(ContentWarnings::ScriptTag);
			}
			else {
				for k in attrs.keys() {
					if k.eq_ignore_ascii_case("id") {
						warnings.set(ContentWarnings::IdAttr);
					}
					else if k.eq_ignore_ascii_case("class") {
						warnings.set(ContentWarnings::ClassAttr);
					}
					else if k.eq_ignore_ascii_case("style") {
						warnings.set(ContentWarnings::InlineStyle);
					}
					else if k.starts_with("on") {
						warnings.set(ContentWarnings::InlineScript);
					}
				}
			}
		}
	}
	warnings
}

/// # Parse SVG into Symbol.
///
/// This parses and somewhat validates an input SVG, returning it as a `Symbol`
/// suitable for inclusion in the map.
fn parse_as_symbol(path: &Path, stem: &str, prefix: &str)
-> Result<(Symbol, ContentWarnings), SvgError> {
	// Load the SVG. We'll do this as bytes for now.
	let raw: String = std::fs::read_to_string(path)
		.map_err(|_| SvgError::Read(path.to_path_buf()))?;

	// Find the start and end ranges.
	let (start, end) = ranges(raw.as_bytes()).ok_or_else(|| SvgError::Parse(path.to_path_buf()))?;

	// Parse it.
	let mut events: Vec<Event> = Vec::new();
	for event in Parser::new(&raw[start..end]) {
		match event {
			Event::Error(_) => return Err(SvgError::Parse(path.to_path_buf())),
			Event::Tag(_, _, _) | Event::Text(_) => { events.push(event); },
			_ => {},
		}
	}

	// The last event should be a closing SVG tag.
	match events.pop() {
		Some(Event::Tag(s, Type::End, _)) if s.eq_ignore_ascii_case("svg") => {},
		_ => return Err(SvgError::Parse(path.to_path_buf())),
	}

	// Grab the main element.
	events.reverse();
	let mut out = parse_main(events.pop(), path)?
		.set("id", format!("{prefix}-{stem}"));

	// Check for styles, classes, and IDs that may cause issues.
	let warn = has_styles(&events);

	// Append the children.
	while ! events.is_empty() {
		let next = parse_flat(&mut events)
			.ok_or_else(|| SvgError::Parse(path.to_path_buf()))?;
		if ! is_empty(&next) {
			out.append(next);
		}
	}

	Ok((out, warn))
}

/// # Flatten Next Element.
///
/// This returns the next element, recursing as necessary to capture all its
/// children.
fn parse_flat(events: &mut Vec<Event>) -> Option<Element> {
	let next = events.pop()?;
	match next {
		// It already is flat!
		Event::Tag(name, Type::Empty, attrs) => {
			let mut out = Element::new(name.to_ascii_lowercase());
			for (k, v) in attrs {
				out.assign(k, v);
			}
			Some(out)
		},
		Event::Tag(name, Type::Start, attrs) =>
			parse_flat2(name.to_ascii_lowercase(), attrs, events),
		_ => None,
	}
}

/// # Flatten Open Tag.
///
/// This builds a flat element beginning from its opening tag and ending with
/// its closing tag.
///
/// If the closing tag is missing, `None` is returned.
fn parse_flat2(mut name: String, attrs: Attributes, events: &mut Vec<Event>) -> Option<Element> {
	name.make_ascii_lowercase();
	let mut out = Element::new(&name);
	for (k, v) in attrs {
		out.assign(k, v);
	}

	let mut closed = false;
	while let Some(event) = events.pop() {
		match event {
			// We found the end!
			Event::Tag(s, Type::End, _) if s.eq_ignore_ascii_case(&name) => {
				closed = true;
				break;
			},
			// Text just gets added.
			Event::Text(s) => {
				let s = s.trim();
				if ! s.is_empty() {
					out.append(svg::node::Text::new(s));
				}
			},
			// Such tags are only one level deep.
			Event::Tag(s, Type::Empty, attrs) => {
				let mut tmp = Element::new(s.to_ascii_lowercase());
				for (k, v) in attrs {
					tmp.assign(k, v);
				}
				if ! is_empty(&tmp) { out.append(tmp); }
			},
			// Recurse.
			Event::Tag(s, Type::Start, attrs) =>
				if let Some(tmp) = parse_flat2(s.to_ascii_lowercase(), attrs, events) {
					if ! is_empty(&tmp) { out.append(tmp); }
				},
			_ => {},
		}
	}

	if closed { Some(out) }
	else { None }
}

/// # Parse Main.
///
/// This parses the outer SVG element, ensuring it has a `viewBox`. If it
/// doesn't, `None` is returned.
fn parse_main(event: Option<Event>, path: &Path) -> Result<Symbol, SvgError> {
	if let Some(Event::Tag(s, Type::Start, a)) = event {
		if s.eq_ignore_ascii_case("svg") {
			let mut out = Symbol::new();

			// Do we have a viewbox?
			if let Some(vb) = a.get("viewBox").or_else(|| a.get("viewbox")).or_else(|| a.get("VIEWBOX")) {
				out = out.set("viewBox", vb.clone());
			}
			else {
				let vb = parse_wh(
					a.get("width").or_else(|| a.get("WIDTH")),
					a.get("height").or_else(|| a.get("HEIGHT")),
				)
					.ok_or_else(|| SvgError::Viewbox(path.to_path_buf()))?;

				out = out.set("viewBox", vb);
			}

			return Ok(out);
		}
	}

	Err(SvgError::Parse(path.to_path_buf()))
}

/// # Path Stem to ID.
///
/// Take the ASCII alphanumeric and `-` characters from the file stem and
/// return them for use as an ID suffix.
fn parse_stem_id(path: &Path) -> Option<Cow<'_, str>> {
	let mut out = path.file_stem()?.to_string_lossy();

	// Reduce to alphanumeric and -.
	if out.chars().any(|c| c != '-' && ! c.is_ascii_alphanumeric()) {
		out.to_mut().retain(|c: char| c == '-' || c.is_ascii_alphanumeric());
	}

	// Return it if we got it.
	if out.is_empty() { None }
	else { Some(out) }
}

/// # Parse Width/Height.
///
/// This attempts to build a `viewBox` value from a `width` and `height`,
/// returning `None` if either is missing or non-positive.
fn parse_wh(w: Option<&Value>, h: Option<&Value>) -> Option<String> {
	let w: &str = w?.trim_matches(|c: char| ! matches!(c, '0'..='9' | '.' | '-'));
	let h: &str = h?.trim_matches(|c: char| ! matches!(c, '0'..='9' | '.' | '-'));

	// If they're both positive, we're good.
	if 0.0 < w.parse::<f32>().ok()? && 0.0 < h.parse::<f32>().ok()? {
		let mut out = String::with_capacity(5 + w.len() + h.len());
		out.push_str("0 0 ");
		out.push_str(w);
		out.push(' ');
		out.push_str(h);
		Some(out)
	}
	else { None }
}

/// # Find Range.
///
/// This returns the start byte for the first opening SVG tag and the end byte
/// of the last closing SVG tag.
///
/// There are a few gotchas to be aware of:
/// * The opening SVG element must have at least one attribute (`<svg>` is not allowed);
/// * Closing tags cannot have any whitespace (they must be exactly `</svg>`);
/// * Opening tag attributes cannot include unescaped `>` characters;
/// * Opening tags must precede closing tags;
/// * There must be an equal number of opening and closing tags;
///
/// If any of the above fail, or an open or close cannot be found, `None` is
/// returned.
fn ranges(src: &[u8]) -> Option<(usize, usize)> {
	/// # Opening Marker.
	const OPEN: &[u8] = b"<svg ";

	/// # Closing Marker.
	const CLOSE: &[u8] = b"</svg>";

	let mut opens: u8 = 0;
	let mut closes: u8 = 0;

	let mut start_a: usize = 0;
	let mut end_a: usize = 0;

	// Use a window of 6 to capture the full closure.
	for (idx, chunk) in src.windows(6).enumerate() {
		if chunk[0] == b'<' {
			// It's an end!
			if chunk.eq_ignore_ascii_case(CLOSE) {
				// Can't close until we've opened!
				if closes == opens { return None; }
				closes += 1;
				end_a = idx + 6;
			}
			// It's a beginning!
			else if chunk[..5].eq_ignore_ascii_case(OPEN) {
				if opens == 0 {
					start_a = idx;
				}
				opens += 1;
			}
		}
	}

	// We have to have the same number of opens and closes.
	if 0 < opens && opens == closes {
		Some((start_a, end_a))
	}
	else { None }
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_hiddentrue() {
		let mut raw = r#"<div hidden="true"></div>"#.to_owned();
		if let Some(pos) = raw.find(r#" hidden="true""#) {
			raw.replace_range(pos+7..pos+14, "");
		}
		assert_eq!(raw, "<div hidden></div>");
	}

	#[test]
	fn test_parse_stem_id() {
		for (raw, expected, borrowed) in [
			("image.svg", Some("image"), true),
			("image name.svg", Some("imagename"), false),
			("ImAgE.svg", Some("ImAgE"), true),
			("__.svg", None, true),
		] {
			if let Some(expected) = expected {
				let Some(raw2) = parse_stem_id(raw.as_ref()) else {
					panic!("BUG: unable to parse stem/id from {raw:?}");
				};
				assert_eq!(raw2.as_ref(), expected);
				assert_eq!(
					matches!(raw2, Cow::Borrowed(_)),
					borrowed,
					"BUG: expected {raw:?} borrow to be {borrowed}",
				);
			}
			else {
				assert!(
					parse_stem_id(raw.as_ref()).is_none(),
					"BUG: shouldn't have parsed stem/id from {raw:?}",
				);
			}
		}
	}

	#[test]
	#[expect(clippy::type_complexity, reason = "It is what it is.")]
	fn test_ranges() {
		let tests: [(&[u8], Option<(usize, usize)>); 4] = [
			(include_bytes!("../test-assets/close.svg"), Some((0, 287))),
			(b"<svg id=foo><svg id=bar></svg></svg>", Some((0, 36))),
			(b"    <SVG id=foo><svg id=bar></svg></svg>", Some((4, 40))),
			(b"<svg id=foo><svg id=bar></svg>", None),
		];

		for (src, expected) in tests {
			assert_eq!(ranges(src), expected);
		}
	}

	#[test]
	fn test_wh() {
		let tests: [(Option<Value>, Option<Value>, Option<String>); 6] = [
			(
				Some(Value::from(String::from("444.819"))),
				Some(Value::from(String::from("280.371"))),
				Some(String::from("0 0 444.819 280.371")),
			),
			(
				Some(Value::from(String::from("444.819px"))),
				Some(Value::from(String::from("280.371%"))),
				Some(String::from("0 0 444.819 280.371")),
			),
			(
				None, // Missing width.
				Some(Value::from(String::from("280.371"))),
				None,
			),
			(
				Some(Value::from(String::from("0"))), // Zero width.
				Some(Value::from(String::from("280.371"))),
				None,
			),
			(
				Some(Value::from(String::from("apples"))), // Bunk width.
				Some(Value::from(String::from("280.371"))),
				None,
			),
			(
				Some(Value::from(String::from("444.819"))),
				Some(Value::from(String::from("-280.371"))), // Negative height.
				None,
			),
		];

		for (w, h, ex) in tests {
			assert_eq!(parse_wh(w.as_ref(), h.as_ref()), ex);
		}
	}

	fn has_styles_wrapper(raw: &str) -> ContentWarnings {
		// Find the start and end ranges.
		let (start, end) = ranges(raw.as_bytes())
			.expect("Failed to parse SVG.");

		// Parse it.
		let mut events: Vec<Event> = Vec::new();
		for event in Parser::new(&raw[start..end]) {
			match event {
				Event::Error(_) => panic!("Failed to parse SVG."),
				Event::Tag(_, _, _) | Event::Text(_) => { events.push(event); },
				_ => {},
			}
		}

		// The last event should be a closing SVG tag.
		match events.pop() {
			Some(Event::Tag(s, Type::End, _)) if s.eq_ignore_ascii_case("svg") => {},
			_ => panic!("Failed to parse SVG."),
		}

		// Grab the main element.
		events.reverse();
		events.pop();

		// Actually check the styles!
		has_styles(&events)
	}

	#[test]
	fn test_styles() {
		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/arrow-1.svg")),
			ContentWarnings::StyleTag | ContentWarnings::ClassAttr,
			"Missed styles for arrow-1.svg."
		);

		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/arrow-2.svg")),
			ContentWarnings::ClassAttr,
			"Missed styles for arrow-2.svg."
		);

		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/arrow-3.svg")),
			ContentWarnings::IdAttr,
			"Missed styles for arrow-3.svg."
		);

		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/bitcoin.svg")),
			ContentWarnings::None,
			"False positive styles for bitcoin.svg."
		);
	}
}
