use core::cell::Cell;
use core::fmt;
use core::ops::ControlFlow;

use crate::bus;
use crate::reg::Reg;
#[allow(unused_imports)]
use crate::*;

extern crate my_proc_macro;

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

#[derive(Clone, Copy)]
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
            // _ => Self::F,
            _ => unreachable!("invalid 8-bit register idx"),
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

#[derive(Clone, Copy)]
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
    None,
    Mem8(bus::Addr),
    Done8(u8),
    Mem8Ex(bus::Addr),
    Done8Ex(u8),
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
            OpdSrc::None | OpdSrc::Done8(_) | OpdSrc::Done16(_) | OpdSrc::Done8Ex(_) => *self,
            OpdSrc::Mem8(addr) => OpdSrc::Done8(bus.read(*addr)),
            OpdSrc::Mem8Ex(addr) => OpdSrc::Done8Ex(bus.read(*addr)),
            OpdSrc::Mem16(addr) => OpdSrc::Mem16Half(*addr, bus.read(*addr)),
            OpdSrc::Mem16Half(addr, lo) => {
                OpdSrc::Done16((bus.read(*addr + 1) as u16) << 8 | *lo as u16)
            }
        }
    }

    pub fn ready(&self) -> bool {
        matches!(
            self,
            OpdSrc::None | OpdSrc::Done8(_) | OpdSrc::Done16(_) | OpdSrc::Done8Ex(_)
        )
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
        matches!(self, OpdDst::Done)
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
        if let Stage::Write(dst) = self {
            Stage::Wait(*dst)
        } else {
            unreachable!("attempt to wait in invalid stage");
        }
    }
}

#[derive(Debug)]
enum ReadVal {
    None,
    Done8(u8),
    Done8Ex(u8),
    Done16(u16),
}

