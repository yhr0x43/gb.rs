pub type Addr = u16;

pub struct Bus {
    boot_rom: [u8; 0x100], /*unsigned8; Hex notation*/
    vram: [u8; 0x2000], 
    wram: [u8; 0x2000],
}

impl Bus {
    pub fn new(boot_rom: &[u8; 0x100]) -> Self {
        Bus {
            boot_rom: *boot_rom,
            vram: [0; 0x2000],
            wram: [0; 0x2000],
        }
    }

    pub fn read(&self, addr: Addr) -> u8 {
        let uaddr: usize = addr.into();

        if addr & 0xE000 == 0xC000 { // C000 <= addr <= DFFF
            self.wram[uaddr - 0xC000]
        } else if addr & 0xE000 == 0x8000 { // 8000 <= addr <= 9FFF
            self.vram[uaddr - 0x8000]
        } else if (addr & 0xFF00) == 0 {
            self.boot_rom[uaddr]
        } else {
            todo!("memory address {:04X}", addr)
        }
    }

    pub fn write(&mut self, addr: Addr, val: u8) {
        let uaddr: usize = addr.into();

        if (addr & 0xE000) == 0xC000 { // C000 <= addr <= DFFF
            self.wram[uaddr - 0xC000] = val
        } else if addr & 0xE000 == 0x8000 { // 8000 <= addr <= 9FFF
            self.vram[uaddr - 0x8000] = val
        } else if addr == 0xFFFF {
            todo!("intr mask")
        } else {
            todo!("memory address {:04X}", addr)
        }
    }
}
