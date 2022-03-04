/*!
# Yesvgmap: Boundaries
*/

use crate::SvgError;
use fyi_msg::Msg;
use std::{
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



#[derive(Debug, Clone)]
/// # SVG Map.
///
/// This holds an `svg` element with some number of `symbol` children.
pub(super) struct Map {
	inner: SVG,
	len: usize,
}

impl fmt::Display for Map {
	/// # To String.
	///
	/// This emits SVG code, slightly compressed.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(
			&self.inner.to_string()
				.replace(" hidden=\"true\"", " hidden")
				.replace(">\n<", "><")
		)
	}
}

impl Map {
	/// # New.
	pub(super) fn new(
		id: Option<&str>,
		class: Option<&str>,
		hide: HideType,
		prefix: &str,
		paths: Vec<PathBuf>
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
		let mut warned: Vec<String> = Vec::new();
		let len: usize = paths.len();
		let mut nice_paths: Vec<(String, Symbol)> = Vec::with_capacity(len);
		for path in paths {
			// The symbol ID is built from the alphanumeric (and dash)
			// characters in the file name.
			let stem: String = path.file_stem()
				.ok_or_else(|| SvgError::Read(path.clone()))?
				.to_string_lossy()
				.chars()
				.filter(|x| x.is_ascii_alphanumeric() || '-'.eq(x))
				.collect();

			// Build up the symbol.
			let (s, warn) = parse_as_symbol(&path, &stem, prefix)?;

			// Push it to temporary storage.
			nice_paths.push((stem, s));

			// Note if this has styles or other issues.
			if warn {
				warned.push(path.file_name().unwrap().to_string_lossy().into_owned());
			}
		}

		// Sort and dedup by stem.
		nice_paths.sort_by(|a, b| a.0.cmp(&b.0));
		nice_paths.dedup_by(|a, b| a.0 == b.0);

		// If the length changed, there are duplicates.
		if nice_paths.len() != len {
			return Err(SvgError::Duplicate);
		}

		// Mention any potential style/class issues.
		if ! warned.is_empty() {
			Msg::warning("The following SVG(s) contain styles, classes, and/or IDs that might not work
correctly when embedded in a sprite map. If you experience issues, remove those
elements from the source(s), then regenerate the map.")
				.print();

			warned.sort();
			for w in warned {
				println!("    \x1b[1;95m•\x1b[0m {}", w);
			}

			println!();
		}

		// Done!
		Ok(Self {
			// We can add the children on-the-fly.
			inner: nice_paths.into_iter().fold(map, |m, (_, s)| m.add(s)),
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
	None,
	Hidden,
	Offscreen,
}



/// # Parse SVG into Symbol.
///
/// This parses and somewhat validates an input SVG, returning it as a `Symbol`
/// suitable for inclusion in the map.
fn parse_as_symbol(path: &Path, stem: &str, prefix: &str)
-> Result<(Symbol, bool), SvgError> {
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
		.set("id", format!("{}-{}", prefix, stem));

	// Check for styles, classes, and IDs that may cause issues.
	let warn = has_styles(&events);

	// Append the children.
	while ! events.is_empty() {
		let next = parse_flat(&mut events)
			.ok_or_else(|| SvgError::Parse(path.to_path_buf()))?;
		out.append(next);
	}

	Ok((out, warn))
}

/// # Check for Styles, Classes, IDs.
///
/// Styles, classes, and IDs inside of SVGs have a habit of colliding with one
/// another, particularly in map contexts. This method looks to see if there
/// are any so we can issue a warning.
fn has_styles(src: &[Event]) -> bool {
	src.iter()
		.filter_map(|e|
			if let Event::Tag(name, _, attrs) = e { Some((name, attrs)) }
			else { None }
		)
		.any(|(name, attrs)|
			name.eq_ignore_ascii_case("style") ||
			attrs.keys().any(|k| k.eq_ignore_ascii_case("id") || k.eq_ignore_ascii_case("class"))
		)
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
			return Some(out);
		},
		Event::Tag(name, Type::Start, attrs) => {
			return parse_flat2(name.to_ascii_lowercase(), attrs, events);
		},
		_ => {},
	}

	None
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
				out.append(svg::node::Text::new(s));
			},
			// Such tags are only one level deep.
			Event::Tag(s, Type::Empty, attrs) => {
				let mut tmp = Element::new(s.to_ascii_lowercase());
				for (k, v) in attrs {
					tmp.assign(k, v);
				}
				out.append(tmp);
			},
			// Recurse.
			Event::Tag(s, Type::Start, attrs) => {
				if let Some(tmp) = parse_flat2(s.to_ascii_lowercase(), attrs, events) {
					out.append(tmp);
				}
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

/// # Parse Width/Height.
///
/// This attempts to build a `viewBox` value from a `width` and `height`,
/// returning `None` if either is missing or non-positive.
fn parse_wh(w1: Option<&Value>, h1: Option<&Value>) -> Option<String> {
	let w1 = w1?.to_string();
	let h1 = h1?.to_string();

	let w2 = w1.trim_matches(|c: char| ! matches!(c, '0'..='9' | '.' | '-'));
	let h2 = h1.trim_matches(|c: char| ! matches!(c, '0'..='9' | '.' | '-'));

	let w3 = w2.parse::<f32>().ok()?;
	let h3 = h2.parse::<f32>().ok()?;

	// If they're both positive, we're good.
	if 0.0 < w3 && 0.0 < h3 {
		Some(["0 0 ", w2, " ", h2].concat())
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
	const OPEN: &[u8] = b"<svg ";
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

	fn has_styles_wrapper(raw: &str) -> bool {
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
			true,
			"Missed styles for arrow-1.svg."
		);

		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/arrow-2.svg")),
			true,
			"Missed styles for arrow-2.svg."
		);

		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/arrow-3.svg")),
			true,
			"Missed styles for arrow-3.svg."
		);

		assert_eq!(
			has_styles_wrapper(include_str!("../test-assets/bitcoin.svg")),
			false,
			"False positive styles for bitcoin.svg."
		);
	}
}