impl From<OpdSrc> for ReadVal {
    fn from(value: OpdSrc) -> ReadVal {
        match value {
            OpdSrc::None => ReadVal::None,
            OpdSrc::Done8(val) => ReadVal::Done8(val),
            OpdSrc::Done8Ex(val) => ReadVal::Done8Ex(val),
            OpdSrc::Done16(val) => ReadVal::Done16(val),
            OpdSrc::Mem8(_) | OpdSrc::Mem16(_) | OpdSrc::Mem16Half(_, _) | OpdSrc::Mem8Ex(_) => {
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

type InstOp = fn(&Cpu, Phase) -> Stage;
mod inst {
    use super::cpu::*;

    #[inline(always)]
    fn decode_regind8_src(cpu: &Cpu, idx: u8) -> Stage {
        Stage::Read(if idx == 6 {
            OpdSrc::Mem8(cpu.hl().get())
        } else {
            OpdSrc::Done8(cpu.r8(RegId8::decode(idx)).get())
        })
    }

    #[inline(always)]
    fn decode_regind8_dst(cpu: &Cpu, idx: u8, val: u8) -> Stage {
        if idx == 6 {
            Stage::Write(OpdDst::Mem8(cpu.hl().get(), val))
        } else {
            cpu.r8(RegId8::decode(idx)).set(val);
            Stage::Fetch
        }
    }

    #[inline(always)]
    fn decode_regind16_src(cpu: &Cpu, idx: u8) -> Stage {
        Stage::Read(OpdSrc::Mem8(match idx {
            0 => cpu.bc().get(),
            1 => cpu.de().get(),
            2 => cpu.hl().post_inc(1),
            3 => cpu.hl().post_dec(1),
            _ => unreachable!("invalid reg indirect 16 idx"),
        }))
    }

    #[inline(always)]
    fn decode_regind16_dst(cpu: &Cpu, idx: u8, val: u8) -> Stage {
        Stage::Write(OpdDst::Mem8(
            match idx {
                0 => cpu.bc().get(),
                1 => cpu.de().get(),
                2 => cpu.hl().post_inc(1),
                3 => cpu.hl().post_dec(1),
                _ => unreachable!("invalid reg indirect 16 idx"),
            },
            val,
        ))
    }

    #[inline(always)]
    fn decode_reg16(cpu: &Cpu, idx: u8) -> &Reg<u16> {
        match idx {
            0 => cpu.bc(),
            1 => cpu.de(),
            2 => cpu.hl(),
            3 => cpu.af(),
            _ => unreachable!(),
        }
    }

    #[inline(always)]
    fn decode_reg16_src(cpu: &Cpu, idx: u8) -> Stage {
        Stage::Read(OpdSrc::Done16(decode_reg16(cpu, idx).get()))
    }

    #[inline(always)]
    fn decode_reg16_dst(cpu: &Cpu, idx: u8, val: u16) -> Stage {
        decode_reg16(cpu, idx).set(val);
        Stage::Write(OpdDst::Done)
    }

    #[inline(always)]
    fn masked_swap(a: &mut u8, b: &mut u8, mask: u8) {
        *a ^= *b & mask;
        *b ^= *a & mask;
        *a ^= *b & mask;
    }

    fn rl(cpu: &Cpu, phase: Phase) -> Stage {
        // RLC r/m8; RL r/m8;
        let idx = cpu.opcode & 0x7;
        match phase {
            Phase::InstFetch => decode_regind8_src(cpu, idx),
            Phase::ValueReady(src) => {
                let mut rot = src.get8().rotate_left(1);
                let mut flg;
                if cpu.opcode & 0x10 == 0 {
                    flg = (rot & 0x01) << Cpu::CBIT;
                } else {
                    flg = cpu.f().get() & 0x10 >> 4;
                    masked_swap(&mut rot, &mut flg, 0x01);
                }
                flg <<= 4;
                flg |= ((rot == 0) as u8) << Cpu::ZBIT;
                cpu.f().set(flg);
                cpu.pc().inc(1);
                decode_regind8_dst(cpu, idx, rot)
            }
        }
    }

    fn rrc(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }
    fn rr(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }
    fn sla(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }
    fn sra(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }
    fn swap(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }
    fn srl(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn bit(cpu: &Cpu, phase: Phase) -> Stage {
        // BIT n3, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x7;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let offset = (cpu.opcode >> 3) & 0x7;
                let z = ((src.get8() & (1u8 << offset) == 0) as u8) << Cpu::ZBIT;
                let c = cpu.f().get() & 1u8 << Cpu::CBIT;
                let h = 1u8 << Cpu::HBIT;
                cpu.f().set(z | c | h);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn res(cpu: &Cpu, phase: Phase) -> Stage {
        // RES n3, r/m8
        let idx = cpu.opcode & 0x7;
        match phase {
            Phase::InstFetch => decode_regind8_src(cpu, idx),
            Phase::ValueReady(src) => {
                let offset = (cpu.opcode >> 3) & 0x7;
                cpu.pc().inc(1);
                decode_regind8_dst(cpu, idx, src.get8() & !(1 << offset))
            }
        }
    }

    fn set(cpu: &Cpu, phase: Phase) -> Stage {
        // SET n3, r/m8
        let idx = cpu.opcode & 0x7;
        match phase {
            Phase::InstFetch => decode_regind8_src(cpu, idx),
            Phase::ValueReady(src) => {
                let offset = (cpu.opcode >> 3) & 0x7;
                cpu.pc().inc(1);
                decode_regind8_dst(cpu, idx, src.get8() | (1 << offset))
            }
        }
    }

    #[rustfmt::skip]
    pub(super) const PREFIX_INST_TABLE: [InstOp; 0x20] = [
        /* 0o0xx */
        rl,   rrc,  rl,   rr,   sla,  sra,  swap, srl,
        /* 0o1xx */
        bit,  bit,  bit,  bit,  bit,  bit,  bit,  bit,
        /* 0o2xx */
        res,  res,  res,  res,  res,  res,  res,  res,
        /* 0o3xx */
        set,  set,  set,  set,  set,  set,  set,  set,
    ];

    #[inline(always)]
    fn cond(opcode: u8, flag: u8) -> bool {
        let test = 1u8
            << (if (opcode & 0x10) == 0x10 {
                Cpu::CBIT
            } else {
                Cpu::ZBIT
            });
        let ncond = flag & test == 0;
        let neg = opcode & 0x08 == 0;
        ncond == neg
    }

    #[inline(always)]
    fn alu(acc: u8, opd: u8, carry: bool) -> (u8, u8) {
        let (sum, c) = acc.carrying_add(opd, carry);
        let z = ((sum == 0) as u8) << Cpu::ZBIT;
        let h = (((acc & 0x0F + opd & 0x0F) & 0x10) as u8) << Cpu::HBIT;
        let c = (c as u8) << Cpu::CBIT;
        (sum, z | h | c)
    }

    #[inline(always)]
    fn negf(f: u8) -> u8 {
        (f ^ (1 << Cpu::HBIT | 1 << Cpu::CBIT)) | 1 << Cpu::NBIT
    }

    fn unimpl(cpu: &Cpu, _: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn inval(cpu: &Cpu, _: Phase) -> Stage {
        todo!("invalid instruction: {:?}", cpu)
    }

    fn nop(cpu: &Cpu, _phase: Phase) -> Stage {
        // NOP
        cpu.pc().inc(1);
        Stage::Fetch
    }

    fn ld8(cpu: &Cpu, phase: Phase) -> Stage {
        // LD rm8, rm8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let idx = cpu.opcode >> 3 & 0x07;
                cpu.pc().inc(1);
                decode_regind8_dst(cpu, idx, src.get8())
            }
        }
    }

    fn ld16(cpu: &Cpu, phase: Phase) -> Stage {
        // LD r16, n16
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem16(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => {
                let idx = cpu.opcode >> 4;
                cpu.r16(RegId16::decode(idx)).set(src.get16());
                cpu.pc().inc(2);
                Stage::Fetch
            }
        }
    }

    fn ld16sp(cpu: &Cpu, phase: Phase) -> Stage {
        // LD [a16], SP
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem16(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => {
                cpu.pc().inc(2);
                Stage::Write(OpdDst::Mem16(src.get16(), cpu.sp().get()))
            }
        }
    }

    fn ldinda(cpu: &Cpu, phase: Phase) -> Stage {
        // LD [r16+-], A;
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::None),
            Phase::ValueReady(_) => {
                let idx = cpu.opcode >> 4;
                cpu.pc().inc(1);
                decode_regind16_dst(cpu, idx, cpu.a().get())
            }
        }
    }

    fn ldaind(cpu: &Cpu, phase: Phase) -> Stage {
        // LD A, [r16+-];
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode >> 4;
                decode_regind16_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                cpu.a().set(src.get8());
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn ld8imm(cpu: &Cpu, phase: Phase) -> Stage {
        // LD ri8, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => {
                cpu.pc().inc(1);
                let idx = cpu.opcode >> 3 & 0x7;
                decode_regind8_dst(cpu, idx, src.get8())
            }
        }
    }

    fn ldhca(cpu: &Cpu, phase: Phase) -> Stage {
        // LDH [C], A
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Done8(cpu.a().get())),
            Phase::ValueReady(src) => {
                cpu.pc().inc(1);
                Stage::Write(OpdDst::Mem8(cpu.c().get().as_hiaddr(), src.get8()))
            }
        }
    }

