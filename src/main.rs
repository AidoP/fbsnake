// kd.h IOCTL constants
const GIO_CMAP: u64 = 0x4B70;
const PIO_CMAP: u64 = 0x4B71;

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

use std::ffi::c_void as void;

// System functions
extern "C" {
    pub fn ioctl(__fd: i32, __request: u64, ...) -> i32;

    pub fn open(__file: *const u8, __oflag: i32, ...) -> i32;
    
    pub fn mmap(__addr: *const void, __len: usize, __prot: i32, __flags: i32, __fd: i32, __offset: isize) -> *mut void;

    pub fn tcgetattr(__fd: i32, __termios_p: *const termios) -> i32;
    pub fn tcsetattr(__fd: i32, __optional_actions: i32, __termios_p: *const termios) -> i32;
    pub fn tcflush(__fd: i32, __queue_selector: i32) -> i32;
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

#[cfg(not(target_os = "linux"))]
compile_error!("fbsnake requires Linux as it utilises Linux-only features.");

#[cfg(target_os = "linux")]
fn main() { std::process::exit({
    // Disable terminal output
    let mut termios: termios = unsafe { std::mem::zeroed() };
    assert!(0 == unsafe { tcgetattr(0, &termios as *const termios) }, "Unable to get terminal info");
    termios.c_lflag &= !(TC_ECHO | TC_ICANON);
    assert!(0 == unsafe { tcsetattr(0, TCSANOW, &termios as *const termios) }, "Unable to configure terminal");

    let fb = unsafe { open("/dev/fb0\0".as_ptr(), O_RDWR) };
    assert!(fb > 0, "Unable to open framebuffer");

    let info: fb_var_screeninfo = unsafe { std::mem::zeroed() };
    assert!(0 == unsafe { ioctl(fb, FBIOGET_VSCREENINFO, &info) }, "Unable to get framebuffer info");

    let len = 4 * info.xres as usize * info.yres as usize;

    let buffer = unsafe {
        let buffer = mmap(0 as *const void, len, PROT_READ | PROT_WRITE, MAP_SHARED, fb, 0) as *mut u32;
        assert!(buffer as usize != MAP_FAILED as usize);
        std::slice::from_raw_parts_mut(buffer, len)
    };

    let mut args = std::env::args();
    let name = args.next().unwrap_or_else(|| "fbsnake".to_owned());
    let error = format!("Usage: '{} RRGGBB width height'", &name);
    let colour = u32::from_str_radix(&args.next().expect(&error), 16).expect("Invalid colour: use form 'RRGGBB'");
    let width = usize::from_str_radix(&args.next().expect(&error), 10).expect("Invalid width: must be a decimal integer");
    let height = usize::from_str_radix(&args.next().expect(&error), 10).expect("Invalid height: must be a decimal integer");
    let scale = usize::from_str_radix(&args.next().expect(&error), 10).expect("Invalid scale: must be a decimal integer");

    assert!(width *  scale <= info.xres as usize, "'width' * 'scale' cannot be bigger than framebuffer width");
    assert!(height * scale <= info.yres as usize, "'height' * 'scale' cannot be bigger than framebuffer height");

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
    let mut dir = Right; 

    let mut set_xy = |x: isize, y: isize, colour: u32| {
        for x_scaled in 0..scale {
            for y_scaled in 0..scale {
                buffer[scale * x as usize + ((scale * y as usize + y_scaled) * info.xres as usize) + x_scaled] = colour;
            }
        }
    };

    // Clear play area
    for x in 0..width { for y in 0..height { set_xy(x as isize, y as isize, 0) } }

    use std::io::Read;

    let (tx, input) = std::sync::mpsc::channel::<[u8; 3]>();
    std::thread::spawn(move || {
        loop {
            let mut input = [0u8; 3];
            unsafe { tcflush(0, TCIOFLUSH) };
            let index = std::io::stdin().read(&mut input).unwrap_or_else(|_| 0);
            tx.send(input).unwrap();

            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });

    // Game loop
    loop {
        let input = input.try_recv().unwrap_or_else(|_| [0u8; 3]);
        
        if input[0] == b'\x1B' && input[1] == b'\0' { return };
    
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
        //println!("Set {:?}", pos);

        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    0
})}
