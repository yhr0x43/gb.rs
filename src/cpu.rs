use crate::bus;
use std::cell::Cell;
use std::fmt;
use std::ops;

#[repr(transparent)]
struct Reg<T>(Cell<T>);

impl<T: Sized> Reg<T> {
    fn from_mut(t: &mut T) -> &mut Self {
        unsafe { &mut *(t as *mut T as *mut Self) }
    }
}

impl<T> ops::Deref for Reg<T> {
    type Target = Cell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: fmt::UpperHex + Copy> fmt::Debug for Reg<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{:X}", &self.get())
    }
}

impl<T: ops::BitXor<Output = T> + Copy> ops::BitXorAssign<T> for Reg<T> {
    fn bitxor_assign(&mut self, other: T) {
        self.set(self.get() ^ other)
    }
}

impl<T: ops::Add<Output = T> + Copy> Reg<T> {
    fn inc(&self, t: T) -> T {
        self.set(self.get() + t);
        self.get()
    }

    fn inc_post(&self, t: T) -> T {
        self.replace(self.get() + t)
    }
}

impl<T: ops::Sub<Output = T> + Copy> Reg<T> {
    fn dec(&self, t: T) -> T {
        self.set(self.get() - t);
        self.get()
    }

    fn dec_post(&self, t: T) -> T {
        self.replace(self.get() - t)
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
    fn decode(idx: u8) -> Self {
        match idx {
            0 => Self::B,
            1 => Self::C,
            2 => Self::D,
            3 => Self::E,
            4 => Self::H,
            5 => Self::L,
            7 => Self::A,
            _ => unreachable!("invalid 8-bit register idx"),
        }
    }

    fn resides(&self) -> RegId16 {
        match self {
            RegId8::B | RegId8::C => RegId16::BC,
            RegId8::D | RegId8::E => RegId16::DE,
            RegId8::H | RegId8::L => RegId16::HL,
            RegId8::A | RegId8::F => RegId16::AF,
        }
    }

    fn hilo(&self) -> HiLo {
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

#[derive(Debug)]
enum Opd8 {
    Reg(RegId8),
    IndReg(RegId16),
}

impl Opd8 {}

#[derive(Debug, Clone, Copy)]
enum OpdSrc {
    None,
    Mem8(bus::Addr),
    Done8(u8),
    Mem16(bus::Addr),
    Mem16Half(bus::Addr, u8),
    Done16(u16),
}

#[derive(Debug, Clone, Copy)]
enum OpdDst {
    None,
    Mem8(bus::Addr, u8),
    Mem16(bus::Addr, u16),
    Mem16Half(bus::Addr, u8),
    Done,
}

impl OpdSrc {
    pub fn read_step(&self, bus: &bus::Bus) -> OpdSrc {
        match self {
            OpdSrc::None | OpdSrc::Done8(_) | OpdSrc::Done16(_) => *self,
            OpdSrc::Mem8(addr) => OpdSrc::Done8(bus.read(*addr)),
            OpdSrc::Mem16(addr) => OpdSrc::Mem16Half(*addr, bus.read(*addr)),
            OpdSrc::Mem16Half(addr, lo) => {
                OpdSrc::Done16((bus.read(*addr + 1) as u16) << 8 | *lo as u16)
            }
        }
    }

    pub fn ready(&self) -> bool {
        match self {
            OpdSrc::None | OpdSrc::Done8(_) | OpdSrc::Done16(_) => true,
            _ => false,
        }
    }
}

impl OpdDst {
    pub fn write_step(&self, bus: &mut bus::Bus) -> OpdDst {
        match self {
            OpdDst::None | OpdDst::Done => *self,
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
            OpdDst::None | OpdDst::Done => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
enum Stage {
    Fetch,
    Read(OpdSrc),
    Wait(OpdDst),
    Write(OpdDst),
}

enum ReadVal {
    None,
    Done8(u8),
    Done16(u16),
}

impl From<OpdSrc> for ReadVal {
    fn from(value: OpdSrc) -> ReadVal {
        match value {
            OpdSrc::None => ReadVal::None,
            OpdSrc::Done8(val) => ReadVal::Done8(val),
            OpdSrc::Done16(val) => ReadVal::Done16(val),
            OpdSrc::Mem8(_) | OpdSrc::Mem16(_) | OpdSrc::Mem16Half(_, _) => {
                unreachable!("illegal conversion to phase")
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
pub struct Cpu {
    regs: [u16; 6],

    /* sub-instruction M-cycles state */
    opcode: u8, /* executing opcode */
    stage: Stage,
}

#[rustfmt::skip]
impl Cpu {
    fn bc(&self) -> &mut Reg<u16> { self.r16(RegId16::BC) }
    fn de(&self) -> &mut Reg<u16> { self.r16(RegId16::DE) }
    fn hl(&self) -> &mut Reg<u16> { self.r16(RegId16::HL) }
    fn af(&self) -> &mut Reg<u16> { self.r16(RegId16::AF) }
    fn sp(&self) -> &mut Reg<u16> { self.r16(RegId16::SP) }
    fn pc(&self) -> &mut Reg<u16> { self.r16(RegId16::PC) }
    fn b(&self) -> &mut Reg<u8> { self.r8(RegId8::B) }
    fn c(&self) -> &mut Reg<u8> { self.r8(RegId8::C) }
    fn d(&self) -> &mut Reg<u8> { self.r8(RegId8::D) }
    fn e(&self) -> &mut Reg<u8> { self.r8(RegId8::E) }
    fn h(&self) -> &mut Reg<u8> { self.r8(RegId8::H) }
    fn l(&self) -> &mut Reg<u8> { self.r8(RegId8::L) }
    fn a(&self) -> &mut Reg<u8> { self.r8(RegId8::A) }
    fn f(&self) -> &mut Reg<u8> { self.r8(RegId8::F) }
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            regs: [0; 6],
            opcode: 0, /* TODO(yhr0x43): starting opcode? */
            stage: Stage::Fetch,
        }
    }

    fn r16(&self, id: RegId16) -> &mut Reg<u16> {
        Reg::<u16>::from_mut(unsafe { &mut *(&raw const self.regs[id as usize] as *mut u16) })
    }

    fn r8(&self, id: RegId8) -> &mut Reg<u8> {
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

    fn regmem_src(&self, cpu: &Cpu, idx: u8) -> OpdSrc {
        if idx == 6 {
            OpdSrc::Mem8(cpu.hl().get())
        } else {
            OpdSrc::Done8(cpu.r8(RegId8::decode(idx)).get())
        }
    }

    fn inst_step(&self, phase: Phase) -> Stage {
        if self.opcode & 0xCF == 0x01 {
            // LD r16, n16
            match phase {
                Phase::InstFetch => Stage::Read(OpdSrc::Mem16(self.pc().get() + 1)),
                Phase::ValueReady(src) => {
                    let val = src.get16();
                    self.r16(RegId16::decode(self.opcode >> 4)).set(val);
                    self.pc().inc(3);
                    Stage::Fetch
                }
            }
        } else if self.opcode & 0xCF == 0x02 {
            // LD [r16(+/-)], A
            match phase {
                Phase::InstFetch => {
                    self.pc().inc(1);
                    Stage::Write(OpdDst::Mem8(
                        match self.opcode >> 4 {
                            0 => self.bc().get(),
                            1 => self.de().get(),
                            2 => self.hl().inc_post(1),
                            3 => self.hl().dec_post(1),
                            _ => unreachable!("invalid reg indirect idx"),
                        },
                        self.a().get(),
                    ))
                }
                Phase::ValueReady(_) => unreachable!(),
            }
        } else if self.opcode & 0xF8 == 0xA8 {
            // XOR A, r/m8
            match phase {
                Phase::InstFetch => {
                    let idx = self.opcode & 0x07;
                    Stage::Read(if idx == 6 {
                        OpdSrc::Mem8(self.hl().get())
                    } else {
                        OpdSrc::Done8(self.r8(RegId8::decode(idx)).get())
                    })
                }
                Phase::ValueReady(src) => {
                    *self.a() ^= src.get8();
                    self.f()
                        .set(0x70 | if self.a().get() == 0 { 0x80 } else { 0 });
                    self.pc().inc(1);
                    Stage::Fetch
                }
            }
        } else {
            todo!("unimpl inst {0:X}", self.opcode)
        }
    }

    /*
     * one cycle is one M-cycle, 4 T states in Z80 terms
     * one cycle can only have at most 1 bus read/write
     */
    pub fn cycle(&mut self, bus: &mut bus::Bus) {
        self.stage = match self.stage {
            Stage::Fetch => {
                self.opcode = bus.read(self.pc().get());
                self.inst_step(Phase::InstFetch)
            }
            Stage::Read(src) => {
                let src = src.read_step(bus);
                if src.ready() {
                    self.inst_step(Phase::ValueReady(src.into()))
                } else {
                    Stage::Read(src)
                }
            }
            Stage::Wait(dst) => Stage::Write(dst),
            Stage::Write(dst) => {
                let dst = dst.write_step(bus);
                if dst.ready() {
                    Stage::Fetch
                } else {
                    Stage::Write(dst)
                }
            }
        }
    }
}
