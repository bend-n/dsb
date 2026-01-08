#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct Style {
    pub bg: [u8; 3],
    pub fg: [u8; 3],
    pub secondary_color: [u8; 3],
    // one of [Style::BOLD]..
    pub flags: u8,
}
impl Hash for Cell {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.style.flags.hash(state);
        self.letter.hash(state);
    }
}
impl Cell {
    pub fn store(x: &[Cell]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                x.as_ptr().cast(),
                x.len() * size_of::<Cell>(),
            )
        }
    }
    pub unsafe fn load(x: &[u8]) -> &[Cell] {
        std::slice::from_raw_parts(
            x.as_ptr().cast(),
            x.len() / size_of::<Cell>(),
        )
    }
}
impl Style {
    pub fn basic(self, c: char) -> Cell {
        Cell {
            style: self,
            letter: Some(c),
        }
    }
    pub fn empty(self) -> Cell {
        Cell {
            style: self,
            letter: None,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            bg: [0; 3],
            fg: [255; 3],
            secondary_color: [0; 3],
            flags: 0,
        }
    }
}

use std::default::Default::default;
use std::fmt::Debug;
use std::hash::Hash;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign};
impl Style {
    pub const BOLD: u8 = 1;
    pub const DIM: u8 = 1 << 1;
    pub const ITALIC: u8 = 1 << 2;
    pub const UNDERLINE: u8 = 1 << 3;
    pub const STRIKETHROUGH: u8 = 1 << 4;
    pub const UNDERCURL: u8 = 1 << 5;
    pub const USE_SECONDARY_COLOR: u8 = 1 << 7;
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
impl BitOr<u8> for Style {
    type Output = Self;

    fn bitor(self, rhs: u8) -> Self::Output {
        Self {
            flags: self.flags | rhs,
            ..self
        }
    }
}
impl BitOrAssign<(u8, [u8; 3])> for Style {
    fn bitor_assign(&mut self, (f, c): (u8, [u8; 3])) {
        self.flags |= f;
        self.fg = c;
    }
}
impl BitAnd<(u8, [u8; 3])> for Style {
    type Output = Style;
    fn bitand(mut self, (flags, bg): (u8, [u8; 3])) -> Self::Output {
        self.flags |= flags;
        self.bg = bg;
        self
    }
}
impl BitAndAssign<(u8, [u8; 3])> for Style {
    fn bitand_assign(&mut self, rhs: (u8, [u8; 3])) {
        *self = *self & rhs;
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
