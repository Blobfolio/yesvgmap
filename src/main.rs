/*!
# Yesvgmap
*/

#![forbid(unsafe_code)]

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
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]



mod error;
mod img;



use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use dowser::{
	Dowser,
	Extension,
};
pub(crate) use error::SvgError;
use fyi_msg::Msg;
use img::{
	HideType,
	Map,
};
use std::{
	ffi::OsStr,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



/// # Main.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(SvgError::Argue(ArgyleError::WantsVersion)) => {
			println!(concat!("Yesvgmap v", env!("CARGO_PKG_VERSION")));
		},
		Err(SvgError::Argue(ArgyleError::WantsHelp)) => {
			helper();
		},
		Err(e) => {
			Msg::error(e.to_string()).die(1);
		},
	}
}

#[inline]
/// # Actual main.
///
/// Do our work here so we can easily bubble up errors and handle them nice and
/// pretty.
fn _main() -> Result<(), SvgError> {
	// The SVG extension we're looking for.
	const E_SVG: Extension = Extension::new3(*b"svg");

	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	// Make sure the output path is defined before we do any hard work.
	let out: Option<PathBuf> = args.option2(b"-o", b"--output")
		.map(|x| PathBuf::from(OsStr::from_bytes(x)))
		.filter(|p| ! p.is_dir());

	// The ID prefix.
	let prefix: &str = args.option2(b"-p", b"--prefix")
		.and_then(|x| std::str::from_utf8(x).ok())
		.unwrap_or("i");

	// Hiding strategy.
	let hide =
		if args.switch(b"--hidden") { HideType::Hidden }
		else if args.switch(b"--offscreen") { HideType::Offscreen }
		else { HideType::None };

	// ID and class.
	let id = args.option(b"--map-id").and_then(|x| std::str::from_utf8(x).ok());
	let class = args.option(b"--map-class").and_then(|x| std::str::from_utf8(x).ok());

	// Find the files!
	let map = Map::new(
		id,
		class,
		hide,
		prefix,
		Dowser::filtered(|p: &Path| Extension::try_from3(p).map_or(false, |e| e == E_SVG))
			.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x)))
			.into_vec()
	)?;

	// Save it to a file.
	if let Some(path) = out {
		write_atomic::write_file(&path, map.to_string().as_bytes())
			.map_err(|_| SvgError::Write)?;

		Msg::success(format!(
			"A sprite with {} images has been saved to {:?}",
			map.len(),
			std::fs::canonicalize(&path).unwrap()
		)).print();
	}
	// Just print it.
	else { println!("{}", map); }

	// Done!
	Ok(())
}

#[cold]
/// # Print Help.
fn helper() {
	println!(concat!(
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
    -h, --help                  Prints help information.
        --hidden                Hide with the "hidden" attribute. Overrides
                                --offscreen if both are set.
        --offscreen             Hide by placing the element offscreen with inline
                                styles.
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
"#
	));
}
