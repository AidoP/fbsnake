// fb IOCTL constants
const FBIOGET_VSCREENINFO: u64 = 0x4600;

// mmap memory protection
const PROT_NONE:    i32 = 0x0;
const PROT_READ:    i32 = 0x1;
const PROT_WRITE:   i32 = 0x2;
const PROT_EXEC:    i32 = 0x4;

const MAP_FAILED:   i32 = -1;
const MAP_SHARED:   i32 = 0x1;

// fcntl.h modes
const O_RDONLY: i32 = 0;
const O_WRONLY: i32 = 1;
const O_RDWR:   i32 = 2;

// termios constants
const TC_ECHO:  u32 = 0o0010;
const TC_ICANON:u32 = 0o0002;
const TCSANOW:  i32 = 0;
const TCIFLUSH: i32 = 0;
const TCOFLUSH: i32 = 1;
const TCIOFLUSH:i32 = 2;

// SIGNALS
const SIGINT:   i32 = 2;

use std::ffi::c_void as void;

// System functions
extern "C" {
    pub fn ioctl(__fd: i32, __request: u64, ...) -> i32;

    pub fn open(__file: *const u8, __oflag: i32, ...) -> i32;
    
    pub fn mmap(__addr: *const void, __len: usize, __prot: i32, __flags: i32, __fd: i32, __offset: isize) -> *mut void;

    pub fn tcgetattr(__fd: i32, __termios_p: *const termios) -> i32;
    pub fn tcsetattr(__fd: i32, __optional_actions: i32, __termios_p: *const termios) -> i32;
    pub fn tcflush(__fd: i32, __queue_selector: i32) -> i32;

    pub fn signal(__sig: i32, __handler: extern fn(i32)) -> extern fn(i32);
}

#[repr(C)]
pub struct fb_bitfield {
    offset:     u32,
    length:     u32,
    msb_right:  u32
}

#[repr(C)]
pub struct fb_var_screeninfo {
    xres:           u32,
    yres:           u32,
    xres_virtual:   u32,
    yres_virtual:   u32,
    xoffset:        u32,
    yoffset:        u32,

    bits_per_pixel: u32,
    grayscale:      u32,

    red:            fb_bitfield,
    green:          fb_bitfield,
    blue:           fb_bitfield,
    transp:         fb_bitfield,

    non_std:        u32,
    activate:       u32,

    height:         u32,
    width:          u32,

    accel_flags:    u32,

