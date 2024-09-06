/*!
# Yesvgmap: Build
*/

use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::Path,
};



/// # Build.
///
/// We might as well pre-compile the extensions we're looking for.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	let out = format!(
		"/// # Extension: SVG.\nconst E_SVG: Extension = {};",
		Extension::codegen(b"svg"),
	);

	let out_path = std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join("yesvgmap-extensions.rs");

	write(&out_path, out.as_bytes());
}

/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}
