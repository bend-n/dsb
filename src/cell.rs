#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    pub bg: [u8; 3],
    pub color: [u8; 3],
    // one of [Style::BOLD]..
    pub flags: u8,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            bg: [0; 3],
            color: [255; 3],
            flags: 0,
        }
    }
}

use std::default::Default::default;
use std::fmt::Debug;
impl Style {
    pub const BOLD: u8 = 1;
    pub const DIM: u8 = 1 << 1;
    pub const ITALIC: u8 = 1 << 2;
    pub const UNDERLINE: u8 = 1 << 3;
    pub const STRIKETHROUGH: u8 = 1 << 4;
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Cell {
    pub style: Style,
    pub letter: Option<char>,
}
impl Debug for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.letter.unwrap_or(' '))
    }
}
impl Cell {
    pub fn basic(c: char) -> Self {
        Self {
            letter: Some(c),
            ..default()
        }
    }
}
