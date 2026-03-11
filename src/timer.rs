use crate::intr::{Intr, IntrSrc};

pub(crate) struct Timer {
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,

    tick_count: u8,
}

impl Timer {
    pub const fn new() -> Timer {
        Timer {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,

            tick_count: 0,
        }
    }

    pub fn tick(&mut self, intr: &mut Intr) {
        self.tick_count = self.tick_count.wrapping_add(1);

        if self.tick_count % 64 == 0 {
            self.div = self.div.wrapping_add(1);
        }

        if self.tac & 0x40 != 0 && self.tick_count % (4 << ((self.tac - 1) & 0x3) * 2) == 0 {
            if self.tima == 0xFF {
                self.tima = self.tma;
                intr.raise(IntrSrc::Timer);
            } else {
                self.tima += 1;
            }
        }
    }
}
