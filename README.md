# fbsnake

A snake-like game that uses fbdev. Allows you to play snake in the virtual console (ie. tty), without X.

Written in pure Rust with large doses of unsafe to interface with fbdev, termios and some Linux syscalls.

Demonstrates how Rust can be used just like C.

# Usage

Obviously, requires Linux. Assumes the C function signatures and as such may not work on platforms other than `x86_64` (also tested on the Raspberry Pi 3 B+ with success), please leave an issue if this is the case.

To open `/dev/fb0` you must be in the `video` group.

Either:
```sh
    # Run with cargo
    cargo run -- <options>
```
*or*
```sh
    # Install
    cargo build --release && sudo mv target/release/fbsnake /usr/local/bin/

    # And run
    fbsnake <options>
```
## Options

 - `-c` or `--colour` the colour of the snake in the form `RRGGBB` where R/G/B are hex digits
 - `-w`/`-h` or `--width`/`--height` are the integer canvas dimesnions in `--scale` * pixels
 - `-s` or `--scale` is the integer scaling for the whole canvas
 - `-r` or `--rate` or `--speed` is the time in milliseconds the snake waits before moving
 - `-l` or `--length` is the starting length of the snake


Note: `--scale` * (`--width` and `--height`) must be smaller than or equal to the framebuffer size (probably your screen dimensions).

###### Pro-tip: Losing the game will exit with non-zero status while winning will exit with 0

## Controls

Move with the arrow keys. Exit with `escape` (or SIGINT, ie `^C`). Pause with `p`

# License

MIT License

Copyright (c) 2020 Aidan Prangnell

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
