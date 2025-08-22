/*!
# Yesvgmap: Build
*/

use argyle::{
	FlagsBuilder,
	KeyWordsBuilder,
};
use std::{
	collections::BTreeSet,
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
};



/// # Spec Attributes.
static ATTRIBUTES: [&str; 194] = [
	"accumulate",
	"additive",
	"alignment-baseline",
	"amplitude",
	"attributeName",
	"attributeType",
	"azimuth",
	"baseFrequency",
	"baseProfile",
	"baseline-shift",
	"begin",
	"bias",
	"by",
	"calcMode",
	"class",
	"clip",
	"clip-path",
	"clip-rule",
	"clipPathUnits",
	"color",
	"color-interpolation",
	"color-interpolation-filters",
	"cursor",
	"cx",
	"cy",
	"d",
	"decoding",
	"diffuseConstant",
	"direction",
	"display",
	"divisor",
	"dominant-baseline",
	"dur",
	"dx",
	"dy",
	"edgeMode",
	"elevation",
	"end",
	"exponent",
	"fill",
	"fill-opacity",
	"fill-rule",
	"filter",
	"filterUnits",
	"flood-color",
	"flood-opacity",
	"font-family",
	"font-size",
	"font-size-adjust",
	"font-stretch",
	"font-style",
	"font-variant",
	"font-weight",
	"fr",
	"from",
	"fx",
	"fy",
	"glyph-orientation-horizontal",
	"glyph-orientation-vertical",
	"gradientTransform",
	"gradientUnits",
	"height",
	"href",
	"id",
	"image-rendering",
	"in",
	"in2",
	"intercept",
	"k1",
	"k2",
	"k3",
	"k4",
	"kernelMatrix",
	"kernelUnitLength",
	"keyPoints",
	"keySplines",
	"keyTimes",
	"lang",
	"lengthAdjust",
	"letter-spacing",
	"lighting-color",
	"limitingConeAngle",
	"marker-end",
	"marker-mid",
	"marker-start",
	"markerHeight",
	"markerUnits",
	"markerWidth",
	"mask",
	"maskContentUnits",
	"maskUnits",
	"max",
	"media",
	"method",
	"min",
	"mode",
	"numOctaves",
	"opacity",
	"operator",
	"order",
	"orient",
	"origin",
	"overflow",
	"paint-order",
	"path",
	"pathLength",
	"patternContentUnits",
	"patternTransform",
	"patternUnits",
	"pointer-events",
	"points",
	"pointsAtX",
	"pointsAtY",
	"pointsAtZ",
	"preserveAlpha",
	"preserveAspectRatio",
	"primitiveUnits",
	"r",
	"radius",
	"refX",
	"refY",
	"repeatCount",
	"repeatDur",
	"requiredFeatures",
	"restart",
	"result",
	"rotate",
	"rx",
	"ry",
	"scale",
	"seed",
	"shape-rendering",
	"side",
	"slope",
	"spacing",
	"specularConstant",
	"specularExponent",
	"spreadMethod",
	"src",
	"startOffset",
	"stdDeviation",
	"stitchTiles",
	"stop-color",
	"stop-opacity",
	"stroke",
	"stroke-dasharray",
	"stroke-dashoffset",
	"stroke-linecap",
	"stroke-linejoin",
	"stroke-miterlimit",
	"stroke-opacity",
	"stroke-width",
	"style",
	"surfaceScale",
	"systemLanguage",
	"tabindex",
	"tableValues",
	"target",
	"targetX",
	"targetY",
	"text-anchor",
	"text-decoration",
	"text-rendering",
	"textLength",
	"to",
	"transform",
	"transform-origin",
	"type",
	"unicode-bidi",
	"values",
	"vector-effect",
	"version",
	"viewBox",
	"visibility",
	"width",
	"word-spacing",
	"writing-mode",
	"x",
	"x1",
	"x2",
	"xChannelSelector",
	"xlink:arcrole",
	"xlink:href",
	"xlink:show",
	"xlink:title",
	"xlink:type",
	"xml:lang",
	"xml:space",
	"y",
	"y1",
	"y2",
	"yChannelSelector",
	"z",
	"zoomAndPan",
];

