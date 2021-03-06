/*
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
*/

#[cfg(not(target_os = "linux"))]
compile_error!("fbsnake requires Linux as it utilises Linux-only features.");

// fb IOCTL constants
const FBIOGET_VSCREENINFO: u64 = 0x4600;

// mmap memory protection
const PROT_READ: i32 = 0x1;
const PROT_WRITE: i32 = 0x2;

const MAP_FAILED: i32 = -1;
const MAP_SHARED: i32 = 0x1;

// fcntl.h modes
const O_RDWR: i32 = 2;

// termios constants
const TC_ECHO: u32 = 0o0010;
const TC_ICANON: u32 = 0o0002;
const TCSANOW: i32 = 0;
const TCIOFLUSH: i32 = 2;

// SIGNALS
const SIGINT: i32 = 2;

use std::ffi::c_void as void;

// System functions
extern "C" {
    pub fn ioctl(__fd: i32, __request: u64, ...) -> i32;

    pub fn open(__file: *const u8, __oflag: i32, ...) -> i32;

    pub fn mmap(
        __addr: *const void,
        __len: usize,
        __prot: i32,
        __flags: i32,
        __fd: i32,
        __offset: isize,
    ) -> *mut void;

    pub fn tcgetattr(__fd: i32, __termios_p: *const termios) -> i32;
    pub fn tcsetattr(__fd: i32, __optional_actions: i32, __termios_p: *const termios) -> i32;
    pub fn tcflush(__fd: i32, __queue_selector: i32) -> i32;

    pub fn signal(__sig: i32, __handler: extern "C" fn(i32)) -> extern "C" fn(i32);
}

#[repr(C)]
pub struct fb_bitfield {
    offset: u32,
    length: u32,
    msb_right: u32,
}

#[repr(C)]
pub struct fb_var_screeninfo {
    xres: u32,
    yres: u32,
    xres_virtual: u32,
    yres_virtual: u32,
    xoffset: u32,
    yoffset: u32,

    bits_per_pixel: u32,
    grayscale: u32,

    red: fb_bitfield,
    green: fb_bitfield,
    blue: fb_bitfield,
    transp: fb_bitfield,

    non_std: u32,
    activate: u32,

    height: u32,
    width: u32,

    accel_flags: u32,

    pixclock: u32,
    left_margin: u32,
    right_margin: u32,
    upper_margin: u32,
    lower_margin: u32,
    hsync_len: u32,
    vsync_len: u32,
    sync: u32,
    vmode: u32,
    rotate: u32,
    colorspace: u32,
    reserved: [u32; 4],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,

    c_line: u8,
    c_cc: [u8; 32],

    c_ispeed: u32,
    c_ospeed: u32,
}

impl termios {
    pub const fn none() -> Self {
        Self {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,

            c_line: 0,
            c_cc: [0; 32],

            c_ispeed: 0,
            c_ospeed: 0,
        }
    }
}

fn main() {
    // Disable terminal output
    let mut termios: termios = unsafe { std::mem::zeroed() };
    assert!(
        0 == unsafe { tcgetattr(0, &termios as *const termios) },
        "Unable to get terminal info"
    );

    // Save terminal state
    unsafe { TERMIOS_SAVE_STATE = termios };

    let fb = unsafe { open("/dev/fb0\0".as_ptr(), O_RDWR) };
    assert!(
        fb > 0,
        "Unable to open framebuffer; Are you in the 'video' group?"
    );

    let info: fb_var_screeninfo = unsafe { std::mem::zeroed() };
    assert!(
        0 == unsafe { ioctl(fb, FBIOGET_VSCREENINFO, &info) },
        "Unable to get framebuffer info"
    );

    let len = 4 * info.xres as usize * info.yres as usize;

    // Make the framebuffer addressible by our program
    let buffer = unsafe {
        let buffer = mmap(
            std::ptr::null(),
            len,
            PROT_READ | PROT_WRITE,
            MAP_SHARED,
            fb,
            0,
        ) as *mut u32;
        assert!(
            buffer as usize != MAP_FAILED as usize,
            "Unable to mmap framebuffer"
        );
        std::slice::from_raw_parts_mut(buffer, len)
    };

    // Disable echo last; No panics should occur beyond this point
    termios.c_lflag &= !(TC_ECHO | TC_ICANON);
    assert!(
        0 == unsafe { tcsetattr(0, TCSANOW, &termios as *const termios) },
        "Unable to configure terminal"
    );

    // Catch SIGINT to prevent the application exiting without reenabling echo
    unsafe { signal(SIGINT, restore) };

    // Restore state; Safe as tcsetattr states that it doesn't modify the termios struct
    restore(
        match execute(buffer, info.xres as usize, info.yres as usize) {
            Ok(_) => 0,
            Err(error) => {
                eprintln!("{}", error);
                1
            }
        },
    );
}

