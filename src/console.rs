use std::{convert::TryInto, ffi::CString, num::ParseIntError};

use winapi::{
    ctypes::c_void,
    shared::{minwindef::TRUE, ntdef::HANDLE},
    um::{
        consoleapi::{GetConsoleMode, SetConsoleMode, WriteConsoleA},
        processenv::GetStdHandle,
        wincon::{ENABLE_PROCESSED_OUTPUT, ENABLE_VIRTUAL_TERMINAL_PROCESSING},
    },
};

const STD_OUTPUT_HANDLE: u32 = -11_i32 as u32;

fn escape_sequence(sequence: &str) -> String {
    format!("\x1b[{}m", sequence)
}

#[repr(u8)]
pub enum TextFormatting {
    Default = 0,
    Bold = 1,
    NoBold = 22,
    Underline = 4,
    NoUnderline = 24,
    Negative = 7,
    NoNegative = 27,
}

impl From<TextFormatting> for String {
    fn from(formatting: TextFormatting) -> Self {
        (formatting as u8).to_string()
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum SimpleColor {
    Black = 30,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
}

pub enum Color {
    Simple(SimpleColor),
    Extended { r: u8, g: u8, b: u8 },
}

impl Color {
    pub fn to_string(&self, is_background: bool) -> String {
        let offset = if is_background { 10 } else { 0 };

        match self {
            Color::Simple(color) => (*color as u8 + offset).to_string(),
            Color::Extended { r, g, b } => format!("{};2;{};{};{}", (38 + offset), r, g, b),
        }
    }

    pub fn from_hex(hex: &str) -> Result<Self, ParseIntError> {
        let trimmed = hex.trim_start_matches('#');

        // TODO: Probably shouldn't be an assert
        assert!(trimmed.len() == 6);

        let r = u8::from_str_radix(&trimmed[0..2], 16)?;
        let g = u8::from_str_radix(&trimmed[2..4], 16)?;
        let b = u8::from_str_radix(&trimmed[4..6], 16)?;

        Ok(Color::Extended { r, g, b })
    }
}

impl From<SimpleColor> for String {
    fn from(color: SimpleColor) -> Self {
        (color as u8).to_string()
    }
}

struct Console {
    std_out_handle: HANDLE,
}

static mut CONSOLE: Option<Console> = None;

pub fn init() {
    unsafe {
        let std_out_handle = GetStdHandle(STD_OUTPUT_HANDLE);

        assert!(!std_out_handle.is_null());

        let mut actual_mode = 0;

        assert!(GetConsoleMode(std_out_handle, &mut actual_mode) == 1);

        let wanted_mode = ENABLE_VIRTUAL_TERMINAL_PROCESSING | ENABLE_PROCESSED_OUTPUT;

        #[allow(clippy::collapsible_if)]
        if actual_mode & wanted_mode != wanted_mode {
            if SetConsoleMode(std_out_handle, wanted_mode) != TRUE {
                println!("WARNING: Failed to set virtual processing mode. Terminal emulator doesn't support ANSI sequences.");
            }
        }

        CONSOLE = Some(Console { std_out_handle })
    }
}

pub fn write(message: impl Into<String>) -> u32 {
    let mut chars_written = 0;
    let message_string = message.into();
    let message_slice = message_string.as_str();
    let cstr = CString::new(message_slice).unwrap();

    unsafe {
        WriteConsoleA(
            CONSOLE.as_ref().unwrap().std_out_handle,
            cstr.as_ptr() as *const c_void,
            message_slice.len().try_into().unwrap(),
            &mut chars_written,
            std::ptr::null_mut(),
        );
    }

    chars_written
}

pub fn writeln(message: impl Into<String>) -> u32 {
    write(format!("{}\r\n", message.into()))
}

pub struct Text {
    message: String,
    sequences: Vec<String>,
}

impl Text {
    pub fn formatting(mut self, formatting: TextFormatting) -> Self {
        self.sequences.push(formatting.into());

        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.sequences.push(color.to_string(false));

        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.sequences.push(color.to_string(true));

        self
    }
}

impl From<Text> for String {
    fn from(console_text: Text) -> Self {
        format!(
            "{}{}{}",
            escape_sequence(&console_text.sequences.join(";")),
            console_text.message,
            escape_sequence("0")
        )
    }
}

pub fn text(message: impl Into<String>) -> Text {
    let message_string = message.into();

    // We cannot deal with messages ending with '\n' currently because that breaks the reset sequence
    // as its being put on the next line.
    assert!(!message_string.ends_with('\n'));

    Text {
        message: message_string,
        sequences: Vec::new(),
    }
}
