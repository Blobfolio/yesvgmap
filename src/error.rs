/*!
# Yesvgmap: Errors
*/

use fyi_msg::{
	fyi_ansi::{
		ansi,
		csi,
		dim,
	},
	Msg,
};
use std::{
	error,
	ffi::OsString,
	fmt,
};



/// # Help Text.
///
/// Note that this is intentionally missing the deprecated arguments.
const HELP: &str = concat!(r#"
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
     `    ` |  \|   |  "#, csi!(199), "Yesvgmap", ansi!((cornflower_blue) " v", env!("CARGO_PKG_VERSION")), r#"
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
    -h, --help                  Print help information and exit.
    -V, --version               Print version information and exit.

OPTIONS:
    -a, --attribute <KEY[=VAL]> Add an attribute — id, class, etc. — to the
                                top-level <svg> element.
    -l, --list <PATH>           Read (absolute) file and/or directory paths
                                from this text file — or STDIN if "-" — one
                                entry per line, instead of or in addition to
                                any trailing <PATH(S)>.
    -o, --output <PATH>         Save the generated map to this location. If
                                omitted, the map will print to STDOUT instead.
    -p, --prefix <STRING>       Set a custom prefix for the IDs of each entry
                                in the map. (IDs look like PREFIX-STEM, where
                                STEM is the alphanumeric portion of the source
                                file name, e.g. "i-close".) [default: i]

ARGS:
    <PATH(S)>...                One or more file and/or directory paths to
                                crunch and/or (recursively) crawl. Only files
                                with the extension .svg will ultimately be
                                included.
"#);



// # Generated by build.rs.
include!(concat!(env!("OUT_DIR"), "/content-warnings.rs"));

impl fmt::Display for ContentWarnings {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.is_none() { return Ok(()); }

		// Print the tags first, if we have any.
		let mut any = self.contains(Self::StyleTag);
		if any {
			f.write_str(concat!(ansi!((bold, magenta) "<style>"), " tags"))?;
		}

		// Now do the same for the attributes.
		if self.contains_any(Self::Attributes).is_some() {
			// Print a joiner if we had tag problems.
			if any { f.write_str(" and ")?; }

			// Now each issue.
			any = false;
			for (k, v) in [
				(Self::ClassAttr, ansi!((bold, cyan) "class")),
				(Self::IdAttr, ansi!((bold, cyan) "id")),
				(Self::StyleAttr, ansi!((bold, cyan) "style")),
			] {
				if self.contains(k) {
					if any { f.write_str("/")?; }
					else { any = true; }
					f.write_str(v)?;
				}
			}

			// And a label.
			f.write_str(" attributes")?;
		}

		// Done!
		Ok(())
	}
}



#[derive(Debug, Clone, Eq, PartialEq)]
/// # Error (Maybe w/ Details).
pub(crate) struct SvgError {
	/// # Error Kind.
	kind: SvgErrorKind,

	/// # Details.
	details: Option<OsString>,
}

impl fmt::Display for SvgError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.kind.as_str())?;
		self.details.as_ref().map_or(Ok(()), |details| write!(
			f,
			dim!(" {details}"),
			details=details.to_string_lossy(),
		))
	}
}

impl error::Error for SvgError {}

impl From<SvgErrorKind> for SvgError {
	#[inline]
	fn from(kind: SvgErrorKind) -> Self {
		Self { kind, details: None }
	}
}

impl<D: Into<OsString>> From<(SvgErrorKind, D)> for SvgError {
	#[inline]
	fn from((kind, details): (SvgErrorKind, D)) -> Self {
		Self { kind, details: Some(details.into()) }
	}
}

impl From<SvgError> for Msg {
	#[inline]
	fn from(err: SvgError) -> Self {
		let SvgError { kind, details } = err;
		let mut msg = Self::error(kind.as_str());
		if let Some(d) = details {
			let d = d.to_string_lossy();

			// Add details inline or not depending on length.
			let pad: &str =
				if fyi_msg::width(msg.as_str()) + d.len() + 1 < 80 { " " }
				else { "\n       " };

			msg.set_suffix(format!(
				dim!("{pad}{d}"),
				pad=pad,
				d=d,
			));
		}
		msg
	}
}

impl SvgError {
	/// # Error Kind.
	pub(crate) const fn kind(&self) -> SvgErrorKind { self.kind }
}



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Error Kind.
///
/// This `Copy`-friendly enum is used to handle the generic parts of an error.
/// Think [`std::io::ErrorKind`] or similar.
pub(crate) enum SvgErrorKind {
	/// # Deprecated (and recently removed): `--hidden`.
	DeprecatedHidden,

	/// # Deprecated (and recently removed): `--offscreen`.
	DeprecatedOffscreen,

