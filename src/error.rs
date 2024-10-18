/*!
# Yesvgmap: Errors
*/

use std::{
	error::Error,
	fmt,
	path::PathBuf,
};



/// # Help Text.
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
     `    ` |  \|   |  "#, "\x1b[38;5;199mYesvgmap\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r#"
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
        --hidden                Hide the map using the "hidden" HTML attribute.
                                This takes priority over --offscreen when both
                                are present.
        --offscreen             Hide the map using inline styles to position it
                                offscreen.
    -V, --version               Print version information and exit.

OPTIONS:
    -l, --list <FILE>           Read (absolute) file and/or directory paths
                                from this text file — or STDIN if "-" — one
                                entry per line, instead of or addition to
                                (actually trailing) <PATH(S)>.
        --map-class <CLASS>     Add this class to the generated SVG map.
                                [default: ]
        --map-id <ID>           Add this ID to the generated SVG map.
                                [default: ]
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



#[derive(Debug, Clone)]
/// # Error type.
pub(super) enum SvgError {
	/// # Duplicate entry.
	Duplicate(String),

	/// # No SVGs.
	NoSvgs,

	/// # Parse.
	Parse(PathBuf),

	/// # SVG Read.
	Read(PathBuf),

	/// # Viewbox.
	Viewbox(PathBuf),

	/// # Write.
	Write,

	/// # Print Help (Not an Error).
	PrintHelp,

	/// # Print Version (Not an Error).
	PrintVersion,
}

impl Error for SvgError {}

impl fmt::Display for SvgError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Duplicate(s) => write!(f, "Normalized name collision: {s}."),
			Self::Parse(p) => write!(f, "Unable to parse: {p:?}."),
			Self::Read(p) => write!(f, "Unreadable: {p:?}."),
			Self::Viewbox(p) => write!(f, "Missing viewBox: {p:?}"),
			_ => f.write_str(self.as_str()),
		}
	}
}

impl SvgError {
	/// # As Str.
	pub(super) const fn as_str(&self) -> &'static str {
		match self {
			Self::NoSvgs => "No SVGs were found.",
			Self::Write => "Unable to save the SVG map.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("Yesvgmap v", env!("CARGO_PKG_VERSION")),
			_ => "",
		}
	}
}
