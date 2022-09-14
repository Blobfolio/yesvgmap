# Yesvgmap

[![ci](https://img.shields.io/github/workflow/status/Blobfolio/yesvgmap/Build.svg?style=flat-square&label=ci)](https://github.com/Blobfolio/yesvgmap/actions)
[![deps.rs](https://deps.rs/repo/github/blobfolio/yesvgmap/status.svg?style=flat-square&label=deps.rs)](https://deps.rs/repo/github/blobfolio/yesvgmap)<br>
[![license](https://img.shields.io/badge/license-wtfpl-ff1493?style=flat-square)](https://en.wikipedia.org/wiki/WTFPL)
[![contributions welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square&label=contributions)](https://github.com/Blobfolio/yesvgmap/issues)



Yesvgmap is a fast, lightweight CLI app for x86-64 Linux machines that compiles SVG sprite maps from one or more standalone SVG files.



## Features and Non-Features

Yesvgmap will:

* Normalize the XML output;
* Strip comments, instructions, and declarations from the sources;
* Reconstruct missing `viewBox` attributes using `width`/`height` (if present);
* Help suppress browser display using the `hidden` attribute or inline positioning styles;

If you find an icon isn't working correctly after being jammed into a map, take a look at its source code to make sure it has a `viewBox` beginning `0 0` and ending with two positive decimals, e.g. `0 0 123 456`. If it doesn't, you'll need to edit the original image to give it a canvas size matching the content, and/or recenter the layers to avoid janky offsets.

Yesvgmap will _not_ heavily compress the output. The normalization and comment-stripping passes help, but for real shrinkage, you should run your source images through something like [svgo](https://github.com/svg/svgo) first.

Yesvgmap does employ a lot of general sanity checks, but is not a spec-complete SVG validator. If your sources are weird/broken, the map might have some weirdness too.



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
| -h | --help | | Print help information and exit. | |
| | --hidden | | Hide the map using the "hidden" HTML attribute. | |
| -l | --list | *path* | Read (absolute) file and/or directory paths from this text file, one entry per line. | |
| | --map-class | *string* | Add this class to the generated SVG map. | |
| | --map-id | *string* | Add this ID to the generated SVG map. | |
| | --offscreen | | Hide the map using inline styles to position it offscreen. | |
| -o | --output | *path* | Save the generated map to this location. If omitted, the map will print to STDOUT instead. | |
| -p | --prefix | *string* | Set a custom prefix for the IDs of each entry in the map. (IDs look like `PREFIX-STEM`, where "STEM" is the alphanumeric portion of the source file name.) | `"i"` |
| -V | --version | | Print version information and exit. | |



## Installation

Debian and Ubuntu users can just grab the pre-built `.deb` package from the [latest release](https://github.com/Blobfolio/yesvgmap/releases/latest).

This application is written in [Rust](https://www.rust-lang.org/) and can alternatively be built from source using [Cargo](https://github.com/rust-lang/cargo):

```bash
# Clone the source.
git clone https://github.com/Blobfolio/yesvgmap.git

# Go to it.
cd yesvgmap

# Build as usual. Specify additional flags as desired.
cargo build \
    --bin yesvgmap \
    --release
```

(This should work under other 64-bit Unix environments too, like MacOS.)



## FAQ

### What Are SVG Sprite Maps?

An SVG sprite map is a non-displayable SVG comprising one or more displayable SVGs, each accessible via a unique ID.

Rather than inlining a full SVG like…

```html
<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512.001 512.001"><path fill="currentColor" d="M512.001 84.853L427.148 0 256.001 171.147 84.853 0 0 84.853 171.148 256 0 427.148l84.853 84.853 171.148-171.147 171.147 171.147 84.853-84.853L340.853 256z"/></svg>
```

…each time you need a simple `X` icon to appear in your document, you can inline the SVG map _once_, and then link to the (single) `X` icon over and over again like…

```html
<svg><use xlink:href="#i-close"></use></svg>
```

Depending on the amount of repetition and the size of the images, SVG sprite maps can substantially reduce the size of the HTML payload, and improve/speed up GZIP/Brotli compression.

But that said, be careful not to go overboard. Sprite maps should only include images that are actually referenced on the page. If you build a map with thousands of unused icons, that's only creating more bloat for yourself. ;)



### When Not to Use a Sprite Map?

Generally speaking, directly inlining an SVG makes more sense than using a sprite map if:
* The image only appears once;
* You need to be able to manipulate its `<path>`s at runtime for e.g. animation;
* It has no `viewBox` or requires canvas overflow for proper display;



## License

See also: [CREDITS.md](CREDITS.md)

Copyright © 2022 [Blobfolio, LLC](https://blobfolio.com) &lt;hello@blobfolio.com&gt;

This work is free. You can redistribute it and/or modify it under the terms of the Do What The Fuck You Want To Public License, Version 2.

    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    Version 2, December 2004
    
    Copyright (C) 2004 Sam Hocevar <sam@hocevar.net>
    
    Everyone is permitted to copy and distribute verbatim or modified
    copies of this license document, and changing it is allowed as long
    as the name is changed.
    
    DO WHAT THE FUCK YOU WANT TO PUBLIC LICENSE
    TERMS AND CONDITIONS FOR COPYING, DISTRIBUTION AND MODIFICATION
    
    0. You just DO WHAT THE FUCK YOU WANT TO.