/// # Spec Tags.
static TAGS: [&str; 74] = [
	"a",
	"animate",
	"animateMotion",
	"animateTransform",
	"circle",
	"clipPath",
	"defs",
	"desc",
	"discard",
	"ellipse",
	"feBlend",
	"feColorMatrix",
	"feComponentTransfer",
	"feComposite",
	"feConvolveMatrix",
	"feDiffuseLighting",
	"feDisplacementMap",
	"feDistantLight",
	"feDropShadow",
	"feFlood",
	"feFuncA",
	"feFuncB",
	"feFuncG",
	"feFuncR",
	"feGaussianBlur",
	"feImage",
	"feMerge",
	"feMergeNode",
	"feMorphology",
	"feOffset",
	"fePointLight",
	"feSpecularLighting",
	"feSpotLight",
	"feTile",
	"feTurbulence",
	"filter",
	"font",
	"font-face",
	"font-face-format",
	"font-face-name",
	"font-face-src",
	"font-face-uri",
	"foreignObject",
	"g",
	"glyph",
	"hkern",
	"image",
	"line",
	"linearGradient",
	"marker",
	"mask",
	"metadata",
	"missing-glyph",
	"mpath",
	"path",
	"pattern",
	"polygon",
	"polyline",
	"radialGradient",
	"rect",
	"script",
	"set",
	"stop",
	"style",
	"svg",
	"switch",
	"symbol",
	"text",
	"textPath",
	"title",
	"tspan",
	"use",
	"view",
	"vkern",
];



/// # Build.
///
/// We might as well pre-compile the CLI keys and extensions we're looking for.
fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	build_cli();
	build_flags();
	build_spec();
}

/// # Build CLI Keys.
fn build_cli() {
	let mut builder = KeyWordsBuilder::default();
	builder.push_keys([
		"-h", "--help",
		"-V", "--version",
	]);
	builder.push_keys_with_values([
		"-a", "--attribute",
		"-l", "--list",
		"-o", "--output",
		"-p", "--prefix",
	]);
	builder.save(out_path("argyle.rs"));
}

/// # Build Flags.
fn build_flags() {
	FlagsBuilder::new("ContentWarnings")
		.with_flag("ScriptTag", None)
		.with_flag("StyleTag", None)
		.with_flag("ClassAttr", None)
		.with_flag("IdAttr", None)
		.with_flag("OnAttr", None)
		.with_flag("StyleAttr", None)
		.with_alias("Scripts", ["ScriptTag", "OnAttr"], None)
		.with_alias("Attributes", ["ClassAttr", "IdAttr", "StyleAttr"], None)
		.save(out_path("content-warnings.rs"));
}

/// # Build Attribute/Tag Lists.
///
/// These are just static arrays, but XML's abusive camelCase makes sorting
/// tricky, so it's best to deal with that here where it can be guaranteed to
/// be correct.
fn build_spec() {
	let attr: Vec<&str> = ATTRIBUTES.iter()
		.copied()
		.collect::<BTreeSet<&str>>()
		.into_iter()
		.collect();

	let tags: Vec<&str> = TAGS.iter()
		.copied()
		.collect::<BTreeSet<&str>>()
		.into_iter()
		.collect();

	assert_eq!(
		attr.len(),
		ATTRIBUTES.len(),
		"BUG: Spec attribute list contains duplicates?!",
	);
	assert_eq!(
		tags.len(),
		TAGS.len(),
		"BUG: Spec tag list contains duplicates?!",
	);

	// Build it!
	let out = format!(
		"/// # Official(ish) SVG Attribute List (Correctly Cased).
static ATTR: [&str; {attr_len}] = {attr:?};

/// # Official(ish) SVG Tag List (Correctly Cased).
static TAGS: [&str; {tags_len}] = {tags:?};
",
		attr_len=attr.len(),
		tags_len=tags.len(),
	);

	write(&out_path("yesvgmap-spec.rs"), out.as_bytes());
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