    pixclock:       u32,
    left_margin:    u32,
    right_margin:   u32,
    upper_margin:   u32,
    lower_margin:   u32,
    hsync_len:      u32,
    vsync_len:      u32,
    sync:           u32,
    vmode:          u32,
    rotate:         u32,
    colorspace:     u32,
    reserved:       [u32; 4],
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct termios {
    c_iflag: u32,
    c_oflag: u32,
    c_cflag: u32,
    c_lflag: u32,

    c_line:  u8,
    c_cc:    [u8; 32],

    c_ispeed: u32,
    c_ospeed: u32
}

impl termios {
    pub const fn none() -> Self {
        Self {
            c_iflag: 0,
            c_oflag: 0,
            c_cflag: 0,
            c_lflag: 0,

            c_line:  0,
            c_cc:    [0; 32],

            c_ispeed: 0,
            c_ospeed: 0
        }
    }
}

#[cfg(not(target_os = "linux"))]
compile_error!("fbsnake requires Linux as it utilises Linux-only features.");

#[cfg(target_os = "linux")]
fn main() { 
    // Disable terminal output
    let mut termios: termios = unsafe { std::mem::zeroed() };
    assert!(0 == unsafe { tcgetattr(0, &termios as *const termios) }, "Unable to get terminal info");

    // Save terminal state
    unsafe { TERMIOS_SAVE_STATE = termios.clone() };

    let fb = unsafe { open("/dev/fb0\0".as_ptr(), O_RDWR) };
    assert!(fb > 0, "Unable to open framebuffer; Are you in the 'video' group?");

    let info: fb_var_screeninfo = unsafe { std::mem::zeroed() };
    assert!(0 == unsafe { ioctl(fb, FBIOGET_VSCREENINFO, &info) }, "Unable to get framebuffer info");

    let len = 4 * info.xres as usize * info.yres as usize;

    // Make the framebuffer addressible by our program
    let buffer = unsafe {
        let buffer = mmap(0 as *const void, len, PROT_READ | PROT_WRITE, MAP_SHARED, fb, 0) as *mut u32;
        assert!(buffer as usize != MAP_FAILED as usize);
        std::slice::from_raw_parts_mut(buffer, len)
    };

    // Disable echo last; No panics should occur beyond this point
    termios.c_lflag &= !(TC_ECHO | TC_ICANON);
    assert!(0 == unsafe { tcsetattr(0, TCSANOW, &termios as *const termios) }, "Unable to configure terminal");
    
    // Catch SIGINT to prevent the application exiting without reenabling echo
    unsafe { signal(SIGINT, restore) };

    // Restore state; Safe as tcsetattr states that it doesn't modify the termios struct
    restore(
        match execute(buffer, info.xres as usize, info.yres as usize) {
            Ok(_) => 0,
            Err(error) => { eprintln!("{}", error); 1 }
        }
    );
}

// Not really any way to get around this. Safe as written exactly once during initialisation.
static mut TERMIOS_SAVE_STATE: termios = termios::none();

extern fn restore(signal: i32) {
    assert!(0 == unsafe { tcsetattr(0, TCSANOW, &TERMIOS_SAVE_STATE as *const termios) }, "Unable to restore terminal; You should run 'reset'");

    std::process::exit(if signal == 2 { 0 } else { signal });
}

#[cfg(target_os = "linux")]
fn execute(buffer: &mut [u32], xres: usize, yres: usize) -> Result<(), String> {
    let mut args = std::env::args();
    let name = args.next().unwrap_or_else(|| "fbsnake".to_owned());
    let error = Err(format!("Usage: '{} RRGGBB width height'", &name));
    let colour =    match   u32::from_str_radix(match &args.next() { Some(s) => s, _ => return error }, 16) {
        Ok(colour) =>   colour, _ => return Err("Invalid colour: use form 'RRGGBB'".to_string()) };
    let width =     match usize::from_str_radix(match &args.next() { Some(s) => s, _ => return error }, 10) {
        Ok(width) =>    width, _ => return Err("Invalid width: must be a decimal integer".to_string()) };
    let height =    match usize::from_str_radix(match &args.next() { Some(s) => s, _ => return error }, 10) {
        Ok(height) =>   height, _ => return Err("Invalid height: must be a decimal integer".to_string()) };
    let scale =     match usize::from_str_radix(match &args.next() { Some(s) => s, _ => return error }, 10) {
        Ok(scale) =>    scale, _ => return Err("Invalid scale: must be a decimal integer".to_string()) };

    if !(width * scale <= xres) { return  Err("'width' * 'scale' cannot be bigger than framebuffer width".to_string()) };
    if !(height * scale <= yres) { return Err("'height' * 'scale' cannot be bigger than framebuffer height".to_string()) };

    #[derive(Debug, PartialEq, Copy, Clone)]
    enum Direction {
        Left,
        Right,
        Up,
        Down
    };
    impl Direction {
        fn step(&self, pos: &mut (isize, isize)) {
            match self {
                Left => pos.0 -= 1,
                Right => pos.0 += 1,
                Up => pos.1 -= 1,
                Down => pos.1 += 1
            };
        }

        fn opposite(&self) -> Self {
            match self {
                Left    => Right,
                Right   => Left,
                Up      => Down,
                Down    => Up
            }
        }
    }
    use Direction::*;

    let mut pos = (0, 0);
    let mut pellet_pos = (width as isize / 2, height as isize / 2);
    let mut dir = Right;

    // Snake tile vec
    let mut snake = VecDeque::with_capacity(MAX_SNAKE_LENGTH + 1);
    let mut snake_length = 10;
    let mut seed = 0x3A7B9F02;

    let mut set_xy = |x: isize, y: isize, colour: u32| {
        for x_scaled in 0..scale {
            for y_scaled in 0..scale {
                buffer[scale * x as usize + ((scale * y as usize + y_scaled) * xres as usize) + x_scaled] = colour;
            }
        }
    };

    // Clear play area
    for x in 0..width { for y in 0..height { set_xy(x as isize, y as isize, 0) } }
    // Draw pellet. Invert colour and make sure it isn't going to be the same as the background
    set_xy(pellet_pos.0, pellet_pos.1, !colour | 0x010101);

    use std::io::Read;

    let (tx, input) = std::sync::mpsc::channel::<[u8; 3]>();
    std::thread::spawn(move || {
        'input: loop {
            let mut input = [0u8; 3];
            unsafe { tcflush(0, TCIOFLUSH) };
            std::io::stdin().read(&mut input).ok();
            match tx.send(input) {
                Ok(_) => (),
                Err(_) => break 'input
            };

            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });

    use std::collections::VecDeque;

    const MAX_SNAKE_LENGTH: usize = 50;

    // Game loop
    loop {
        let input = input.try_recv().unwrap_or_else(|_| [0u8; 3]);
        seed ^= input[0] as u32 | (input[0] as u32) << 1 | (input[0] as u32) << 2 | (input[0] as u32) << 3; 
        hash(&mut seed);
       
        if input[0] == b'\x1B' && input[1] == b'\0' { break Ok(()) };
    
        if input.len() == 3 {
            dir = if !(input[0] == b'\x1B' && input[1] == b'[' ) { dir } else {
                let newdir = match input[2] {
                    b'A' => Up,
                    b'B' => Down,
                    b'C' => Right,
                    b'D' => Left,
                    _    => dir
                };

                if newdir == dir.opposite() { dir } else { newdir }
            };
        }

        dir.step(&mut pos);

        // Clamp position, teleporting to other end if oob
        if pos.0 >= width as isize { pos.0 = 0 };
        if pos.0 < 0 { pos.0 = width as isize - 1 };
        if pos.1 >= height as isize { pos.1 = 0 };
        if pos.1 < 0 { pos.1 = height as isize - 1 };

        set_xy(pos.0, pos.1, colour);
        if snake.contains(&pos) { println!("You lost. Better luck next time..."); return Err("\0".to_string()) };


        // Pellet captured by the player so add length
        if pos == pellet_pos { snake_length += 1; };
        if snake_length == width * height { println!("You won!"); return Ok(()) }

        // Move snake 
        snake.push_front(pos);
        if snake.len() > snake_length {
            if let Some(old_pos) = snake.pop_back() {
                set_xy(old_pos.0, old_pos.1, 0);
            }
        }

        // Pellet captured by the player so update its position
        if pos == pellet_pos {
            // TODO: BAD. May take a long time to complete when few tiles left on larger boards
            while { // do-while-do loop
                let rand_x = rand(width as u32 - 1, &mut seed) as _;
                let rand_y = rand(height as u32 - 1, &mut seed) as _;
                pellet_pos = (rand_x, rand_y);
                snake.contains(&pellet_pos)
            } { println!("Miss! {:?}", pellet_pos) };
            set_xy(pellet_pos.0, pellet_pos.1, !colour | 0x010101);
        }
    

        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

// Fast hash from http://burtleburtle.net/bob/hash/integer.html
fn hash(inp: &mut u32) {
    use std::num::Wrapping;
    let mut x = Wrapping((*inp ^ 61) ^ (*inp >> 16));
    x += x << 3;
    x ^= x >> 4;
    x *= Wrapping(0x27d4eb2d);
    x ^= x >> 15;
    x &= Wrapping(0xFFFFFFFF);
    *inp = x.0;
}

fn rand(max: u32, seed: &mut u32) -> u32 {
    // Ensure consecutive uses result in different values
    *seed ^= max;
    hash(seed);

    *seed & max
}
