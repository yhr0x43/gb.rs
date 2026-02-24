use bus;
use std::fmt;

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

#[derive(Debug)]
pub enum RegId8 {
    B,
    C,
    D,
    E,
    H,
    L,
    A,
    F,
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
    BC,
    DE,
    HL,
    SP,
    AF,
    PC,
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
            Self::Reg(RegId8::new(idx))
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
        Self::Reg(RegId16::new(idx))
    }
}

pub struct Reg(u16);

impl Reg {
    pub fn new(val: u16) -> Self {
        Self(val)
    }
    pub fn x(&mut self) -> &mut u16 {
        &mut self.0
    }
    pub fn hi(&mut self) -> &mut u8 {
        #[cfg(target_endian = "little")]
        unsafe {
            &mut *((&raw mut self.0).cast::<u8>().add(1))
        }
        #[cfg(not(target_endian = "little"))]
        unsafe {
            &mut *((&raw mut self.0).cast::<u8>())
        }
    }
    pub fn lo(&mut self) -> &mut u8 {
        #[cfg(target_endian = "little")]
        unsafe {
            &mut *((&raw mut self.0).cast::<u8>())
        }
        #[cfg(not(target_endian = "little"))]
        unsafe {
            &mut *((&raw mut self.0).cast::<u8>().add(1))
        }
    }
}

impl fmt::Debug for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:X}", self.0)
    }
}

#[derive(Debug)]
pub struct Cpu {
    reg_bc: Reg,
    reg_de: Reg,
    reg_hl: Reg,
    reg_sp: Reg,
    reg_af: Reg,
    reg_pc: Reg,

    /* sub-instruction M-cycles state */
    opcode: u8,    /* executing opcode */
    memop: MemOp,  /* requested memory operation for this cycle */
    read_val: u16, /* result of read op, */
    subcycle: u8,  /* inc per memop, set to 0 at the end of inst */
}

impl Cpu {
    pub fn bc(&mut self) -> &mut u16 {
        self.reg_bc.x()
    }
    pub fn de(&mut self) -> &mut u16 {
        self.reg_de.x()
    }
    pub fn hl(&mut self) -> &mut u16 {
        self.reg_hl.x()
    }
    pub fn sp(&mut self) -> &mut u16 {
        self.reg_sp.x()
    }
    pub fn af(&mut self) -> &mut u16 {
        self.reg_af.x()
    }
    pub fn pc(&mut self) -> &mut u16 {
        self.reg_pc.x()
    }
    pub fn b(&mut self) -> &mut u8 {
        self.reg_bc.hi()
    }
    pub fn c(&mut self) -> &mut u8 {
        self.reg_bc.lo()
    }
    pub fn d(&mut self) -> &mut u8 {
        self.reg_de.hi()
    }
    pub fn e(&mut self) -> &mut u8 {
        self.reg_de.lo()
    }
    pub fn h(&mut self) -> &mut u8 {
        self.reg_hl.hi()
    }
    pub fn l(&mut self) -> &mut u8 {
        self.reg_hl.lo()
    }
    pub fn a(&mut self) -> &mut u8 {
        self.reg_af.hi()
    }
    pub fn f(&mut self) -> &mut u8 {
        self.reg_af.lo()
    }

    pub fn new() -> Self {
        Cpu {
            reg_af: Reg::new(0),
            reg_bc: Reg::new(0),
            reg_de: Reg::new(0),
            reg_hl: Reg::new(0),
            reg_sp: Reg::new(0),
            reg_pc: Reg::new(0),
            opcode: 0, /* TODO(yhr0x43): starting opcode? */
            memop: MemOp::None,
            read_val: 0,
            subcycle: 0,
        }
    }

    fn reg16(&mut self, id: RegId16) -> &mut u16 {
        match id {
            RegId16::BC => self.bc(),
            RegId16::DE => self.de(),
            RegId16::HL => self.hl(),
            RegId16::SP => self.sp(),
            RegId16::AF => self.af(),
            RegId16::PC => self.pc(),
        }
    }

    fn reg8(&mut self, id: RegId8) -> &mut u8 {
        match id {
            RegId8::B => self.b(),
            RegId8::C => self.c(),
            RegId8::D => self.d(),
            RegId8::E => self.e(),
            RegId8::H => self.h(),
            RegId8::L => self.l(),
            RegId8::A => self.a(),
            RegId8::F => self.f(),
        }
    }

    fn reg16idx(&mut self, idx: u8) -> &mut u16 {
        self.reg16(RegId16::new(idx))
    }

    fn reg8idx(&mut self, idx: u8) -> &mut u8 {
        self.reg8(RegId8::new(idx))
    }

    /* one cycle is one M-cycle, 4 T-states
     * one cycle can only have at most 1 bus read/write
     * thus it can be separate into 3 parts: pre-, perform-, post- memory
     */
    pub fn cycle(&mut self, bus: &mut bus::Bus) {
        /* begin pre-memory */

        if self.subcycle == 0 {
            if bus.intr_poll() {
                /* respond to intr after last inst and before fetch */
                unreachable!("unimpl cpu intr handling");
            }
            self.memop = MemOp::Read8(*self.reg_pc.x());
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
                self.memop = MemOp::Read16(*self.pc() + 1);
            } else if self.subcycle == 1 {
                let val = self.read_val;
                *self.reg16(RegId16::new(self.opcode >> 4)) = val;
                *self.pc() += 3;
            }
        } else if self.opcode & 0xCF == 0x02 {
            // LD [r16(+/-)], A
            if self.subcycle == 0 {
                self.subcycle = 2;
                self.memop = MemOp::Write8(
                    match self.opcode >> 4 {
                        0 => *self.bc(),
                        1 => *self.de(),
                        2 | 3 => *self.hl(),
                        _ => unreachable!("invalid reg indirect idx"),
                    },
                    *self.a(),
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
                        *self.a() ^= *self.reg8(regid);
                        *self.f() = 0x70 | if *self.a() == 0 { 0x80 } else { 0 };
                        *self.pc() += 1;
                    }
                    Opd8::IndReg(regid) => {
                        self.subcycle = 2;
                        self.memop = MemOp::Read8(*self.reg16(regid));
                    }
                }
            } else if self.subcycle == 1 {
                *self.a() ^= self.read_val as u8;
                *self.f() = 0x70 | if *self.a() == 0 { 0x80 } else { 0 };
                *self.pc() += 1;
            }
        } else {
            unreachable!(format!("unimpl inst {0:X}", self.opcode))
        }

        /* end post-memory */

        self.subcycle -= 1;
    }
}
