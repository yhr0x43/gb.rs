use core::ops::ControlFlow;

use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::intr::IntrSrc;

use crate::*;

pub(crate) const FRAME_WIDTH: usize = 160;
pub(crate) const FRAME_HEIGHT: usize = 144;
// FRAME_BUFFER has data in 8-bit RGBA format
// thus buffersize is pixel count time 4
// chosen to better interop with WebAPI ImageData
// which assumes 8-bit RGBA in Uint8ClampedArray
pub(crate) const FRAME_BUFFER_SIZE: usize = gb::FRAME_WIDTH * gb::FRAME_HEIGHT * 4;

pub(crate) const MAX_CART_ROM_SIZE: usize = 0x800000;

pub struct GB {
    pub(crate) bus: Bus,
    pub(crate) cpu: Cpu,
    tick: u128,

    pub(crate) paused: bool,
}

impl GB {
    pub fn init(&mut self) {
        self.cpu.init();
        self.bus.init();
        self.tick = 0;
        self.paused = false;

        self.bus.boot_map = false;
        self.cpu.pc().set(0x0100);
    }

    pub fn tick(&mut self) -> ControlFlow<()> {
        if self.paused {
            return ControlFlow::Break(());
        }

        self.cpu.tick(&mut self.bus);

        if self.bus.ppu.tick() {
            self.bus.intr.raise(IntrSrc::VBlank);
        }

        self.bus.apu.tick(0);
        self.bus.timer.tick(&mut self.bus.intr);

        self.bus.intr.tick(&mut self.cpu);

        // if matches!(self.cpu.pc().get(), 0x671A..0x6720) && self.cpu.de().get() < 10
        // // if matches!(self.cpu.pc().get(), 0x671A..0x6720)
        // {
        //     println!("{}: {:?}", self.tick, self.cpu);
        // }

        self.tick += 1;
        ControlFlow::Continue(())
    }

    pub fn stack_dump(&self) {
        let sp = self.cpu.sp().get();
        if matches!(sp, 0xDF00..0xE000) && (sp - 1).is_multiple_of(2) {
            println!("stack dump:");
            for s in (sp..0xE000).step_by(2) {
                println!("{:04X}", self.bus.read(s) as u16 | (self.bus.read(s+1) as u16) << 8);
            }
        }
    }
}
