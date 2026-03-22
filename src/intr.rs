use crate::cpu;

pub(crate) enum IntrSrc {
    VBlank = 0x01,
    LCD = 0x02,
    Timer = 0x04,
    Serial = 0x08,
    Joypad = 0x10,
}

pub(crate) struct Intr {
    reg_ie: u8,
    reg_if: u8,
}

impl Intr {
    pub const fn init(&mut self) { }
    
    pub fn read_ie(&self) -> u8 {
        self.reg_ie
    }

    pub fn write_ie(&mut self, val: u8) {
        self.reg_ie = val & 0x1F;
    }

    pub fn read_if(&self) -> u8 {
        self.reg_if
    }

    pub fn write_if(&mut self, val: u8) {
        self.reg_if = val & 0x1F;
    }

    pub fn raise(&mut self, intr: IntrSrc) {
        let intr = intr as u8;
        if self.reg_ie & intr != 0 {
            self.reg_if |= intr;
        }
    }

    pub fn tick(&mut self, cpu: &mut cpu::Cpu) {
        let trail = (self.reg_if & self.reg_ie).trailing_zeros() as u16;
        if trail > 4 {
            return;
        }
        if cpu.intr(0x0040 + trail * 0x8) {
            self.reg_if &= !(1 << trail);
        }
    }
}
