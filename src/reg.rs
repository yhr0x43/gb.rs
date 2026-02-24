use std::cell::Cell;
use std::ops::{AddAssign, BitXorAssign};
use std::fmt;


pub struct Reg16(Cell<u16>);
impl Reg16 {
    pub fn new(v: u16) -> Self { Self(Cell::new(v)) }
    pub fn get(&self) -> u16 { self.0.get() }
    pub fn set(&self, v: u16) { self.0.set(v) }

    pub fn lo(&self) -> Reg8<'_> { Reg8 { r: self, hi: false } }
    pub fn hi(&self) -> Reg8<'_> { Reg8 { r: self, hi: true } }
}

impl AddAssign<u16> for Reg16 {
    fn add_assign(&mut self, rhs: u16) {
        self.0.set(self.0.get().wrapping_add(rhs));
    }
}

impl fmt::Debug for Reg16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0.get())
    }
}


pub struct Reg8<'a> {
    r: &'a Reg16,
    hi: bool,
}

impl<'a> Reg8<'a> {
    pub fn get(&self) -> u8 {
        let v = self.r.get();
        if self.hi { (v >> 8) as u8 } else { v as u8 }
    }

    pub fn set(&self, b: u8) {
        let v = self.r.get();
        let new = if self.hi {
            (v & 0x00FF) | ((b as u16) << 8)
        } else {
            (v & 0xFF00) | (b as u16)
        };
        self.r.set(new);
    }
}

impl BitXorAssign<u8> for Reg8<'_> {
    fn bitxor_assign(&mut self, rhs: u8) {
        self.set(self.get() ^ rhs);
    }
}


/* both RegId are technically not necessary in terms of code logic,
 * this is just so the "register index" can be named
 */
#[derive(Debug)]
pub enum RegId8 {
    B, C, D, E, H, L, A, F,
}

impl RegId8 {
    pub fn new(idx: u8) -> Self {
        match idx {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            6 => unreachable!("invalid 8-bit register idx"),
            7 => Self::A,
            _ => unreachable!("invalid 8-bit register idx"),
        }
    }
}


#[derive(Debug)]
pub enum RegId16 {
    BC, DE, HL, SP, AF, PC,
}

impl RegId16 {
    pub fn new(idx: u8) -> Self {
        match idx {
            0 => Self::BC,
            1 => Self::DE,
            2 => Self::HL,
            3 => Self::SP,
            _ => unreachable!("invalid 16-bit register idx"),
        }
    }
}