    fn ldhia(cpu: &Cpu, phase: Phase) -> Stage {
        // LDH [a8], A
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => {
                let addr = 0xFF00 | (src.get8() as u16);
                cpu.pc().inc(1);
                Stage::Write(OpdDst::Mem8(addr, cpu.a().get()))
            }
        }
    }

    fn ldhai(cpu: &Cpu, phase: Phase) -> Stage {
        // LDH A, [a8]
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => match src {
                ReadVal::Done8(val) => {
                    let addr = 0xFF00 | (val as u16);
                    Stage::Read(OpdSrc::Mem8Ex(addr))
                }
                ReadVal::Done8Ex(val) => {
                    cpu.pc().inc(1);
                    cpu.a().set(val);
                    Stage::Fetch
                }
                _ => unreachable!(),
            },
        }
    }

    fn ldhac(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn ldia(cpu: &Cpu, phase: Phase) -> Stage {
        // LD [a16], A
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem16(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => {
                cpu.pc().inc(2);
                Stage::Write(OpdDst::Mem8(src.get16(), cpu.a().get()))
            }
        }
    }

    fn ldai(cpu: &Cpu, phase: Phase) -> Stage {
        // LD [a16], A
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem16(cpu.pc().pre_inc(1))),
            Phase::ValueReady(src) => match src {
                ReadVal::Done16(val) => {
                    let addr = val;
                    Stage::Read(OpdSrc::Mem8(addr))
                }
                ReadVal::Done8(val) => {
                    cpu.pc().inc(2);
                    cpu.a().set(val);
                    Stage::Fetch
                }
                _ => unreachable!(),
            },
        }
    }

    fn ldsphl(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn offtsp(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn inc16(cpu: &Cpu, phase: Phase) -> Stage {
        // INC r16
        let idx = cpu.opcode >> 4 & 0x03;

        match phase {
            Phase::InstFetch => decode_reg16_src(cpu, idx),
            Phase::ValueReady(src) => {
                cpu.pc().inc(1);
                decode_reg16_dst(cpu, idx, src.get16().wrapping_add(1))
            }
        }
    }

    fn dec16(cpu: &Cpu, phase: Phase) -> Stage {
        // DEC r16
        let idx = cpu.opcode >> 4 & 0x03;

        match phase {
            Phase::InstFetch => decode_reg16_src(cpu, idx),
            Phase::ValueReady(src) => {
                cpu.pc().inc(1);
                decode_reg16_dst(cpu, idx, src.get16().wrapping_sub(1))
            }
        }
    }

    fn inc8(cpu: &Cpu, phase: Phase) -> Stage {
        // INC r/m8
        let idx = cpu.opcode >> 3 & 0x7;
        match phase {
            Phase::InstFetch => decode_regind8_src(cpu, idx),
            Phase::ValueReady(src) => {
                let (inc, f) = alu(src.get8(), 1, false);
                cpu.f().set(f);
                cpu.pc().inc(1);
                decode_regind8_dst(cpu, idx, inc)
            }
        }
    }

    fn dec8(cpu: &Cpu, phase: Phase) -> Stage {
        // DEC r/m8
        let idx = cpu.opcode >> 3 & 0x7;
        match phase {
            Phase::InstFetch => decode_regind8_src(cpu, idx),
            Phase::ValueReady(src) => {
                let (dec, f) = alu(src.get8(), 1u8.wrapping_neg(), false);
                cpu.f().set(negf(f));
                cpu.pc().inc(1);
                decode_regind8_dst(cpu, idx, dec)
            }
        }
    }

    // TODO(yhr0x43): more readable rotate and shift
    fn rlca(cpu: &Cpu, phase: Phase) -> Stage {
        // RLCA
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Done8(cpu.a().get())),
            Phase::ValueReady(src) => {
                let rot = src.get8().rotate_left(1);
                let z = ((rot == 0) as u8) << Cpu::ZBIT;
                let c = (rot & 0x01) << Cpu::CBIT;
                cpu.f().set(z | c);
                cpu.pc().inc(1);
                cpu.a().set(rot);
                Stage::Fetch
            }
        }
    }

    fn rla(cpu: &Cpu, phase: Phase) -> Stage {
        // RLA
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Done8(cpu.a().get())),
            Phase::ValueReady(src) => {
                let mut rot = src.get8().rotate_left(1);
                let mut flg = cpu.f().get() >> 4 & 0x01;
                masked_swap(&mut rot, &mut flg, 0x01);
                flg <<= 4;
                flg |= ((rot == 0) as u8) << Cpu::ZBIT;
                cpu.f().set(flg);
                cpu.pc().inc(1);
                cpu.a().set(rot);
                Stage::Fetch
            }
        }
    }

    fn jr(cpu: &Cpu, phase: Phase) -> Stage {
        match phase {
            Phase::InstFetch => {
                cpu.pc().inc(1);
                Stage::Read(OpdSrc::Mem8(cpu.pc().get()))
            }
            Phase::ValueReady(src) => {
                cpu.pc().inc(src.get8().cast_signed() as u16 + 1);
                Stage::Wait(OpdDst::Done)
            }
        }
    }

    fn jrcc(cpu: &Cpu, phase: Phase) -> Stage {
        // JR cc, e8
        match phase {
            Phase::InstFetch => jr(cpu, phase),
            Phase::ValueReady(_) => {
                if cond(cpu.opcode, cpu.f().get()) {
                    jr(cpu, phase)
                } else {
                    cpu.pc().inc(1);
                    Stage::Fetch
                }
            }
        }
    }

    fn jp(cpu: &Cpu, phase: Phase) -> Stage {
        // JP n16
        match phase {
            Phase::InstFetch => {
                cpu.pc().inc(1);
                Stage::Read(OpdSrc::Mem16(cpu.pc().get()))
            }
            Phase::ValueReady(src) => {
                cpu.pc().set(src.get16());
                Stage::Wait(OpdDst::Done)
            }
        }
    }

    fn jpcc(cpu: &Cpu, phase: Phase) -> Stage {
        // JP cc, n16
        match phase {
            Phase::InstFetch => jp(cpu, phase),
            Phase::ValueReady(_) => {
                if cond(cpu.opcode, cpu.f().get()) {
                    jp(cpu, phase)
                } else {
                    cpu.pc().inc(1);
                    Stage::Fetch
                }
            }
        }
    }

    fn jphl(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn add(cpu: &Cpu, phase: Phase) -> Stage {
        // ADD A, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let opd1 = cpu.a().get();
                let opd2 = src.get8();
                let (sum, c) = opd1.carrying_add(opd2, false);
                cpu.a().set(sum);
                let z = ((sum == 0) as u8) << Cpu::ZBIT;
                let n = 1u8 << Cpu::NBIT;
                let h = (((opd1 & 0x0F + opd2 & 0x0F) & 0x10 != 0) as u8) << Cpu::HBIT;
                let c = (c as u8) << Cpu::CBIT;
                cpu.f().set(z | n | h | c);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn adc(cpu: &Cpu, phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn sub(cpu: &Cpu, phase: Phase) -> Stage {
        // SUB A, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let opd1 = cpu.a().get();
                let opd2 = src.get8();
                let (diff, c) = opd1.borrowing_sub(opd2, false);
                cpu.a().set(diff);
                let z = ((diff == 0) as u8) << Cpu::ZBIT;
                let n = 1u8 << Cpu::NBIT;
                let h = ((opd1 & 0x0F < opd2 & 0x0F) as u8) << Cpu::HBIT;
                let c = (c as u8) << Cpu::CBIT;
                cpu.f().set(z | n | h | c);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn sbc(cpu: &Cpu, phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn and(cpu: &Cpu, phase: Phase) -> Stage {
        // AND A, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let and = cpu.a().get() & src.get8();
                cpu.a().set(and);
                let z = ((and == 0) as u8) << Cpu::ZBIT;
                let h = 1 << Cpu::HBIT;
                cpu.f().set(z | h);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn xor(cpu: &Cpu, phase: Phase) -> Stage {
        // XOR A, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let xor = cpu.a().get() ^ src.get8();
                cpu.a().set(xor);
                let z = ((xor == 0) as u8) << Cpu::ZBIT;
                cpu.f().set(z);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn or(cpu: &Cpu, phase: Phase) -> Stage {
        // OR A, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let or = cpu.a().get() | src.get8();
                cpu.a().set(or);
                let z = ((or == 0) as u8) << Cpu::ZBIT;
                cpu.f().set(z);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn cp(cpu: &Cpu, phase: Phase) -> Stage {
        // CP A, r/m8
        match phase {
            Phase::InstFetch => {
                let idx = cpu.opcode & 0x07;
                decode_regind8_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                let opd1 = cpu.a().get();
                let opd2 = src.get8();
                let (diff, c) = opd1.borrowing_sub(opd2, false);
                let z = ((diff == 0) as u8) << Cpu::ZBIT;
                let n = 1u8 << Cpu::NBIT;
                let h = ((opd1 & 0x0F < opd2 & 0x0F) as u8) << Cpu::HBIT;
                let c = (c as u8) << Cpu::CBIT;
                cpu.f().set(z | n | h | c);
                cpu.pc().inc(1);
                Stage::Fetch
            }
        }
    }

    fn addimm(cpu: &Cpu, phase: Phase) -> Stage {
        // ADD A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => add(cpu, phase),
        }
    }

    fn adcimm(cpu: &Cpu, phase: Phase) -> Stage {
        // ADC A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => adc(cpu, phase),
        }
    }

    fn subimm(cpu: &Cpu, phase: Phase) -> Stage {
        // SUB A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => sub(cpu, phase),
        }
    }

    fn sbcimm(cpu: &Cpu, phase: Phase) -> Stage {
        // SBC A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => sbc(cpu, phase),
        }
    }

    fn andimm(cpu: &Cpu, phase: Phase) -> Stage {
        // AND A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => and(cpu, phase),
        }
    }

    fn xorimm(cpu: &Cpu, phase: Phase) -> Stage {
        // XOR A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => xor(cpu, phase),
        }
    }

    fn orimm(cpu: &Cpu, phase: Phase) -> Stage {
        // OR A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => or(cpu, phase),
        }
    }

    fn cpimm(cpu: &Cpu, phase: Phase) -> Stage {
        // CP A, n8
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem8(cpu.pc().pre_inc(1))),
            Phase::ValueReady(_) => cp(cpu, phase),
        }
    }

    fn rst(cpu: &Cpu, _phase: Phase) -> Stage {
        todo!("inst {:?}", cpu)
    }

    fn ret(cpu: &Cpu, phase: Phase) -> Stage {
        // RET; RETI
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem16(cpu.sp().get())),
            Phase::ValueReady(src) => {
                if cpu.opcode & 0x10 == 0x10 {
                    cpu.ime.set(true);
                }
                cpu.sp().inc(2);
                cpu.pc().set(src.get16());
                Stage::Wait(OpdDst::Done)
            }
        }
    }

    fn retcc(cpu: &Cpu, phase: Phase) -> Stage {
        // RET cc
        match phase {
            Phase::InstFetch => ret(cpu, phase),
            Phase::ValueReady(_) => {
                if cond(cpu.opcode, cpu.f().get()) {
                    ret(cpu, phase)
                } else {
                    cpu.pc().inc(1);
                    Stage::Fetch
                }
            }
        }
    }

    fn call(cpu: &Cpu, phase: Phase) -> Stage {
        // CALL a16
        match phase {
            Phase::InstFetch => {
                cpu.pc().inc(1);
                Stage::Read(OpdSrc::Mem16(cpu.pc().post_inc(2)))
            }
            Phase::ValueReady(src) => Stage::Wait(OpdDst::Mem16(
                cpu.sp().pre_dec(2),
                cpu.pc().replace(src.get16()),
            )),
        }
    }

    fn callcc(cpu: &Cpu, phase: Phase) -> Stage {
        // CALL cc, a16
        match phase {
            Phase::InstFetch => call(cpu, phase),
            Phase::ValueReady(_) => {
                if cond(cpu.opcode, cpu.f().get()) {
                    call(cpu, phase)
                } else {
                    Stage::Fetch
                }
            }
        }
    }

    fn pop(cpu: &Cpu, phase: Phase) -> Stage {
        // POP r16
        match phase {
            Phase::InstFetch => Stage::Read(OpdSrc::Mem16(cpu.sp().get())),
            Phase::ValueReady(src) => {
                let idx = cpu.opcode >> 4 & 0x3;
                cpu.pc().inc(1);
                cpu.sp().inc(2);
                decode_reg16_dst(cpu, idx, src.get16())
            }
        }
    }

    fn push(cpu: &Cpu, phase: Phase) -> Stage {
        // PUSH r16
        match phase {
            Phase::InstFetch => {
                let idx = (cpu.opcode >> 4) & 0x3;
                decode_reg16_src(cpu, idx)
            }
            Phase::ValueReady(src) => {
                cpu.pc().inc(1);
                Stage::Wait(OpdDst::Mem16(cpu.sp().pre_dec(2), src.get16()))
            }
        }
    }

    fn di(cpu: &Cpu, _phase: Phase) -> Stage {
        // DI
        cpu.ime.set(false);
        cpu.pc().inc(1);
        Stage::Fetch
    }

    fn ei(cpu: &Cpu, _phase: Phase) -> Stage {
        // EI
        cpu.ime_enable.set(ImeSet::Init);
        cpu.pc().inc(1);
        Stage::Fetch
    }

    /* PREFIX is inval because it is handled by outer fetch loop */
    #[rustfmt::skip]
    pub(super) const INST_TABLE: [InstOp; 0x100] = {
        [
            /* 0x0x */
            nop,    ld16,   ldinda, inc16,  inc8,   dec8,   ld8imm, rlca,
            ld16sp, unimpl, ldaind, dec16,  inc8,   dec8,   ld8imm, unimpl,
            /* 0x1x */
            unimpl, ld16,   ldinda, inc16,  inc8,   dec8,   ld8imm, rla,
            jr,     unimpl, ldaind, dec16,  inc8,   dec8,   ld8imm, unimpl,
            /* 0x2x */
            jrcc,   ld16,   ldinda, inc16,  inc8,   dec8,   ld8imm, unimpl,
            jrcc,   unimpl, ldaind, dec16,  inc8,   dec8,   ld8imm, unimpl,
            /* 0x3x */
            jrcc,   ld16,   ldinda, inc16,  inc8,   dec8,   ld8imm, unimpl,
            jrcc,   unimpl, ldaind, dec16,  inc8,   dec8,   ld8imm, unimpl,
            /* 0x4x */
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            /* 0x5x */
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            /* 0x6x */
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            /* 0x7x */
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    unimpl, ld8,
            ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,    ld8,
            /* 0x8x */
            add,    add,    add,    add,    add,    add,    add,    add,
            adc,    adc,    adc,    adc,    adc,    adc,    adc,    adc,
            /* 0x9x */
            sub,    sub,    sub,    sub,    sub,    sub,    sub,    sub,
            sbc,    sbc,    sbc,    sbc,    sbc,    sbc,    sbc,    sbc,
            /* 0xAx */
            and,    and,    and,    and,    and,    and,    and,    and,
            xor,    xor,    xor,    xor,    xor,    xor,    xor,    xor,
            /* 0xBx */
            or,     or,     or,     or,     or,     or,     or,     or,
            cp,     cp,     cp,     cp,     cp,     cp,     cp,     cp,
            /* 0xCx */
            retcc,  pop,    jpcc,   jp,     callcc, push,   addimm, rst,
            retcc,  ret,    jpcc,   inval,  callcc, call,   adcimm, rst,
            /* 0xDx */
            retcc,  pop,    jpcc,   inval,  callcc, push,   subimm, rst,
            retcc,  ret,    jpcc,   inval,  callcc, inval,  sbcimm, rst,
            /* 0xEx */
            ldhia,  pop,    ldhca,  inval,  inval,  push,   andimm, rst,
            offtsp, jphl,   ldia,   inval,  inval,  inval,  xorimm, rst,
            /* 0xFx */
            ldhai,  pop,    ldhac,  di,     inval,  push,   orimm,  rst,
            offtsp, ldsphl, ldai,   ei,     inval,  inval,  cpimm,  rst,
        ]
    };
}

