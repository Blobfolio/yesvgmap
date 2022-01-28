/*!
# Yesvgmap: Boundaries
*/

use crate::SvgError;
use once_cell::sync::Lazy;
use regex::bytes::Regex;
use std::{
	ffi::OsStr,
	path::Path,
};



/// # Parse SVG.
///
/// This parses a standalone SVG file and converts it into a `<symbol>` that
/// can be stuffed into our map.
pub(super) fn parse(path: &Path, prefix: &str) -> Result<String, SvgError> {
	// We'll need the file name eventually. It's a lightweight query; might as
	// well get it out of the way and fail early if necessary.
	let stem: &str = path.file_stem()
		.and_then(OsStr::to_str)
		.ok_or_else(|| SvgError::Read(path.to_path_buf()))?;

	// Load the SVG. We'll do this as bytes for now.
	let mut svg: Vec<u8> = std::fs::read(path)
		.map_err(|_| SvgError::Read(path.to_path_buf()))?;

	// Find the start and end ranges.
	let (start_a, start_b, end) = ranges(&svg).ok_or_else(|| SvgError::Parse(path.to_path_buf()))?;

	// Find the viewBox.
	let vb = viewbox(&svg[start_a..start_b])
		.ok_or_else(|| SvgError::Viewbox(path.to_path_buf()))?;

	// Go ahead and chop off the end.
	svg.truncate(end);

	// Turn it into a string.
	let mut svg = String::from_utf8(svg).map_err(|_| SvgError::Parse(path.to_path_buf()))?;

	// Replace the start with a symbol.
	svg.replace_range(
		start_a..start_b,
		&format!(
			r#"<symbol id="{}-{}" viewBox="{}">"#,
			prefix,
			stem,
			vb,
		)
	);

	// Add the closing tag.
	svg.push_str("</symbol>");

	// Done!
	Ok(svg)
}

/// # Find Ranges.
///
/// This finds the start and end indexes of the first opening tag, and the
/// start index of the last closing tag.
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
fn ranges(src: &[u8]) -> Option<(usize, usize, usize)> {
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
				end_a = idx;
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
		// We have to find the (non-inclusive) end of the opening tag.
		let start_b: usize = src.iter().skip(start_a + 5).position(|b| b'>'.eq(b))? + 6 + start_a;
		if start_b < end_a {
			return Some((start_a, start_b, end_a));
		}
	}

	None
}

/// # Parse or Build Viewbox.
///
/// All SVGs in the map must have `viewBox` coordinates. If they're already
/// present, great!, if not, we'll try to build them from `width`/`height`.
fn viewbox(src: &[u8]) -> Option<String> {
	_viewbox(src).or_else(|| _dimensions(src))
}

/// # Parse Viewbox.
///
/// This parses an existing `viewBox` attribute, if any.
fn _viewbox(src: &[u8]) -> Option<String> {
	static VB: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)viewbox\s*=\s*('|")\s*([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s*('|")"#).unwrap());

	// Direct hit!
	let caps = VB.captures(src)?;
	let mut out = String::new();

	for idx in 2..=5 {
		let raw = caps.get(idx)
			.and_then(|m| std::str::from_utf8(&src[m.start()..m.end()]).ok())?;

		// Make sure this is a valid float.
		let parsed: f32 = raw.parse::<f32>().ok()?;
		if 0.0 < parsed || idx < 4 {
			if ! out.is_empty() { out.push(' '); }
			out.push_str(raw);
		}
		else { return None; }
	}

	Some(out)
}

/// # Build Viewbox.
///
/// This attempts to build a `viewBox` using an existing `width` and `height`.
/// Both values must be present and greater than zero to be accepted. They
/// shouldn't have units, but if they do, the units will be stripped off.
fn _dimensions(src: &[u8]) -> Option<String> {
	static WH: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)(?P<key>(width|height))\s*=\s*('|")?\s*(?P<value>[\d.]+)[\sa-z%]*('|")?"#).unwrap());

	let mut width: String = String::new();
	let mut height: String = String::new();

	for caps in WH.captures_iter(src) {
		let key = caps["key"].to_ascii_lowercase();
		if key == b"width" {
			width = String::from_utf8(caps["value"].to_vec()).ok()?;
			if width.parse::<f32>().ok()? <= 0.0 { return None; }
		}
		else if key == b"height" {
			height = String::from_utf8(caps["value"].to_vec()).ok()?;
			if height.parse::<f32>().ok()? <= 0.0 { return None; }
		}
	}

	if width.is_empty() || height.is_empty() { None }
	else {
		Some(["0 0 ", &width, " ", &height].concat())
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_bounds() {
		let tests: [(&[u8], Option<(usize, usize, usize)>); 4] = [
			(include_bytes!("../test-assets/close.svg"), Some((0, 95, 281))),
			(b"<svg id=foo><svg id=bar></svg></svg>", Some((0, 12, 30))),
			(b"    <SVG id=foo><svg id=bar></svg></svg>", Some((4, 16, 34))),
			(b"<svg id=foo><svg id=bar></svg>", None),
		];

		for (src, expected) in tests {
			assert_eq!(ranges(src), expected);
		}
	}

	#[test]
	fn test_dims() {
		let tests: [(&[u8], Option<String>, Option<String>); 3] = [
			(
				br#"<svg xmlns="http://www.w3.org/2000/svg" width="444.819" height="280.371" viewBox="0 0 444.819 280.371">"#,
				Some(String::from("0 0 444.819 280.371")),
				Some(String::from("0 0 444.819 280.371")),
			),
			(
				br#"<svg xmlns="http://www.w3.org/2000/svg" width="444.819em" height="280.371%" viewBox="0 0 444.819 -280.371">"#,
				None,
				Some(String::from("0 0 444.819 280.371")),
			),
			(
				br#"<svg xmlns="http://www.w3.org/2000/svg" height="280.371" viewBox="0 0 444.819 280.371">"#,
				Some(String::from("0 0 444.819 280.371")),
				None,
			),
		];

		for (src, ex_vb, ex_wh) in tests {
			assert_eq!(_viewbox(src), ex_vb);
			assert_eq!(_dimensions(src), ex_wh);
		}
	}
}
