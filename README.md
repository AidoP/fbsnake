# fbsnake

A snake-like game that uses fbdev. Allows you to play snake in the virtual console (ie. tty), without X.

Written in pure Rust with large doses of unsafe to interface with fbdev, termios and some Linux syscalls.

Demonstrates how Rust can be used just like C.

# Usage

To open `/dev/fb0` you must be in the `video` group.

Either:
```sh
    # Run with cargo
    cargo run -- <colour> <width> <height> <scale>
```
*or*
```sh
    # Install
    cargo build --release && sudo mv target/release/fbsnake /usr/local/bin/

    # And run
    fbsnake <colour> <width> <height> <scale>
```
Where
 - `colour` is the hex `RRGGBB`
 - `width`/`height` are the decimal canvas size in pixels
 - `scale` is the integer scaling for the whole canvas
 - `scale` * (`width` and `height`) is smaller than or equal to the framebuffer size (probably your screen dimensions).

# License

GPL3 I guess.
