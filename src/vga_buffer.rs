#[allow(dead_code)]

use volatile::Volatile;
use core::fmt;
use lazy_static::lazy_static;
use spin::Mutex;


/*
The code for display colors
*/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

/*
The color code has 1 byte
4 bits: forground color | 3 bits: background color | 1 bit: blink
*/
impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}


/*
a character on screen, consisting of the ascii code and color code
1 byte: ascii character | 1 byte: color code
*/ 
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode
}

// VGA text buffer is a 2D array with 25 rows and 80 cols
const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

// a 2D array representing characters to write to screen
#[repr(transparent)]
struct Buffer {
    // Volatile: used to present unexpected compiler optimization
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT]
}


pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer
}


// implement methods for writing to screen
impl Writer {
    // write a character to screen
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            // start a new line
            b'\n' => self.new_line(),
            // write a byte to screen
            byte => {
                // start a new line if the line is full
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                // start writing from the last line
                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                // write the character to screen
                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code
                });
                self.column_position += 1;
            }
        }
    }

    // write a string to screen
    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // writing a legal character to screen
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // handling an illegal character
                _ => self.write_byte(0xfe)
            }
        }
    }

    // start a new line
    // move each previous line one row above, the first row gets overriden
    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                // writing each row to its previous row
                self.buffer.chars[row - 1][col].write(character);
            }
        }
        // clear the last row
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    // helper method for new_line(), clear an entire row
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

// implement format writing for Writer
impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    } 
}

// initialize singleton Writer
/* 
lazy_static: initializing the static variable only when it is
first invoked, allowing initializing global variables at runtime
*/ 
lazy_static! {
    /*
    A spinlock Mutex is used to prevent competing writing operations.
    When the spinlock is acquired, the other programs waiting for the
    spinlock would loop continuously to check for the lock availability
    */
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Cyan, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) }
    });
}


// implement print and println macro
// #[macro_export] exports the macro to crate root
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// _print is helper method for print! and println! macro
// _print need to be public to be called in print! and println! in
// other files. However, we don't want _print to be on public documentation
// so we use #[doc(hidden)] to hide it
#[doc(hidden)]  
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}