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
	path::PathBuf,
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
	let out: Option<PathBuf> = args.option2_os(b"-o", b"--output")
		.map(PathBuf::from)
		.filter(|p| ! p.is_dir());

	// The ID prefix.
	let prefix: &str = args.option2_os(b"-p", b"--prefix")
		.and_then(OsStr::to_str)
		.unwrap_or("i");

	// Hiding strategy.
	let hide =
		if args.switch(b"--hidden") { HideType::Hidden }
		else if args.switch(b"--offscreen") { HideType::Offscreen }
		else { HideType::None };

	// ID and class.
	let id = args.option_os(b"--map-id").and_then(OsStr::to_str);
	let class = args.option_os(b"--map-class").and_then(OsStr::to_str);

	// Find the files!
	let map = Map::new(
		id,
		class,
		hide,
		prefix,
		Dowser::default()
			.with_paths(args.args_os())
			.into_vec(|p| Some(E_SVG) == Extension::try_from3(p))
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
    -h, --help                  Print help information and exit.
        --hidden                Hide the map using the "hidden" HTML attribute.
                                This takes priority over --offscreen when both
                                are present.
        --offscreen             Hide the map using inline styles to position it
                                offscreen.
    -V, --version               Print version information and exit.

OPTIONS:
    -l, --list <FILE>           Read (absolute) file and/or directory paths
                                from this text file, one entry per line.
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
                                crunch and/or (resursively) crawl. Only files
                                with the extension .svg will ultimately be
                                included.
"#
	));
}