enum IntrStage {
    None,
    Init(u16),
    Wait(u16),
    Exec(u16),
}

#[derive(Clone, Copy)]
enum ImeSet {
    None,
    Init,
    Wait,
}

pub struct Cpu {
    regs: [u16; 6],
    pub(self) ime: Cell<bool>, /* interrupt master enable */
    pub(self) stop: Cell<bool>,
    pub(self) halt: Cell<bool>,

    /* interrupt states */
    intr_stage: IntrStage,
    pub(self) ime_enable: Cell<ImeSet>,

    /* sub-instruction M-cycles state */
    pub(self) opcode: u8, /* executing opcode */
    pub(self) instop: InstOp,
    stage: Stage,
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

    pub const fn init(&mut self) {
        self.regs = [0; 6];
        self.ime = Cell::new(false);
        self.stop = Cell::new(false);
        self.halt = Cell::new(false);
        self.intr_stage = IntrStage::None;;
        self.ime_enable = Cell::new(ImeSet::None);

        self.opcode = 0 ;
        self.stage = Stage::Fetch;
        self.instop = inst::INST_TABLE[0];
    }

    const ZBIT: u8 = 7;
    const NBIT: u8 = 6;
    const HBIT: u8 = 5;
    const CBIT: u8 = 4;

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