	/// # Deprecated (and recently removed): `--map-class`.
	DeprecatedMapClass,

	/// # Deprecated (and recently removed): `--map-id`.
	DeprecatedMapId,

	/// # Duplicate Attribute.
	DupeAttribute,

	/// # Duplicate File Name.
	DupeFileName,

	/// # Duplicate ID.
	DupeId,

	/// # Invalid Attribute.
	InvalidAttribute,

	/// # Invalid CLI Arg.
	InvalidCli,

	/// # Invalid Output Path.
	InvalidDst,

	/// # Invalid File Name.
	InvalidFileName,

	/// # Invalid Prefix.
	InvalidPrefix,

	/// # No SVGs.
	NoSvgs,

	/// # File Does Not Start w/ SVG Tag.
	ParseStart,

	/// # File Does Not End w/ SVG Tag.
	ParseEnd,

	/// # File Contains Javascript.
	ParseScript,

	/// # Multiple SVG Tags.
	ParseSvgSvg,

	/// # Malformed Viewbox.
	ParseViewBox,

	/// # Viewbox Has Non-Zero Offset.
	ParseViewBoxOffset,

	/// # Missing Viewport Attributes.
	ParseViewportAttr,

	/// # Non-Positive Viewport Size.
	ParseViewportSize,

	/// # General Parsing Error.
	Parse,

	/// # Read Error.
	Read,

	/// # Write Error.
	Write,

	/// # Print Help (Not an Error).
	PrintHelp,

	/// # Print Version (Not an Error).
	PrintVersion,
}

impl fmt::Display for SvgErrorKind {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl error::Error for SvgErrorKind {}

impl SvgErrorKind {
	/// # As String Slice.
	pub(crate) const fn as_str(self) -> &'static str {
		match self {
			Self::DeprecatedHidden => concat!(
				dim!("--hidden"),
				" has been mooted; sprite maps just use ",
				dim!(
					csi!(light_cyan), "style",
					csi!(dark_orange), "=",
					csi!(light_yellow), "\"",
					csi!(!fg), "display:none",
					csi!(light_yellow), "\"",
				),
				" now.",
			),
			Self::DeprecatedOffscreen => concat!(
				dim!("--offscreen"),
				" has been mooted; sprite maps just use ",
				dim!(
					csi!(light_cyan), "style",
					csi!(dark_orange), "=",
					csi!(light_yellow), "\"",
					csi!(!fg), "display:none",
					csi!(light_yellow), "\"",
				),
				" now.",
			),
			Self::DeprecatedMapClass => concat!(
				dim!("--map-class"),
				" has been removed; use ",
				dim!("-a class=XXX"),
				" instead.",
			),
			Self::DeprecatedMapId => concat!(
				dim!("--map-id"),
				" has been removed; use ",
				dim!("-a id=XXX"),
				" instead.",
			),
			Self::DupeAttribute => "Sprite attributes must be unique.",
			Self::DupeFileName => "File names must be unique.",
			Self::DupeId => concat!("Element ", ansi!((bold, cyan) "id"), "s must be unique."),
			Self::InvalidAttribute => "Invalid attribute.",
			Self::InvalidCli => "Invalid/unknown CLI argument.",
			Self::InvalidDst => concat!("Output path must end with ", dim!(".svg"), "!"),
			Self::InvalidFileName => "File name would be invalid as a symbol ID.",
			Self::InvalidPrefix => concat!(
				"Prefix must begin with an ASCII letter and contain only ",
				dim!("a-Z"),
				", ",
				dim!("0-9"),
				", ",
				dim!("-"),
				", and ",
				dim!("_"),
				"."
			),
			Self::NoSvgs => "No SVG sources found.",
			Self::ParseEnd => concat!("File does not end with ", dim!("</svg>"), " tag."),
			Self::Parse => "Unable to parse SVG.",
			Self::ParseScript => "Sprite map icons cannot contain Javascript.",
			Self::ParseStart => concat!("File does not start with ", dim!("<svg>"), " tag."),
			Self::ParseSvgSvg => concat!("File contains multiple ", dim!("<svg>"), " tags."),
			Self::ParseViewBox => concat!("Invalid ", dim!("viewBox"), " value."),
			Self::ParseViewBoxOffset => concat!("Non-zero ", dim!("viewBox"), " offsets are unsupported."),
			Self::ParseViewportAttr => concat!(
				"A ", dim!("viewBox"), " and/or separate ",
				dim!("width"), " and ", dim!("height"), " attributes are required.",
			),
			Self::ParseViewportSize => "Image width/height must be positive.",
			Self::Read => "Unable to read file.",
			Self::Write => "Unable to save SVG to file.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("Yesvgmap v", env!("CARGO_PKG_VERSION")),
		}
	}
}
