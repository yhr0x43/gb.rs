use crate::bus;
use std::cell::Cell;
use std::fmt;
use std::ops;

#[derive(Debug)]
enum MemOp {
    None,
    Read8(u16),
    Write8(u16, u8),
    Read16(u16),
    Write16(u16, u16),
    Read16hi(u16),
    Write16hi(u16, u16),
}

impl MemOp {
    fn next(&self) -> Self {
        match self {
            Self::None => Self::None,
            Self::Read8(_) | Self::Write8(_, _) | Self::Read16hi(_) | Self::Write16hi(_, _) => {
                Self::None
            }
            Self::Read16(addr) => Self::Read16hi(*addr),
            Self::Write16(addr, val) => Self::Write16hi(*addr, *val),
        }
    }
}


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

impl<T: ops::Add<Output = T> + Copy> ops::AddAssign<T> for Reg<T> {
    fn add_assign(&mut self, other: T) {
        self.set(self.get() + other)
    }
}

impl<T: ops::Sub<Output = T> + Copy> ops::SubAssign<T> for Reg<T> {
    fn sub_assign(&mut self, other: T) {
        self.set(self.get() - other)
    }
}


enum HiLo {
    Hi,
    Lo,
}


#[derive(Debug)]
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

#[derive(Debug)]
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

impl Opd8 {
    pub fn regmem(idx: u8) -> Self {
        if idx == 6 {
            Self::IndReg(RegId16::HL)
        } else {
            Self::Reg(RegId8::decode(idx))
        }
    }
}

#[derive(Debug)]
enum Opd16 {
    Reg(RegId16),
    Mem(u16),
}

impl Opd16 {
    pub fn reg(idx: u8) -> Self {
        Self::Reg(RegId16::decode(idx))
    }
}

#[derive(Debug)]
pub struct Cpu {
    regs: [u16; 6],

    /* sub-instruction M-cycles state */
    opcode: u8,    /* executing opcode */
    memop: MemOp,  /* requested memory operation for this cycle */
    read_val: u16, /* result of read op, */
    subcycle: u8,  /* inc per memop, set to 0 at the end of inst */
}

#[rustfmt::skip]
impl Cpu {
    fn bc(&self) -> &mut Reg<u16> { self.r16(RegId16::BC) }
    fn de(&self) -> &mut Reg<u16> { self.r16(RegId16::DE) }
    fn hl(&self) -> &mut Reg<u16> { self.r16(RegId16::HL) }
    fn sp(&self) -> &mut Reg<u16> { self.r16(RegId16::SP) }
    fn af(&self) -> &mut Reg<u16> { self.r16(RegId16::AF) }
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
            memop: MemOp::None,
            read_val: 0,
            subcycle: 0,
        }
    }

    fn r16(&self, id: RegId16) -> &mut Reg<u16> {
        Reg::<u16>::from_mut(
            unsafe { &mut *(&raw const self.regs[id as usize] as *mut u16) }
        )
    }

    fn r8(&self, id: RegId8) -> &mut Reg<u8> {
        const ARCH_IS_LE: bool = cfg!(target_endian = "little");
        const HI_OFFSET: usize = if ARCH_IS_LE { 1 } else { 0 };
        const LO_OFFSET: usize = if ARCH_IS_LE { 0 } else { 1 };

        let p_reg16 = &raw const self.regs[id.resides() as usize] as *mut u16;
        Reg::<u8>::from_mut(unsafe {
            &mut *p_reg16.cast::<u8>().add(
                match id.hilo() {
                    HiLo::Hi => HI_OFFSET,
                    HiLo::Lo => LO_OFFSET,
                }
            )
        })
    }

    /* one cycle is one M-cycle, 4 T-states
     * one cycle can only have at most 1 bus read/write
     * thus it can be separate into 3 parts: pre-, perform-, post- memory
     *
     * all inst can be broken down to these stages:
     * - Fetch/Decode
     * - Mem Read
     * - Execute
     * - Mem Write
     * - Wait
     */
    pub fn cycle(&mut self, bus: &mut bus::Bus) {
        /* begin pre-memory */

        if self.subcycle == 0 {
            if bus.intr_poll() {
                /* respond to intr after last inst and before fetch */
                unreachable!("unimpl cpu intr handling");
            }
            self.memop = MemOp::Read8(self.pc().get());
        }

        /* end pre-memory */

        match self.memop {
            MemOp::None => {}
            MemOp::Read8(addr) => self.read_val = bus.read(addr) as u16,
            MemOp::Read16(addr) => self.read_val = bus.read(addr) as u16,
            MemOp::Read16hi(addr) => self.read_val |= (bus.read(addr + 1) as u16) << 8,
            MemOp::Write8(addr, val) => bus.write(addr, val),
            MemOp::Write16(addr, val) => bus.write(addr, val as u8),
            MemOp::Write16hi(addr, val) => bus.write(addr + 1, (val >> 8) as u8),
        }
        self.memop = self.memop.next();

        /* begin post-memory */

        /* fetch instruction */
        if self.subcycle == 0 {
            self.opcode = self.read_val as u8;
        }

        /* instruction decoding */
        /* TODO(yhr0x43): decode the inst byte every cycle, the code is cleaner
         * for now... but an internal repr of types of inst could be helpful
         */
        if self.opcode & 0xCF == 0x01 {
            // LD r16, n16
            if self.subcycle == 0 {
                self.subcycle = 3;
                self.memop = MemOp::Read16(self.pc().get() + 1);
            } else if self.subcycle == 1 {
                self.r16(RegId16::decode(self.opcode >> 4))
                    .set(self.read_val);
                *self.pc() += 3;
            }
        } else if self.opcode & 0xCF == 0x02 {
            // LD [r16(+/-)], A
            if self.subcycle == 0 {
                self.subcycle = 2;
                self.memop = MemOp::Write8(
                    match self.opcode >> 4 {
                        0 => self.bc().get(),
                        1 => self.de().get(),
                        2 | 3 => self.hl().get(),
                        _ => unreachable!("invalid reg indirect idx"),
                    },
                    self.a().get(),
                );
            } else if self.subcycle == 1 {
                match self.opcode >> 4 {
                    2 => *self.hl() += 1,
                    3 => *self.hl() -= 1,
                    _ => {}
                }
                *self.pc() += 1;
            }
        } else if self.opcode & 0xF8 == 0xA8 {
            // XOR A, r/m8
            if self.subcycle == 0 {
                match Opd8::regmem(self.opcode & 0x07) {
                    Opd8::Reg(regid) => {
                        self.subcycle = 1;
                        *self.a() ^= self.r8(regid).get();
                        self.f().set(0x70 | if self.a().get() == 0 { 0x80 } else { 0 });
                        *self.pc() += 1;
                    }
                    Opd8::IndReg(regid) => {
                        self.subcycle = 2;
                        self.memop = MemOp::Read8(self.r16(regid).get());
                    }
                }
            } else if self.subcycle == 1 {
                *self.a() ^= self.read_val as u8;
                self.f().set(0x70 | if self.a().get() == 0 { 0x80 } else { 0 });
                *self.pc() += 1;
            }
        } else {
            todo!("unimpl inst {0:X}", self.opcode)
        }

        /* end post-memory */

        self.subcycle -= 1;
    }
}
