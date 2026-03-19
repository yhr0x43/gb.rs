use core::ops::ControlFlow;

use crate::bus::Bus;
use crate::cpu::Cpu;

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
    pub const fn init(&mut self) {
        self.cpu.init();
        self.bus.init();
        self.tick = 0;
        self.paused = false;
    }

    pub fn tick(&mut self) -> ControlFlow<()> {
        if self.paused {
            return ControlFlow::Break(());
        }

        if !self.cpu.tick(&mut self.bus) {
            return ControlFlow::Break(());
        }

        self.bus.ppu.tick();
        self.bus.apu.tick(0);
        self.bus.timer.tick(&mut self.bus.intr);

        self.bus.intr.tick(&mut self.cpu);

        // if matches!(self.cpu.pc().get(), 0x0A0E..0x0A89 | 0x0E45..0x0E5A)
        //     || matches!(self.cpu.sp().get(), 0xDFF9)
        // if self.tick > 1000000
        // {
        //     println!("{}: {:?}", self.tick, self.cpu);
        // }

        self.tick += 1;
        ControlFlow::Continue(())
    }
}
