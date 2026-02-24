use bus;
use reg;


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
            Self::Read8(_) | Self::Write8(_, _) | Self::Read16hi(_) | Self::Write16hi(_, _) => Self::None,
            Self::Read16(addr) => Self::Read16hi(*addr),
            Self::Write16(addr, val) => Self::Write16hi(*addr, *val),
        }
    }
}


#[derive(Debug)]
enum Opd8 {
    Reg(reg::RegId8),
    IndReg(reg::RegId16),
}

impl Opd8 {
    pub fn regmem(idx: u8) -> Self {
        if idx == 6 {
            Self::IndReg(reg::RegId16::HL)
        } else {
            Self::Reg(reg::RegId8::new(idx))
        }
    }
}


#[derive(Debug)]
enum Opd16 {
    Reg(reg::RegId16),
    Mem(u16),
}

impl Opd16 {
    pub fn reg(idx: u8) -> Self {
        Self::Reg(reg::RegId16::new(idx))
    }
}

#[derive(Debug)]
pub struct Cpu {
    reg_bc: reg::Reg16,
    reg_de: reg::Reg16,
    reg_hl: reg::Reg16,
    reg_sp: reg::Reg16,
    reg_af: reg::Reg16,
    reg_pc: reg::Reg16,

    /* sub-instruction M-cycles state */
    opcode: u8,    /* executing opcode */
    memop: MemOp,  /* requested memory operation for this cycle */
    read_val: u16, /* result of read op, */
    subcycle: u8,  /* inc per memop, set to 0 at the end of inst */
}

impl Cpu {
    pub fn bc(&mut self) -> &mut reg::Reg16 { &mut self.reg_bc }
    pub fn de(&mut self) -> &mut reg::Reg16 { &mut self.reg_de }
    pub fn hl(&mut self) -> &mut reg::Reg16 { &mut self.reg_hl }
    pub fn sp(&mut self) -> &mut reg::Reg16 { &mut self.reg_sp }
    pub fn af(&mut self) -> &mut reg::Reg16 { &mut self.reg_af }
    pub fn pc(&mut self) -> &mut reg::Reg16 { &mut self.reg_pc }
    pub fn b(&self) -> reg::Reg8 { self.reg_bc.hi() }
    pub fn c(&self) -> reg::Reg8 { self.reg_bc.lo() }
    pub fn d(&self) -> reg::Reg8 { self.reg_de.hi() }
    pub fn e(&self) -> reg::Reg8 { self.reg_de.lo() }
    pub fn h(&self) -> reg::Reg8 { self.reg_hl.hi() }
    pub fn l(&self) -> reg::Reg8 { self.reg_hl.lo() }
    pub fn a(&self) -> reg::Reg8 { self.reg_af.hi() }
    pub fn f(&self) -> reg::Reg8 { self.reg_af.lo() }

    pub fn new() -> Self {
        Cpu {
            reg_af: reg::Reg16::new(0),
            reg_bc: reg::Reg16::new(0),
            reg_de: reg::Reg16::new(0),
            reg_hl: reg::Reg16::new(0),
            reg_sp: reg::Reg16::new(0),
            reg_pc: reg::Reg16::new(0),
            opcode: 0, /* TODO(yhr0x43): starting opcode? */
            memop: MemOp::None,
            read_val: 0,
            subcycle: 0,
        }
    }

    /* instruction */
    fn reg16(&mut self, id: reg::RegId16) -> &mut reg::Reg16 {
        match id {
            reg::RegId16::BC => self.bc(),
            reg::RegId16::DE => self.de(),
            reg::RegId16::HL => self.hl(),
            reg::RegId16::SP => self.sp(),
            reg::RegId16::AF => self.af(),
            reg::RegId16::PC => self.pc(),
        }
    }

    fn reg8(&mut self, id: reg::RegId8) -> reg::Reg8 {
        match id {
            reg::RegId8::B => self.b(),
            reg::RegId8::C => self.c(),
            reg::RegId8::D => self.d(),
            reg::RegId8::E => self.e(),
            reg::RegId8::H => self.h(),
            reg::RegId8::L => self.l(),
            reg::RegId8::A => self.a(),
            reg::RegId8::F => self.f(),
        }
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
                panic!("unimpl cpu intr handling");
            }
            self.memop = MemOp::Read8(self.reg_pc.get());
        }

        /* end pre-memory */

        match self.memop {
            MemOp::None => {},
            MemOp::Read8(addr) => self.read_val = bus.read(addr) as u16,
            MemOp::Read16(addr) => self.read_val = bus.read(addr) as u16,
            MemOp::Read16hi(addr) => self.read_val |= (bus.read(addr + 1) as u16) << 8,
            MemOp::Write8(addr, val) => bus.write(addr, val),
            MemOp::Write16(addr, val) => bus.write(addr, val as u8),
            MemOp::Write16hi(addr, val) => bus.write(addr + 1, (val >> 8) as u8),
        }
        self.memop = self.memop.next();
        /* if !(matches!(self.memop, MemOp::None)) { return } */

        /* begin post-memory */

        /* fetch instruction */
        if self.subcycle == 0 {
            self.opcode = self.read_val as u8;
        }

        /* instruction decoding */
        /* TODO(yhr0x43): decode the inst byte every cycle, the code is cleaner
         * for now... but an internal repr of types of inst could be helpful
         */
        if self.opcode & !0x31 == 0 {
            // LD r16, n16
            if self.subcycle == 0 {
                self.memop = MemOp::Read16(self.reg_pc.get() + 1);
                self.subcycle = 3;
            } else if self.subcycle == 1 {
                let val = self.read_val;
                self.reg16(reg::RegId16::new(self.opcode >> 4)).set(val);
                self.reg_pc += 3;
            }
        } else if self.opcode & 0xA8 == 0xA8 {
            // XOR A, r/m8
            if self.subcycle == 0 {
                match Opd8::regmem(self.opcode & 0x07) {
                    Opd8::Reg(regid) => {
                        self.subcycle = 1;
                        let mut r = self.a();
                        r ^= self.read_val as u8;
                        self.reg_pc += 1;
                    },
                    Opd8::IndReg(regid) => {
                        self.subcycle = 2;
                        self.memop = MemOp::Read8(self.reg16(regid).get());
                    },
                }
            } else if self.subcycle == 1 {
                let mut r = self.a();
                r ^= self.read_val as u8;
                self.reg_pc += 1;
            }
        } else {
            panic!("unimpl inst")
        }

        /* end post-memory */

        self.subcycle -= 1;
    }
}
