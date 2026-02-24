pub struct Bus {
    boot_rom: [u8; 0x100],
    wram: [u8; 8*0x1000],

    ime: bool,  /* interrupt master enable */
    reg_ie: u8, /* interrupt enable */
    reg_if: u8, /* interrupt flag */
}

pub enum IntrSource {
    JOYPAD,
}

impl Bus {
    pub fn new(boot_rom: &[u8; 0x100]) -> Self {
        Bus {
            boot_rom: *boot_rom,
            wram: [0; 8*0x1000],
            ime: false, reg_ie: 0, reg_if: 0,
        }
    }

    pub fn intr_poll(&self) -> bool {
        self.ime && (self.reg_ie & self.reg_if) != 0
    }

    pub fn intr_raise(source: IntrSource) {
        match source {
            _ => panic!("unimpl interrupt"),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        let uaddr: usize = addr.into();

        if (addr & 0xE000) == 0xC000 { // C000 <= addr <= DFFF
            self.wram[uaddr - 0xC000]
        } else if (addr & 0xFF00) == 0 {
            self.boot_rom[uaddr]
        } else {
            panic!("unimpl memory address")
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        if (addr & 0xE000) == 0xC000 { // C000 <= addr <= DFFF
            self.wram[(addr - 0xC000) as usize] = val
        } else if addr == 0xFFFF {
            panic!("unimpl intr mask")
        } else {
            panic!("unimpl memory address")
        }
    }
}
