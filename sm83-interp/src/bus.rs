use crate::mbc::Mbc;
use crate::common::post_boot::PostBoot;
use crate::timers::Timers;

use hw_constants::io_regs;
use std::ops::{Index, Range};
use log::info;
use rkyv::{Archive, Deserialize, Serialize};
use crate::joypad::{ButtonsHeld, Joyp};

const MEM_MAP_SIZE: usize = 0x10000;

#[derive(Archive, Deserialize, Serialize)]
pub struct AddressBus {
    pub buffer: [u8; MEM_MAP_SIZE],
    timers: Timers,
    // Number of MCycles the PPU needs to run to catch up with the CPU.
    ppu_catchup: usize,
    mbc: Mbc,
}

impl AddressBus {
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.buffer[0..0x8000].copy_from_slice(&rom[0..0x8000]);
        self.mbc.load_rom(rom);
    }

    pub fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            // Delegate write in the ROM range and the SRAM range to the MBC.
            0x0000..0x8000 | 0xA000..0xC000 => {
                self.mbc.write_byte(&mut self.buffer, index, value);
            }
            
            // Initiate OAM transfer.
            0xFF46 => {
                // Actually write the value to this address before starting the OAM DMA transfer.
                self.buffer[index as usize] = value;

                // TODO: Accurately make this take a few cycles.
                info!(target: "oam_events", "DMA Transfer from 0x{value:X}00!");
                let oam_size = 0xA0;
                let src_start = u16::from_le_bytes([0x00, value]) as usize;
                let src_end = src_start + oam_size;
                let dest = hw_constants::OAM as usize;

                self.buffer.copy_within(src_start..src_end, dest);
            }

            // Certain I/O addresses only use certain bits. Bits which go unused are pulled high.
            // See Appendix B: https://gekkio.fi/files/gb-docs/gbctr.pdf
            io_regs::JOYP | io_regs::NR41 => self.buffer[index as usize] = value | 0b1100_0000,
            io_regs::SC => self.buffer[index as usize] = value | 0b0111_1110,
            io_regs::TAC => self.buffer[index as usize] = value | 0b1111_1000,
            io_regs::DIV => self.timers.system_clock = 0,
            io_regs::IF => self.buffer[index as usize] = value | 0b1110_0000,
            io_regs::STAT | io_regs::NR10 => self.buffer[index as usize] = value | 0b1000_0000,
            io_regs::NR30 => self.buffer[index as usize] = value | 0b0111_1111,
            io_regs::NR32 => self.buffer[index as usize] = value | 0b1001_1111,
            io_regs::NR44 => self.buffer[index as usize] = value | 0b0011_1111,
            io_regs::NR52 => self.buffer[index as usize] = value | 0b0111_0000,

            // There is *nothing* at these addresses, so they don't have names.
            // Their bits are always pulled high.
            0xFF03 | 0xFF08..0xFF0F | 0xFF15 | 0xFF1F | 0xFF27..0xFF30 | 0xFF4C..0xFF80 => {
                self.buffer[index as usize] = value | 0b1111_1111;
            }
            _ => self.buffer[index as usize] = value,
        }
    }

    pub fn increment_timers(&mut self, m_cycles: u16) {
        self.ppu_catchup += m_cycles as usize;

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

    pub fn update_joypad(&mut self, held_buttons: ButtonsHeld) {
        let mut joypad = Joyp::from_bits(self.buffer[io_regs::JOYP as usize]);
        if !joypad.select_buttons() {
            joypad.set_start_down(!held_buttons.start);
            joypad.set_select_up(!held_buttons.select);
            joypad.set_b_left(!held_buttons.b);
            joypad.set_a_right(!held_buttons.a);
        }
        if !joypad.select_dpad() {
            joypad.set_start_down(!held_buttons.down);
            joypad.set_select_up(!held_buttons.up);
            joypad.set_b_left(!held_buttons.left);
            joypad.set_a_right(!held_buttons.right);
        }
        // TODO: Fire the joypad interrupt on a high-to-low change
        self.buffer[io_regs::JOYP as usize] = joypad.into_bits();
    }
    
    // Get the number of MCycles the PPU needs to run for and reset the counter to 0.
    pub fn claim_ppu_cycles(&mut self) -> usize {
        let cycles = self.ppu_catchup;
        self.ppu_catchup = 0;
        cycles
    }
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            buffer: [0; MEM_MAP_SIZE],
            timers: Timers::default(),
            ppu_catchup: 0,
            mbc: Mbc::default(),
        }
    }
}

impl PostBoot for AddressBus {
    fn post_boot_dmg() -> Self {
        Self {
            buffer: {
                let mut buffer = [0; MEM_MAP_SIZE];
                buffer[0xA000..0xC000].fill(0xFF);
                buffer
            },
            // TODO: some memory values should be set. Try to pass the mooneye test.
            timers: Timers::post_boot_dmg(),
            ..Default::default()
        }
    }
}

impl Index<u16> for AddressBus {
    type Output = u8;

    fn index(&self, index: u16) -> &Self::Output {
        &self.buffer[index as usize]
    }
}

impl Index<Range<u16>> for AddressBus {
    type Output = [u8];

    fn index(&self, index: Range<u16>) -> &Self::Output {
        &self.buffer[index.start as usize..index.end as usize]
    }
}
