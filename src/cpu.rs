use core::cell::UnsafeCell;
use core::fmt;
use core::ptr;

use crate::bus;
#[allow(unused_imports)]
use crate::*;

extern crate my_proc_macro;

#[repr(transparent)]
pub struct Reg<T> {
    value: UnsafeCell<T>,
}

impl<T> Reg<T> {
    const fn from_mut(t: &mut T) -> &mut Self {
        unsafe { &mut *(t as *mut T as *mut Self) }
    }

    pub fn set(&self, val: T) {
        unsafe { ptr::write_volatile(self.value.get(), val) }
    }

    pub fn replace(&self, val: T) -> T {
        unsafe {
            let old_val = ptr::read_volatile(self.value.get());
            self.set(val);
            old_val
        }
    }
}

impl<T: Copy> Reg<T> {
    pub fn get(&self) -> T {
        unsafe { *self.value.get() }
    }
}

impl<T: fmt::UpperHex + Copy> fmt::Debug for Reg<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:X}", self.get())
    }
}

macro_rules! impl_reg_ops {
    ($($t:ty),* $(,)?) => {
        $(#[allow(unused)]
         impl Reg<$t> {
            fn add(&self, other: $t) -> $t {
                self.replace(self.get().wrapping_add(other))
            }
            fn sub(&self, other: $t) -> $t {
                self.replace(self.get().wrapping_sub(other))
            }
            fn xor(&self, other: $t) -> $t {
                self.replace(self.get() ^ other)
            }
        })*
    };
}

impl_reg_ops!(u8, u16);

trait AddrConvert8 {
    fn as_hiaddr(&self) -> u16;
}

impl AddrConvert8 for u8 {
    fn as_hiaddr(&self) -> u16 {
        0xFF00 | (*self as u16)
    }
}

enum HiLo {
    Hi,
    Lo,
}

#[derive(Debug, Clone, Copy)]
#[rustfmt::skip]
enum RegId8 { B, C, D, E, H, L, A, F }

impl RegId8 {
    const fn decode(idx: u8) -> Self {
        match idx {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            7 => Self::A,
            _ => Self::F,
            // _ => unreachable!("invalid 8-bit register idx"),
        }
    }

    const fn resides(&self) -> RegId16 {
        match self {
            RegId8::B | RegId8::C => RegId16::BC,
            RegId8::D | RegId8::E => RegId16::DE,
            RegId8::H | RegId8::L => RegId16::HL,
            RegId8::A | RegId8::F => RegId16::AF,
        }
    }

