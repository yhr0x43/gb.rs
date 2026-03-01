use crate::audio;

pub type Addr = u16;

pub struct Bus {
    boot_rom: [u8; 0x100], /*unsigned8; Hex notation*/
    vram: [u8; 0x2000], 
    wram: [u8; 0x2000],
    apu: audio::Apu,

    joy_p1: u8,
    action_buttons: u8, // A, B, Select, Start. 0 = Pressed; 1 = Not pressed.
    direction_buttons: u8, // Right, Left, Up, Down. 0 = Pressed; 1 = Not pressed.


}

impl Bus {
    pub const fn new(boot_rom: &[u8; 0x100]) -> Self {
        Bus {
            boot_rom: *boot_rom,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            apu: audio::Apu::new(),
            joy_p1: 0xFF, 
            action_buttons: 0x0F,
            direction_buttons: 0x0F,
        }
    }

    pub fn read(&self, addr: Addr) -> u8 {
        let uaddr: usize = addr.into();

        match addr {
            0x0000..0x0100 => self.boot_rom[uaddr],
            0x8000..0xA000 => self.vram[uaddr - 0x8000],
            0xC000..0xE000 => self.wram[uaddr - 0xC000],
            0xFF10..0xFF40 => self.apu.readByte(addr),
            0xFF00 => {
                let mut result = self.joy_p1 | 0xC0; // Upper 2 bits always read as 1
                if self.joy_p1 & 0x10 == 0 { result &= self.action_buttons; }
                if self.joy_p1 & 0x20 == 0 { result &= self.direction_buttons; }
                result
                }
            }
            _ => todo!("memory address read {:04X}", addr),
        }

    pub fn write(&mut self, addr: Addr, val: u8) {
        let uaddr: usize = addr.into();
        match addr {
            0x8000..0xA000 => self.vram[uaddr - 0x8000] = val,
            0xC000..0xE000 => self.wram[uaddr - 0xC000] = val,
            0xFF10..0xFF40 => self.apu.writeByte(addr, val),
            0xFF00 => self.joy_p1 = (val & 0x30) | 0xC0, 
            _ => todo!("memory address write {:04X}", addr),
        };
    }
}




