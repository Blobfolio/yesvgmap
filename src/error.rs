/*!
# Yesvgmap: Errors
*/

use argyle::ArgyleError;
use std::{
	error::Error,
	fmt,
	path::PathBuf,
};



#[derive(Debug, Clone)]
/// # Error type.
pub(super) enum SvgError {
	/// # Argyle passthrough.
	Argue(ArgyleError),

	/// # Duplicate entry.
	Duplicate,

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
}

impl Error for SvgError {}

impl fmt::Display for SvgError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Parse(p) => write!(f, "Unable to parse: {:?}.", p),
			Self::Read(p) => write!(f, "Unreadable: {:?}.", p),
			Self::Viewbox(p) => write!(f, "Missing viewBox: {:?}", p),
			_ => f.write_str(self.as_str()),
		}
	}
}

impl From<ArgyleError> for SvgError {
	#[inline]
	fn from(err: ArgyleError) -> Self { Self::Argue(err) }
}

impl SvgError {
	/// # As Str.
	pub(super) const fn as_str(&self) -> &'static str {
		match self {
			Self::Argue(e) => e.as_str(),
			Self::Duplicate => "Normalized file names must be unique.",
			Self::NoSvgs => "No SVGs were found.",
			Self::Write => "Unable to save the SVG map.",
			_ => "",
		}
	}
}
