# fbsnake

A snake-like game that uses fbdev. Allows you to play snake in the virtual console (ie. tty).

Written in pure Rust with large doses of Unsafe to interface with fbdev, termios and some Linux functions.

Demonstrates direct interoperation with C and C-like Rust usage.

# Usage

Execute with cargo: `cargo run -- <colour> <width> <height> <scale>`

Execute after build: `fbsnake <colour> <width> <height> <scale>`

Where `colour` is the hex `RRGGBB` and `width`/`height` are the decimal canvas size in pixels. `scale` is the integer scaling for the whole canvas.

`scale` * (`width` and `height`) must be smaller than or equal to the framebuffer size (probably your screen dimensions).

# License

GPL3 I guess.
