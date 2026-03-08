use core::ops::ControlFlow;

use crate::cpu::Cpu;
use crate::bus::Bus;

use crate::*;

pub(crate) const FRAME_WIDTH: usize = 144;
pub(crate) const FRAME_HEIGHT: usize = 160;
// FRAME_BUFFER has data in 8-bit RGBA format
// thus buffersize is pixel count time 4
// chosen to better interop with WebAPI ImageData
// which assumes 8-bit RGBA in Uint8ClampedArray
pub(crate) const FRAME_BUFFER_SIZE: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;

pub(crate) const MAX_CART_ROM_SIZE: usize = 0x800000;

pub struct GB {
    pub(crate) bus: Bus,
    cpu: Cpu,
    tick: u128,

    pub(crate) paused: bool,
}

impl GB {
    pub const fn new() -> GB {
        GB {
            cpu: Cpu::new(),
            bus: Bus::new(),
            tick: 0,
            paused: false,
        }
    }

    pub fn write_button_state(&mut self, info: u8) {
        todo!("{}", info)
    }

    pub fn cycle(&mut self) -> ControlFlow<()> {
        if self.paused {
            return ControlFlow::Break(());
        }

        if !self.cpu.cycle(&mut self.bus) {
            return ControlFlow::Break(());
        }

        println!("{}: {:?}", self.tick, self.cpu);
        if self.tick > 200000 {
            self.paused = true;
        }

        // if self.cpu.pc().get() > 0x10 {
        //     println!("{}: {:?}", self.tick, self.cpu);
        // }
        self.tick += 1;

        ControlFlow::Continue(())
    }
}

