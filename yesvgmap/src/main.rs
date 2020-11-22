/*!
# `Yesvgmap`
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::map_err_ignore)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]

use fyi_menu::{
	Argue,
	FLAG_REQUIRED,
};
use fyi_msg::MsgKind;
use fyi_witcher::Witcher;
use regex::Regex;
use std::{
	borrow::Cow,
	ffi::OsStr,
	io::Write,
	ops::Range,
	path::PathBuf,
};



/// Main.
fn main() {
	// Parse CLI arguments.
	let mut args = Argue::new(FLAG_REQUIRED)
		.with_version(b"Yesvgmap", env!("CARGO_PKG_VERSION").as_bytes())
		.with_help(helper)
		.with_list();

	// Make sure the output path is defined before we do any hard work.
	let out: Option<PathBuf> = args.option2("-o", "--output")
		.map(PathBuf::from)
		.filter(|p| ! p.is_dir());

	// The ID prefix.
	let prefix: String = args.option2("-p", "--prefix").unwrap_or("i").to_string();

	// Start putting together the map's opening tag.
	let mut map: String = String::from(r#"<svg xmlns="http://www.w3.org/2000/svg" aria-hidden"#);
	if let Some(c) = args.option("--map-class") {
		map.push_str(r#" class=""#);
		map.push_str(c);
		map.push('"');
	}
	else {
		map.push_str(r#" style="position: fixed; top: 0; left: -100px; width: 1px; height: 1px; overflow: hidden;""#)
	}
	if let Some(i) = args.option("--map-id") {
		map.push_str(r#" id=""#);
		map.push_str(i);
		map.push('"');
	}
	map.push('>');

	// Run through the files.
	let mut guts: Vec<String> = Witcher::default()
		.with_ext(b".svg")
		.with_paths(args.args())
		.build()
		.iter()
		.filter_map(|p| svg_to_symbol(p, &prefix))
		.collect();

	if guts.is_empty() {
		die("No SVGs were found for the map.");
	}

	guts.sort();
	map.push_str(&guts.concat());
	map.push_str("</svg>\n");

	// Try to save it.
	if let Some(path) = out {
		tempfile_fast::Sponge::new_for(&path)
			.and_then(|mut file| file.write_all(map.as_bytes()).and_then(|_| file.commit()))
			.unwrap_or_else(|_| {
				die("Unable to save output file.");
				unreachable!();
			});

		MsgKind::Success.into_msg(&format!(
			"A sprite with {} images has been saved to {:?}",
			guts.len(),
			std::fs::canonicalize(&path).unwrap()
		)).println();
	}
	else {
		let writer = std::io::stdout();
		let mut handle = writer.lock();
		let _ = handle.write_all(map.as_bytes())
			.and_then(|_| handle.flush());
	}
}

/// SVG to Symbol.
///
/// This beastly function tries to tease out the `<svg>...</svg>` bits from the
/// raw file contents. If that works, it then looks to see if it can find or
/// calculate a viewbox value for it. Then it returns everything as a
/// `<symbol>...</symbol>` for later map embedding.
fn svg_to_symbol(path: &PathBuf, prefix: &str) -> Option<String> {
	if let Some((svg, stem)) = std::fs::read_to_string(path)
		.ok()
		.zip(path.file_stem().and_then(OsStr::to_str))
	{
		if let Some((open, close)) = svg_bounds(&svg) {
			return Some(
				if let Some(vb) = svg_viewbox(&svg[open.start..open.end]) {
					format!(
						r#"<symbol id="{}-{}" viewBox="{}">{}</symbol>"#,
						prefix,
						stem,
						vb,
						&svg[open.end..close.start]
					)
				}
				else {
					format!(
						r#"<symbol id="{}-{}">{}</symbol>"#,
						prefix,
						stem,
						&svg[open.end..close.start]
					)
				}
			);
		}
	}

	None
}

/// SVG Tag Boundaries
///
/// Find the range of the opening and closing tags of an SVG. A positive return
/// value only exists when both exist.
fn svg_bounds(raw: &str) -> Option<(Range<usize>, Range<usize>)> {
	lazy_static::lazy_static! {
		static ref OPEN: Regex = Regex::new(r#"(?i)<svg(\s+[^>]+)?>"#).unwrap();
		static ref CLOSE: Regex = Regex::new(r"(?i)</svg>").unwrap();
	}

	OPEN.find(raw)
		.map(|m| m.start()..m.end())
		.zip(CLOSE.find(raw).map(|m| m.start()..m.end()))
		.filter(|(s,e)| e.start > s.end)
}

/// SVG Tag Attributes
///
/// Parse the tag attributes, returning a viewbox if possible.
fn svg_viewbox(raw: &str) -> Option<Cow<str>> {
	lazy_static::lazy_static! {
		static ref VB: Regex = Regex::new(r#"(?i)viewbox\s*=\s*('|")([\d. ]+\s+[\d. ]+\s+[\d. ]+\s+[\d. ]+)('|")"#).unwrap();
		static ref WH: Regex = Regex::new(r#"(?i)(?P<key>(width|height))\s*=\s*('|")?(?P<value>[a-z\d. ]+)('|")?"#).unwrap();
	}

	// Direct hit!
	if let Some(m) = VB.captures(raw).and_then(|m| m.get(2)) {
		return Some(Cow::Borrowed(&raw[m.start()..m.end()]));
	}

	// Build the width and height manually.
	let mut width: Option<f64> = None;
	let mut height: Option<f64> = None;

	// Find the matches.
	for caps in WH.captures_iter(raw) {
		let key = caps["key"].to_lowercase();
		if key == "width" {
			width = parse_attr_size(&caps["value"]);
		}
		else if key == "height" {
			height = parse_attr_size(&caps["value"]);
		}
	}

	width.zip(height).map(|(w,h)| Cow::Owned(format!("0 0 {} {}", w, h)))
}

/// Parse Width/Height
///
/// Attribute widths and heights might have units or other garbage that would
/// interfere with straight float conversion.
fn parse_attr_size(value: &str) -> Option<f64> {
	value.parse::<f64>()
		.or_else(|_|
			value.chars()
				.take_while(|c| c.is_numeric() || c == &'.' || c == &'-')
				.collect::<String>()
				.parse::<f64>()
		)
		.ok()
		.filter(|&x| x > 0.0)
}

/// # Error and Exit.
///
/// This prints a formatted error message and exists the program with a status
/// code of `1`.
pub fn die<S>(error: S)
where S: AsRef<str> {
	MsgKind::Error.into_msg(error.as_ref()).eprintln();
	std::process::exit(1);
}

#[cold]
/// Print Help.
fn helper(_: Option<&str>) {
	use fyi_msg::Msg;
	Msg::from(format!(
		r#"
      .--.   _,
  .--;    \ /(_
 /    '.   |   '-._    . ' .
|       \  \    ,-.)  -= * =-
 \ /\_   '. \((` .(    '/. '
  )\ /     \ )\  _/   _/
 /  \\    .-'   '--. /_\
|    \\_.' ,        \/||
\     \_.-';,_) _)'\ \||
 '.       /`\   (   '._/
   `\   .;  |  . '.
     ).'  )/|      \
     `    ` |  \|   |  {}{}{}
             \  |   |  SVG sprite generator.
              '.|   |
                 \  '\__
                  `-._  '. _
                     \`;-.` `._
                      \ \ `'-._\
                       \ |
                        \ )
                         \_\

USAGE:
	yesvgmap [FLAGS] [OPTIONS] <PATH(S)>

FLAGS:
	-h, --help                  Prints help information.
	-V, --version               Prints version information.

OPTIONS:
    -l, --list <FILE>           Read file paths from this list.
        --map-class <CLASS>     A class attribute value to assign to the map
                                itself. [default: ]
        --map-id <ID>           An ID attribute value to assign to the map
                                itself. [default: ]
    -o, --output <PATH>         A file path to save the generated map to. If
                                not specified, the map will print to STDOUT.

ARGS:
    <PATH(S)>...                One or more files or directories to crunch and
                                crawl.

"#,
		"\x1b[38;5;199mYesvgmap\x1b[0;38;5;69m v",
		env!("CARGO_PKG_VERSION"),
		"\x1b[0m",
	)).print();
}
