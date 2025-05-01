# Yesvgmap

[![ci](https://img.shields.io/github/actions/workflow/status/Blobfolio/yesvgmap/ci.yaml?style=flat-square&label=ci)](https://github.com/Blobfolio/yesvgmap/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/yesvgmap/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/yesvgmap)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/yesvgmap/issues)



Yesvgmap is a fast, lightweight CLI app for x86-64 Linux machines that compiles SVG sprite maps from one or more standalone SVG files.



## Features

* Validate/normalize the XML formatting;
* Strip comments, instructions, and declarations from the sources;
* Strip (some) empty tags;
* Correct (known) tag/attribute casing;
* Convert (many) inline styles to attributes (e.g. `style="fill:red"` to `fill="red"`);
* Validate/normalize the `viewBox`;
* Warn if any sources contain (potentially) problematic styles and identifiers;

Stripping and normalization aside, yesvgmap is not a full-blown image optimizer. It is still a good idea to pre-process any SVG sources with a tool like [svgo](https://jakearchibald.github.io/svgomg/) before tossing them into a map.



## Usage

```bash
yesvgmap [FLAGS] [OPTIONS] <PATH(S)>

# Pass as many paths as you like...
yesvgmap -o map.svg icon1.svg icon2.svg icon3.svg

# Or if it is easier to load them from a text file...
yesvgmap -o map.svg -l list.txt
```

| Short | Long | Value | Description | Default |
| ----- | ---- | ----- | ----------- | ------- |
| -a | --attribute | *key\[=val\]* | Add an attribute — id, class, etc. — to the top-level &lt;svg&gt; element. | |
| -h | --help | | Print help information and exit. | |
| -l | --list | *path* | Read (absolute) file and/or directory paths from this text file — or STDIN if "-" — one entry per line, instead of or in addition to `<PATH(S)>`. | |
| -o | --output | *path* | Save the generated map to this location. If omitted, the map will print to STDOUT instead. | |
| -p | --prefix | *string* | Set a custom prefix for the IDs of each entry in the map. (IDs look like `PREFIX-STEM`, where "STEM" is the alphanumeric portion of the source file name.) | `"i"` |
| -V | --version | | Print version information and exit. | |



## HTML Usage

Copy and paste the generated map markup directly into the top of the document `<body>`, then insert the following snippet wherever you want an icon displayed:

```html
<svg><use xlink:href="#i-plus"></use></svg>
<!--                  ^ The icon ID. -->
```

Pithy as that minimal snippet is, in practice you'll often need to dress it up a bit for styling purposes:

```html
<div class="icon" title="Add One">
	<svg class="icon-plus"><use xlink:href="#i-plus"></use></svg>
</div>
```

But that's between you and your designer. ;)



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/yesvgmap/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built/installed from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# See "cargo install --help" for more options.
cargo install \
    --git https://github.com/Blobfolio/yesvgmap.git \
    --bin yesvgmap
```
