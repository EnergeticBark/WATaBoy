use crate::addressable::Addressable;
use crate::joypad::{ButtonsHeld, Joyp};
use crate::mbc::Mbc;
use crate::ppu::Ppu;
use crate::timers::Timers;

use hw_constants::io_regs::{BGP, LCDC, LY, LYC, OBP0, OBP1, SCX, SCY, STAT, WX, WY};
use hw_constants::{OAM_END, OAM_START, PostBoot, VRAM_END, VRAM_START, io_regs};
use log::info;
use rkyv::{Archive, Deserialize, Serialize, with::Skip};

#[derive(Archive, Deserialize, Serialize)]
pub struct AddressBus {
    pub buffer: Box<[u8; hw_constants::MEM_MAP_SIZE]>,
    timers: Timers,
    #[rkyv(with = Skip)]
    pub ppu: Ppu,
    mbc: Mbc,
    half_ticked: bool,
    #[rkyv(with = Skip)]
    pub buttons_held: ButtonsHeld,
}

impl AddressBus {
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.buffer[0..0x8000].copy_from_slice(&rom[0..0x8000]);
        self.mbc.load_rom(rom);
    }

    // TODO: delegate MBC bank switches.
    pub fn read_byte(&self, index: u16) -> u8 {
        match index {
            VRAM_START..VRAM_END
            | OAM_START..OAM_END
            | LCDC
            | STAT
            | SCY
            | SCX
            | LY
            | LYC
            | BGP
            | OBP0
            | OBP1
            | WY
            | WX => self.ppu.read_byte(index),
            _ => self.buffer[index as usize],
        }
    }

    pub fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            // Delegate write in the ROM range and the SRAM range to the MBC.
            0x0000..0x8000 | 0xA000..0xC000 => {
                self.mbc
                    .write_byte(self.buffer.as_mut_array().unwrap(), index, value);
            }

            // Delegate writes to VRAM and OAM to the PPU.
            VRAM_START..VRAM_END | OAM_START..OAM_END => self.ppu.write_byte(index, value),

            // Initiate OAM transfer.
            0xFF46 => {
                // Actually write the value to this address before starting the OAM DMA transfer.
                self.buffer[index as usize] = value;

                // TODO: Accurately make this take a few cycles.
                info!(target: "oam_events", "DMA Transfer from 0x{value:X}00!");
                let oam_size = 0xA0;
                let src_start = u16::from_le_bytes([0x00, value]) as usize;
                let src_end = src_start + oam_size;

                self.ppu
                    .oam
                    .copy_from_slice(&self.buffer[src_start..src_end]);
            }

            // Certain I/O addresses only use certain bits. Bits which go unused are pulled high.
            // See Appendix B: https://gekkio.fi/files/gb-docs/gbctr.pdf
            io_regs::JOYP => {
                // Lower nibble is read-only, only set select bits.
                // See: https://gbdev.io/pandocs/Joypad_Input.html#ff00--p1joyp-joypad
                let written = Joyp::from_bits(value);
                self.buffer[index as usize] = Joyp::from_bits(self.buffer[index as usize])
                    .with_select_buttons(written.select_buttons())
                    .with_select_dpad(written.select_dpad())
                    .into_bits();

                self.update_joypad();
            }
            io_regs::SC => self.buffer[index as usize] = value | 0b0111_1110,
            io_regs::TAC => self.buffer[index as usize] = value | 0b1111_1000,
            io_regs::DIV => self.timers.system_clock = 0,
            io_regs::IF => self.buffer[index as usize] = value | 0b1110_0000,

            io_regs::NR10 => self.buffer[index as usize] = value | 0b1000_0000,
            io_regs::NR30 => self.buffer[index as usize] = value | 0b0111_1111,
            io_regs::NR32 => self.buffer[index as usize] = value | 0b1001_1111,
            io_regs::NR41 => self.buffer[index as usize] = value | 0b1100_0000,
            io_regs::NR44 => self.buffer[index as usize] = value | 0b0011_1111,
            io_regs::NR52 => self.buffer[index as usize] = value | 0b0111_0000,

            // Delegate PPU registers to the PPU.
            LCDC => self.ppu.write_byte(index, value),
            // Still needed until I can update interrupts without passing in all memory :(.
            io_regs::STAT => {
                self.ppu.write_byte(index, value);

                if !self.ppu.disabled {
                    self.ppu.update_stat_interrupt(&mut self.buffer);
                }
            }
            SCY | SCX => self.ppu.write_byte(index, value),
            LY => (),
            LYC => {
                self.ppu.write_byte(index, value);

                if !self.ppu.disabled {
                    self.ppu.update_stat_interrupt(&mut self.buffer);
                }
            }
            BGP | OBP0 | OBP1 | WY | WX => self.ppu.write_byte(index, value),

            // There is *nothing* at these addresses, so they don't have names.
            // Their bits are always pulled high.
            0xFF03 | 0xFF08..0xFF0F | 0xFF15 | 0xFF1F | 0xFF27..0xFF30 | 0xFF4C..0xFF80 => {
                self.buffer[index as usize] = value | 0b1111_1111;
            }
            _ => self.buffer[index as usize] = value,
        }
    }

    pub fn half_increment_timers(&mut self) {
        for _ in 0..2 {
            self.ppu.tick(self.buffer.as_mut_array().unwrap());
        }

        if !self.half_ticked {
            self.half_ticked = true;
            return;
        }
        self.half_ticked = false;

        self.timers
            .update_timer_counter(self.buffer[io_regs::TIMA as usize]);
        self.timers
            .update_timer_modulo(self.buffer[io_regs::TMA as usize]);
        self.timers
            .update_timer_control(self.buffer[io_regs::TAC as usize]);

        self.timers.increment(1);

        self.buffer[io_regs::DIV as usize] = self.timers.div();
        self.buffer[io_regs::TIMA as usize] = self.timers.tima();

        if self.timers.process_interrupt() {
            self.buffer[io_regs::IF as usize] |= 0b0000_0100;
        }
    }

    pub fn increment_timers(&mut self, m_cycles: u16) {
        for _ in 0..m_cycles * 4 {
            self.ppu.tick(self.buffer.as_mut_array().unwrap());
        }

        self.timers
            .update_timer_counter(self.buffer[io_regs::TIMA as usize]);
        self.timers
            .update_timer_modulo(self.buffer[io_regs::TMA as usize]);
        self.timers
            .update_timer_control(self.buffer[io_regs::TAC as usize]);

        self.timers.increment(m_cycles);

        self.buffer[io_regs::DIV as usize] = self.timers.div();
        self.buffer[io_regs::TIMA as usize] = self.timers.tima();

        if self.timers.process_interrupt() {
            self.buffer[io_regs::IF as usize] |= 0b0000_0100;
        }
    }

    pub(crate) fn update_joypad(&mut self) {
        let mut joypad = Joyp::from_bits(self.buffer[io_regs::JOYP as usize]);
        if !joypad.select_buttons() {
            joypad.set_start_down(!self.buttons_held.start);
            joypad.set_select_up(!self.buttons_held.select);
            joypad.set_b_left(!self.buttons_held.b);
            joypad.set_a_right(!self.buttons_held.a);
        }
        if !joypad.select_dpad() {
            joypad.set_start_down(!self.buttons_held.down);
            joypad.set_select_up(!self.buttons_held.up);
            joypad.set_b_left(!self.buttons_held.left);
            joypad.set_a_right(!self.buttons_held.right);
        }
        // TODO: Fire the joypad interrupt on a high-to-low change
        self.buffer[io_regs::JOYP as usize] = joypad.into_bits();
    }
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            buffer: vec![0; hw_constants::MEM_MAP_SIZE]
                .into_boxed_slice()
                .try_into()
                .unwrap(),
            timers: Timers::default(),
            ppu: Ppu::default(),
            mbc: Mbc::default(),
            half_ticked: false,
            buttons_held: ButtonsHeld::default(),
        }
    }
}

impl PostBoot for AddressBus {
    fn post_boot_dmg() -> Self {
        Self {
            buffer: hw_constants::post_boot_hwio(),
            timers: Timers::post_boot_dmg(),
            ppu: Ppu::post_boot_dmg(),
            ..Default::default()
        }
    }
}