    const fn hilo(&self) -> HiLo {
        match self {
            Self::B | Self::D | Self::H | Self::A => HiLo::Hi,
            Self::C | Self::E | Self::L | Self::F => HiLo::Lo,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[rustfmt::skip]
enum RegId16 { BC, DE, HL, AF, SP, PC }

impl RegId16 {
    fn decode(idx: u8) -> Self {
        match idx {
            0 => Self::BC,
            1 => Self::DE,
            2 => Self::HL,
            3 => Self::SP,
            _ => unreachable!("invalid 16-bit register idx"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum OpdSrc {
    Mem8(bus::Addr),
    Done8(u8),
    Mem16(bus::Addr),
    Mem16Half(bus::Addr, u8),
    Done16(u16),
}

#[derive(Debug, Clone, Copy)]
enum OpdDst {
    Mem8(bus::Addr, u8),
    Mem16(bus::Addr, u16),
    Mem16Half(bus::Addr, u8),
    Done,
}

impl OpdSrc {
    pub fn read_step(&self, bus: &bus::Bus) -> OpdSrc {
        match self {
            OpdSrc::Done8(_) | OpdSrc::Done16(_) => *self,
            OpdSrc::Mem8(addr) => OpdSrc::Done8(bus.read(*addr)),
            OpdSrc::Mem16(addr) => OpdSrc::Mem16Half(*addr, bus.read(*addr)),
            OpdSrc::Mem16Half(addr, lo) => {
                OpdSrc::Done16((bus.read(*addr + 1) as u16) << 8 | *lo as u16)
            }
        }
    }

    pub fn ready(&self) -> bool {
        match self {
            OpdSrc::Done8(_) | OpdSrc::Done16(_) => true,
            _ => false,
        }
    }
}

impl OpdDst {
    pub fn write_step(&self, bus: &mut bus::Bus) -> OpdDst {
        match self {
            OpdDst::Done => *self,
            OpdDst::Mem8(addr, val) => {
                bus.write(*addr, *val);
                OpdDst::Done
            }
            OpdDst::Mem16(addr, val) => {
                bus.write(*addr, *val as u8);
                OpdDst::Mem16Half(*addr, (*val >> 8) as u8)
            }
            OpdDst::Mem16Half(addr, hi) => {
                bus.write(*addr + 1, *hi);
                OpdDst::Done
            }
        }
    }

    pub fn ready(&self) -> bool {
        match self {
            OpdDst::Done => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Stage {
    Fetch,
    FetchPrefixed,
    Read(OpdSrc),
    Wait(OpdDst),
    Write(OpdDst),
}

impl Stage {
    fn wait(&self) -> Stage {
        if let Stage::Wait(dst) = self {
            Stage::Write(*dst)
        } else {
            unreachable!("attempt to wait in invalid stage");
        }
    }
}

#[derive(Debug)]
enum ReadVal {
    Done8(u8),
    Done16(u16),
}

impl From<OpdSrc> for ReadVal {
    fn from(value: OpdSrc) -> ReadVal {
        match value {
            OpdSrc::Done8(val) => ReadVal::Done8(val),
            OpdSrc::Done16(val) => ReadVal::Done16(val),
            OpdSrc::Mem8(_) | OpdSrc::Mem16(_) | OpdSrc::Mem16Half(_, _) => {
                unreachable!("illegal conversion from OpdSrc to ReadVal")
            }
        }
    }
}

impl ReadVal {
    pub fn get8(&self) -> u8 {
        if let ReadVal::Done8(val) = self {
            *val
        } else {
            unreachable!("illegal 8-bit value read")
        }
    }

    pub fn get16(&self) -> u16 {
        if let ReadVal::Done16(val) = self {
            *val
        } else {
            unreachable!("illegal 16-bit value read")
        }
    }
}

enum Phase {
    InstFetch,
    ValueReady(ReadVal),
}

#[derive(Debug)]
enum FlagBit {
    Z = 0x80,
    N = 0x40,
    H = 0x20,
    C = 0x10,
}

pub struct Cpu {
    regs: [u16; 6],

    /* sub-instruction M-cycles state */
    opcode: u8, /* executing opcode */
    stage: Stage,
    prefixed: bool,
}

impl fmt::Debug for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_struct("Cpu")
            .field("BC", &format_args!("{:04X}", self.bc().get()))
            .field("DE", &format_args!("{:04X}", self.de().get()))
            .field("HL", &format_args!("{:04X}", self.hl().get()))
            .field("AF", &format_args!("{:04X}", self.af().get()))
            .field("SP", &format_args!("{:04X}", self.sp().get()))
            .field("PC", &format_args!("{:04X}", self.pc().get()))
            .field("opcode", &format_args!("{:02X}", self.opcode))
            .field("stage", &self.stage)
            .finish()
    }
}

impl Cpu {
    // see my_proc_macro.rs for details
    my_proc_macro::reg16!(bc de hl af sp pc);
    my_proc_macro::reg8!(b c d e h l a f);

    pub const fn new() -> Self {
        Cpu {
            regs: [0; 6],
            opcode: 0, /* TODO(yhr0x43): starting opcode? */
            stage: Stage::Fetch,
            prefixed: false,
        }
    }

    fn flag_set(&self, fb: FlagBit, val: bool) -> u8 {
        if val {
            self.f().replace((fb as u8) | self.f().get())
        } else {
            self.f().replace(!(fb as u8) & self.f().get())
        }
    }

    const fn r16(&self, id: RegId16) -> &Reg<u16> {
        Reg::<u16>::from_mut(unsafe { &mut *(&raw const self.regs[id as usize] as *mut u16) })
    }

    const fn r8(&self, id: RegId8) -> &Reg<u8> {
        const ARCH_IS_LE: bool = cfg!(target_endian = "little");
        const HI_OFFSET: usize = if ARCH_IS_LE { 1 } else { 0 };
        const LO_OFFSET: usize = if ARCH_IS_LE { 0 } else { 1 };

        let p_reg16 = &raw const self.regs[id.resides() as usize] as *mut u16;
        Reg::<u8>::from_mut(unsafe {
            &mut *p_reg16.cast::<u8>().add(match id.hilo() {
                HiLo::Hi => HI_OFFSET,
                HiLo::Lo => LO_OFFSET,
            })
        })
    }

    fn decode_regind8_src(&self, idx: u8) -> Stage {
        Stage::Read(if idx == 6 {
            OpdSrc::Mem8(self.hl().get())
        } else {
            OpdSrc::Done8(self.r8(RegId8::decode(idx)).get())
        })
    }

    fn decode_regind8_dst(&self, idx: u8, val: u8) -> Stage {
        if idx == 6 {
            Stage::Write(OpdDst::Mem8(self.hl().get(), val))
        } else {
            self.r8(RegId8::decode(idx)).set(val);
            Stage::Fetch
        }
    }

    fn decode_regind16_src(&self, idx: u8) -> Stage {
        Stage::Read(OpdSrc::Mem8(
            match idx {
                0 => self.bc().get(),
                1 => self.de().get(),
                2 => self.hl().add(1),
                3 => self.hl().sub(1),
                _ => unreachable!("invalid reg indirect 16 idx"),
            },
        ))
    }


    fn decode_regind16_dst(&self, idx: u8, val: u8) -> Stage {
        Stage::Write(OpdDst::Mem8(
            match idx {
                0 => self.bc().get(),
                1 => self.de().get(),
                2 => self.hl().add(1),
                3 => self.hl().sub(1),
                _ => unreachable!("invalid reg indirect 16 idx"),
            },
            val,
        ))
    }

    fn decode_reg16(&self, idx: u8) -> &Reg<u16> {
        match idx {
            0 => self.bc(),
            1 => self.de(),
            2 => self.hl(),
            3 => self.af(),
            _ => unreachable!(),
        }
    }

    fn decode_reg16_src(&self, idx: u8) -> Stage {
        Stage::Read(OpdSrc::Done16(self.decode_reg16(idx).get()))
    }

    fn decode_reg16_dst(&self, idx: u8, val: u16) -> Stage {
        self.decode_reg16(idx).set(val);
        Stage::Write(OpdDst::Done)
    }

    fn inst_prefix_step(&self, phase: Phase) -> Stage {
        if self.opcode & 0xC0 == 0x40 {
            // BIT n3, r/m8
            match phase {
                Phase::InstFetch => {
                    let idx = self.opcode & 0x7;
                    self.decode_regind8_src(idx)
                }
                Phase::ValueReady(src) => {
                    let offset = (self.opcode >> 3) & 0x7;
                    self.flag_set(FlagBit::Z, (src.get8() & (1u8 << offset)) == 0);
                    self.flag_set(FlagBit::N, false);
                    self.flag_set(FlagBit::H, true);
                    self.pc().add(1);
                    Stage::Fetch
                }
            }
        } else {
            todo!(
                "prefix instruction [{:04X}] {:02X}",
                self.pc().get(),
                self.opcode
            )
        }
    }

    fn inst_step(&self, phase: Phase) -> Stage {
        if self.opcode == 0x76 {
            // HALT
            todo!("halt instruction")
        } else if self.opcode & 0xC0 == 0x40 {
            // LD r/m8, r/m8
            match phase {
                Phase::InstFetch => {
                    let idx = self.opcode & 0x07;
                    self.decode_regind8_src(idx)
                }
                Phase::ValueReady(src) => {
                    let idx = self.opcode >> 3 & 0x07;
                    self.pc().add(1);
                    self.decode_regind8_dst(idx, src.get8())
                }
            }
        } else if self.opcode == 0xE0 {
            // LD [a8], A
            match phase {
                Phase::InstFetch => {
                    self.pc().add(1);
                    Stage::Read(OpdSrc::Mem8(self.pc().get()))
                }
                Phase::ValueReady(src) => {
                    self.pc().add(1);
                    Stage::Write(OpdDst::Mem8(0xFF00 + (src.get8() as u16), self.a().get()))
                }
            }
        } else if self.opcode & 0xCF == 0x01 {
            // LD r16, n16
            match phase {
                Phase::InstFetch => {
                    self.pc().add(1);
                    Stage::Read(OpdSrc::Mem16(self.pc().get()))
                },
                Phase::ValueReady(src) => {
                    let idx = self.opcode >> 4;
                    self.r16(RegId16::decode(idx)).set(src.get16());
                    self.pc().add(2);
                    Stage::Fetch
                }
            }
        } else if self.opcode & 0xCF == 0x02 {
            // LD [r16+-], A
            match phase {
                Phase::InstFetch => {
                    self.pc().add(1);
                    let idx = self.opcode >> 4;
                    self.decode_regind16_dst(idx, self.a().get())
                }
                Phase::ValueReady(_) => unreachable!("LD [r16+-], A"),
            }
        } else if self.opcode & 0xCF == 0x0A {
            // LD A, [r16+-]
            match phase {
                Phase::InstFetch => {
                    let idx = self.opcode >> 4;
                    self.decode_regind16_src(idx)
                }
                Phase::ValueReady(dst) => {
                    self.a().set(dst.get8());
                    self.pc().add(1);
                    Stage::Fetch
                }
            }
        } else if self.opcode & 0xF8 == 0xA8 {
            // XOR A, r/m8
            match phase {
                Phase::InstFetch => {
                    let idx = self.opcode & 0x07;
                    self.decode_regind8_src(idx)
                }
                Phase::ValueReady(src) => {
                    self.a().xor(src.get8());
                    self.f().set(0);
                    self.flag_set(FlagBit::Z, self.a().get() == 0);
                    self.pc().add(1);
                    Stage::Fetch
                }
            }
        } else if self.opcode & 0xC7 == 0x06 {
            // LD r/m8, n8
            match phase {
                Phase::InstFetch => {
                    self.pc().add(1);
                    Stage::Read(OpdSrc::Mem8(self.pc().get()))
                }
                Phase::ValueReady(src) => {
                    self.pc().add(1);
                    let idx = self.opcode >> 3 & 0x7;
                    self.decode_regind8_dst(idx, src.get8())
                }
            }
        } else if self.opcode & 0xC7 == 0x04 {
            // INC r/m8
            let idx = self.opcode >> 3 & 0x7;
            match phase {
                Phase::InstFetch => {
                    self.decode_regind8_src(idx)
                }
                Phase::ValueReady(src) => {
                    self.pc().add(1);
                    self.decode_regind8_dst(idx, src.get8() + 1)
                }
            }
        } else if self.opcode & 0xCF == 0x03 {
            // INC r16
            let idx = self.opcode >> 4 & 0x3;
            match phase {
                Phase::InstFetch => {
                    self.decode_reg16_src(idx)
                }
                Phase::ValueReady(src) => {
                    self.pc().add(1);
                    self.decode_reg16_dst(idx, src.get16() + 1)
                }
            }
        } else if self.opcode == 0xE2 {
            // LDH [C], A
            match phase {
                Phase::InstFetch => {
                    Stage::Read(OpdSrc::Done8(self.a().get()))
                }
                Phase::ValueReady(src) => {
                    self.pc().add(1);
                    Stage::Write(OpdDst::Mem8(self.c().get().as_hiaddr(), src.get8()))
                }
            }
        } else if self.opcode & 0xD7 == 0x00 {
            // JR cc, e8
            match phase {
                Phase::InstFetch => {
                    self.pc().add(1);
                    Stage::Read(OpdSrc::Mem8(self.pc().get()))
                }
                Phase::ValueReady(src) => {
                    let flag = (if (self.opcode & 0x01) == 0x01 {
                        FlagBit::C
                    } else {
                        FlagBit::Z
                    }) as u8;
                    let cond = self.f().get() & flag == 0;
                    let neg = self.opcode & 0x80 == 0;
                    if cond == neg {
                        self.pc().add((src.get8() as i8) as u16 + 1);
                        Stage::Write(OpdDst::Done)
                    } else {
                        self.pc().add(1);
                        Stage::Fetch
                    }
                }
            }
        } else if self.opcode & 0xCF == 0xC5 {
            // PUSH r16
            match phase {
                Phase::InstFetch => {
                    let idx = self.opcode >> 4 & 0x3;
                    self.decode_reg16_src(idx)
                }
                Phase::ValueReady(src) => {
                    self.pc().add(1);
                    self.sp().sub(2);
                    Stage::Write(OpdDst::Mem16(self.sp().get(), src.get16()))
                }
            }
        } else {
            todo!(
                "instruction [{:04X}] {:02X}\n{:?}",
                self.pc().get(),
                self.opcode,
                self
            )
        }
    }

    /*
     * one cycle is one M-cycle, 4 T states in Z80 terms
     * one cycle can only have at most 1 bus read/write
     */
    pub fn cycle(&mut self, bus: &mut bus::Bus) {
        let mut memop = false;
        self.stage = match self.stage {
            Stage::Fetch => {
                self.prefixed = false;
                self.opcode = bus.read(self.pc().get());
                memop = true;
                if self.opcode == 0xCB {
                    // PREFIX
                    self.prefixed = true;
                    self.pc().add(1);
                    Stage::FetchPrefixed
                } else {
                    self.inst_step(Phase::InstFetch)
                }
            }
            Stage::FetchPrefixed => {
                self.opcode = bus.read(self.pc().get());
                memop = true;
                self.inst_prefix_step(Phase::InstFetch)
            }
            Stage::Read(_) | Stage::Write(_) => self.stage,
            Stage::Wait(dst) => Stage::Write(dst),
        };
        if let Stage::Read(src) = self.stage {
            if !memop {
                let src = src.read_step(bus);
                if src.ready() {
                    self.stage = if self.prefixed {
                        self.inst_prefix_step(Phase::ValueReady(src.into()))
                    } else {
                        self.inst_step(Phase::ValueReady(src.into()))
                    }
                } else {
                    self.stage = Stage::Read(src);
                }
            }
        }
        if let Stage::Write(dst) = self.stage {
            if !memop {
                let dst = dst.write_step(bus);
                self.stage = if dst.ready() {
                    Stage::Fetch
                } else {
                    Stage::Write(dst)
                }
            }
        }
    }
}
