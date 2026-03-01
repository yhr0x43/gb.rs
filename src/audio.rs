pub struct Apu {
    nr10: u8, //Ch 1 sweep register
    nr11: u8,
    nr12: u8,
    nr13: u8,
    nr14: u8,
    
    nr21: u8, //Ch 2 sound length/wave pattern duty
    nr22: u8,
    nr23: u8,
    nr24: u8,

    nr30: u8, //Ch 3 sound on/off
    nr31: u8,
    nr32: u8,
    nr33: u8,
    nr34: u8,

    nr41: u8, //Ch 4 sound length
    nr42: u8,
    nr43: u8,
    nr44: u8,

    nr50: u8, //Mixer
    nr51: u8, 
    nr52: u8, 

    wave_pattern_ram: [u8; 0x10], 
    sample_rate: u32, cycle_count: u64 
}

impl Apu { 
    pub const fn new () -> Self {
        Apu {
            nr10: 0, nr11: 0, nr12: 0, nr13: 0, nr14: 0,
            nr21: 0, nr22: 0, nr23: 0, nr24: 0,
            nr30: 0, nr31: 0, nr32: 0, nr33: 0, nr34: 0,
            nr41: 0, nr42: 0, nr43: 0, nr44: 0,
            nr50: 0, nr51: 0, nr52: 0,
            wave_pattern_ram: [0; 0x10],
            sample_rate: 44100,
            cycle_count: 0
        }
    }
    pub fn tick(&mut self, cycles: u8) {
        self.cycle_count += cycles as u64; // Update frequency and counters and stuff
    }

    pub fn writeByte(&mut self, addr: u16, val: u8) {
        match addr {
            0xFF10 => self.nr10 = val,
            0xFF11 => self.nr11 = val,
            0xFF12 => self.nr12 = val,
            0xFF13 => self.nr13 = val,
            0xFF14 => self.nr14 = val,

            0xFF16 => self.nr21 = val,
            0xFF17 => self.nr22 = val,
            0xFF18 => self.nr23 = val,
            0xFF19 => self.nr24 = val,

            0xFF1A => self.nr30 = val,
            0xFF1B => self.nr31 = val,
            0xFF1C => self.nr32 = val,
            0xFF1D => self.nr33 = val,
            0xFF1E => self.nr34 = val,

            0xFF20 => self.nr41 = val,
            0xFF21 => self.nr42 = val,
            0xFF22 => self.nr43 = val,
            0xFF23 => self.nr44 = val,

            0xFF24 => self.nr50 = val,
            0xFF25 => self.nr51 = val,
            0xFF26 => self.nr52 = val,

            0xFF30..=0xFF3F => self.wave_pattern_ram[(addr - 0xFF30) as usize] = val,

            _ => unreachable!("invalid APU register address {:04X}", addr)
        }
    }

    pub fn readByte(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => self.nr10,
            0xFF11 => self.nr11,
            0xFF12 => self.nr12,
            0xFF13 => self.nr13,
            0xFF14 => self.nr14,

            0xFF16 => self.nr21,
            0xFF17 => self.nr22,
            0xFF18 => self.nr23,
            0xFF19 => self.nr24,

            0xFF1A => self.nr30,
            0xFF1B => self.nr31,
            0xFF1C => self.nr32,
            0xFF1D => self.nr33,
            0xFF1E => self.nr34,

            0xFF20 => self.nr41,
            0xFF21 => self.nr42,
            0xFF22 => self.nr43,
            0xFF23 => self.nr44,

            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => self.nr52, // Checks which channels are on and playing sound

            0xFF30..=0xFF3F => self.wave_pattern_ram[(addr - 0xFF30) as usize],

            _ => unreachable!("invalid APU register address {:04X}", addr)
        }
    }
    
    pub fn nextSample(&mut self) -> (f32, f32) {
        // Temp. Just return silence for now
        (0.0, 0.0)
    }
}