    pub fn intr(&mut self, addr: bus::Addr) -> bool {
        if self.ime.get() {
            self.intr_stage = IntrStage::Init(addr);
            true
        } else {
            false
        }
    }

    fn intr_step(&mut self) -> ControlFlow<()> {
        self.ime_enable.set(match self.ime_enable.get() {
            ImeSet::None => ImeSet::None,
            ImeSet::Init => ImeSet::Wait,
            ImeSet::Wait => {
                self.ime.set(true);
                ImeSet::None
            }
        });
        self.intr_stage = match self.intr_stage {
            IntrStage::None => IntrStage::None,
            IntrStage::Init(addr) => IntrStage::Wait(addr),
            IntrStage::Wait(addr) => IntrStage::Exec(addr),
            IntrStage::Exec(addr) => {
                // println!("intr from {:04X} to {addr:04X}", self.pc().get());
                self.stage = Stage::Read(OpdSrc::Done16(addr));
                self.instop = inst::INST_TABLE[0xCD]; // call
                IntrStage::None
            }
        };
        match self.intr_stage {
            IntrStage::None => ControlFlow::Continue(()),
            _ => ControlFlow::Break(()),
        }
    }

    /*
     * one tick is one M-cycle, 4 T states in Z80 terms
     * one M-cycle can only have at most 1 bus read/write
     */
    pub fn tick(&mut self, bus: &mut bus::Bus) -> bool {
        if self.stop.get() || self.halt.get() {
            return false;
        }

        let mut memop = false;
        self.stage = match self.stage {
            Stage::Fetch => {
                // if matches!(self.intr_step(), ControlFlow::Break(_)) {
                //     return true;
                // }

                self.opcode = bus.read(self.pc().get());
                memop = true;
                if self.opcode == 0xCB {
                    // PREFIX
                    self.pc().inc(1);
                    Stage::FetchPrefixed
                } else {
                    self.instop = inst::INST_TABLE[self.opcode as usize];
                    (self.instop)(&self, Phase::InstFetch)
                }
            }
            Stage::FetchPrefixed => {
                self.opcode = bus.read(self.pc().get());
                memop = true;
                self.instop = inst::PREFIX_INST_TABLE[(self.opcode >> 3) as usize];
                (self.instop)(&self, Phase::InstFetch)
            }
            Stage::Read(_) | Stage::Write(_) => self.stage,
            Stage::Wait(dst) => {
                memop = true;
                Stage::Write(dst)
            }
        };

        if let Stage::Read(src) = self.stage {
            if src.ready() || !memop {
                let src = src.read_step(bus);
                memop = true;
                self.stage = if src.ready() {
                    (self.instop)(&self, Phase::ValueReady(src.into()))
                } else {
                    Stage::Read(src)
                }
            }
        }

        if let Stage::Write(dst) = self.stage {
            if dst.ready() || !memop {
                let dst = dst.write_step(bus);
                self.stage = if dst.ready() {
                    Stage::Fetch
                } else {
                    Stage::Write(dst)
                }
            }
        };

        true
    }
}