// Not really any way to get around this. Safe as written exactly once during initialisation.
static mut TERMIOS_SAVE_STATE: termios = termios::none();

extern "C" fn restore(signal: i32) {
    assert!(
        0 == unsafe { tcsetattr(0, TCSANOW, &TERMIOS_SAVE_STATE as *const termios) },
        "Unable to restore terminal; You should run 'reset'"
    );

    std::process::exit(if signal == 2 { 0 } else { signal });
}

fn execute(buffer: &mut [u32], xres: usize, yres: usize) -> Result<(), String> {
    let mut args = std::env::args();
    let name = args.next().unwrap_or_else(|| "fbsnake".to_owned());

    let mut colour = 0x00_FF_FF;
    let mut width = 30;
    let mut height = 30;
    let mut scale = 5;
    let mut speed = 75;
    let mut snake_length = 10;

    let mut score = 0;

    macro_rules! arg {
        ($name:ident, $type:ty, $base:expr, $error:expr) => {
            $name = if let Ok($name) = <$type>::from_str_radix(
                if let Some(s) = &args.next() { s }
                else { return Err(format!("Usage: '{} -c <colour> -w <width> -h <height> -s <scale> -r <rate> -l <start length>'", &name)) },
                $base
            ) {
                $name
            } else {
                return Err($error.to_string())
            }
        };
    }

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-c" | "--colour" => arg!(
                colour,
                u32,
                16,
                "Colour must be in the form 'RRGGBB' where R/G/B is a hex digit"
            ),
            "-w" | "--width" => arg!(width, usize, 10, "Width must be an integer"),
            "-h" | "--height" => arg!(height, usize, 10, "Height must be an integer"),
            "-s" | "--scale" => arg!(scale, usize, 10, "Scale must be an integer"),
            "-r" | "--speed" | "--rate" => arg!(
                speed,
                u64,
                10,
                "Speed must be an integer representing the time it takes for the snake to move"
            ),
            "-l" | "--length" => arg!(
                snake_length,
                usize,
                10,
                "Starting length must be an integer"
            ),
            _ => return Err("Unknown argument passed".to_string()),
        }
    }

    if width * scale > xres {
        return Err("'width' * 'scale' cannot be bigger than framebuffer width".to_string());
    };
    if height * scale > yres {
        return Err("'height' * 'scale' cannot be bigger than framebuffer height".to_string());
    };

    #[derive(Debug, PartialEq, Copy, Clone)]
    enum Direction {
        Left,
        Right,
        Up,
        Down,
    };
    impl Direction {
        fn step(self, pos: &mut (isize, isize)) {
            match self {
                Left => pos.0 -= 1,
                Right => pos.0 += 1,
                Up => pos.1 -= 1,
                Down => pos.1 += 1,
            };
        }

        fn opposite(self) -> Self {
            match self {
                Left => Right,
                Right => Left,
                Up => Down,
                Down => Up,
            }
        }
    }
    use Direction::*;

    let mut seed = 0xDEAD_BEEF;

    let mut pos = (width as isize / 2, height as isize / 2);
    let mut pellet_pos = (
        rand(width as u32 - 1, &mut seed) as isize,
        rand(height as u32 - 1, &mut seed) as isize,
    );
    let mut dir = Right;

    // Snake tile vec.
    let mut snake = VecDeque::<(isize, isize)>::new();

    let mut set_xy = |x: isize, y: isize, colour: u32| {
        for x_scaled in 0..scale {
            for y_scaled in 0..scale {
                buffer[scale * x as usize
                    + ((scale * y as usize + y_scaled) * xres as usize)
                    + x_scaled] = colour;
            }
        }
    };

    // Clear play area with the inverse of the chosen colour
    for x in 0..width {
        for y in 0..height {
            set_xy(x as isize, y as isize, 0)
        }
    }
    // Draw pellet
    set_xy(pellet_pos.0, pellet_pos.1, !colour | 0x3F_3F_3F);

    use std::io::Read;

    let (tx, input_rx) = std::sync::mpsc::channel::<[u8; 3]>();
    std::thread::spawn(move || 'input: loop {
        let mut input = [0u8; 3];
        unsafe { tcflush(0, TCIOFLUSH) };
        std::io::stdin().read(&mut input).ok();
        match tx.send(input) {
            Ok(_) => (),
            Err(_) => break 'input,
        };

        std::thread::sleep(std::time::Duration::from_millis(5));
    });

    use std::collections::VecDeque;
    use std::sync::mpsc::TryRecvError::{ Disconnected, Empty };

    // Game loop
    'game: loop {
        let input = match input_rx.try_recv() {
            Ok(input) => input,
            Err(Empty) => [0; 3],
            Err(Disconnected) => {
                return Err("Input thread exited prematurely".to_string())
            }
        };

        let mut entropy = input[0] as u32
            | !(input[0] as u32) << 1
            | (input[0] as u32) << 2
            | !(input[0] as u32) << 3;
        hash(&mut entropy);
        seed ^= entropy;
        hash(&mut seed);

        if input[0] == b'\x1B' && input[1] == b'\0' {
            break Ok(());
        };

        // Pause the game
        if input[0] == b'p' && input[1] == b'\0' {
            'pause: loop {
                while match input_rx.try_recv() {
                    Ok([0, 0, 0])           => true,
                    Ok([b'\x1B', b'\0', _]) => break 'game Ok(()),
                    Ok([b'r', b'\0', _])    => {
                        // Clear play area with the inverse of the chosen colour
                        for x in 0..width {
                            for y in 0..height {
                                set_xy(x as isize, y as isize, 0)
                            }
                        }
                        // Draw snake
                        for pos in &snake { set_xy(pos.0, pos.1, colour) }
                        set_xy(pellet_pos.0, pellet_pos.1, !colour | 0x3F_3F_3F);

                        true
                    },
                    Ok(_)                   => break 'pause,
                    Err(Empty)              => true,
                    Err(Disconnected)       => return Err("Input thread exited prematurely".to_string())
                 } {};
            }
        };

        if input.len() == 3 {
            dir = if !(input[0] == b'\x1B' && input[1] == b'[') {
                dir
            } else {
                let newdir = match input[2] {
                    b'A' => Up,
                    b'B' => Down,
                    b'C' => Right,
                    b'D' => Left,
                    _ => dir,
                };

                if newdir == dir.opposite() {
                    dir
                } else {
                    newdir
                }
            };
        }

        dir.step(&mut pos);

        // Clamp position, teleporting to other end if oob
        if pos.0 >= width as isize {
            pos.0 = 0
        };
        if pos.0 < 0 {
            pos.0 = width as isize - 1
        };
        if pos.1 >= height as isize {
            pos.1 = 0
        };
        if pos.1 < 0 {
            pos.1 = height as isize - 1
        };

        set_xy(pos.0, pos.1, colour);
        if snake.contains(&pos) {
            println!(
                "You lost. Better luck next time...\nYour score was {}",
                score
            );
            return Err("\0".to_string());
        };

        // Pellet captured by the player so add length
        if pos == pellet_pos {
            snake_length += 1;
            score += 1
        };
        if snake_length == width * height {
            println!("You won!\nYour score was {}", score);
            return Ok(());
        }

        // Move snake
        snake.push_front(pos);
        if snake.len() > snake_length {
            if let Some(old_pos) = snake.pop_back() {
                set_xy(old_pos.0, old_pos.1, 0);
            }
        }

        // Pellet captured by the player so update its position
        if pos == pellet_pos {
            // Get a new position for the snake
            let mut new_index = rand((width * height - snake_length) as u32, &mut seed) as isize;
            // Probably a faster method but this should suffice. Would like to remove the modulo
            // Do:      set pellet_pos
            // While:   pellet_pos is inside the snake
            // Do:      increment pellet pos
            while {
                pellet_pos = (new_index % width as isize, new_index / width as isize);
                snake.contains(&pellet_pos)
            } {
                if new_index as usize + 1 >= width * height {
                    new_index = 0
                } else {
                    new_index += 1
                }
            }
            // Draw pellet as !colour and make sure it isn't too dark
            set_xy(pellet_pos.0, pellet_pos.1, !colour | 0x3F_3F_3F);
        }

        std::thread::sleep(std::time::Duration::from_millis(speed));
    }
}

// Fast hash from http://burtleburtle.net/bob/hash/integer.html
fn hash(inp: &mut u32) {
    use std::num::Wrapping;
    let mut x = Wrapping((*inp ^ 61) ^ (*inp >> 16));
    x += x << 3;
    x ^= x >> 4;
    x *= Wrapping(0x27d4_eb2d);
    x ^= x >> 15;
    x &= Wrapping(0xFFFF_FFFF);
    *inp = x.0;
}

/// Low entropy; don't use often
fn rand(max: u32, seed: &mut u32) -> u32 {
    // Ensure consecutive uses result in different values
    hash(seed);

    *seed & max
}
