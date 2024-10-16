/*!
# Yesvgmap: Build
*/

use argyle::KeyWordsBuilder;
use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
};



/// # Build.
///
/// We might as well pre-compile the CLI keys and extensions we're looking for.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	build_cli();

	// Extensions are easy for this one!.
	let out = format!(
		"/// # Extension: SVG.\nconst E_SVG: Extension = {};",
		Extension::codegen(b"svg"),
	);
	write(&out_path("yesvgmap-extensions.rs"), out.as_bytes());
}

/// # Build CLI Keys.
fn build_cli() {
	let mut builder = KeyWordsBuilder::default();
	builder.push_keys([
		"-h", "--help",
		"--hidden",
		"--offscreen",
		"-V", "--version",
	]);
	builder.push_keys_with_values([
		"-l", "--list",
		"--map-class",
		"--map-id",
		"-o", "--output",
		"-p", "--prefix",
	]);
	builder.save(out_path("argyle.rs"));
}

/// # Output Path.
///
/// Append the sub-path to OUT_DIR and return it.
fn out_path(stub: &str) -> PathBuf {
	std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join(stub)
}


/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}
