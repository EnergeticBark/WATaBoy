use crate::addressable::Addressable;
use crate::cpu::InterruptBits;
use crate::joypad::{ButtonsHeld, Joyp};
use crate::mbc::Mbc;
use crate::ppu::Ppu;
use crate::timers::Timers;

use hw_constants::io_regs::{
    BGP, DIV, IF, JOYP, LCDC, LY, LYC, NR10, NR30, NR32, NR41, NR44, NR52, OBP0, OBP1, SC, SCX,
    SCY, STAT, TAC, TIMA, TMA, WX, WY,
};
use hw_constants::{IE, MEM_MAP_SIZE, OAM_END, OAM_START, PostBoot, VRAM_END, VRAM_START};
use log::info;
use rkyv::{Archive, Deserialize, Serialize, with::Skip};

#[derive(Archive, Deserialize, Serialize)]
pub struct AddressBus {
    pub buffer: Box<[u8; MEM_MAP_SIZE]>,
    timers: Timers,
    #[rkyv(with = Skip)]
    pub ppu: Ppu,
    mbc: Mbc,
    half_ticked: bool,
    #[rkyv(with = Skip)]
    pub buttons_held: ButtonsHeld,
    pub clock: u64,
    pub next_interrupt: u64,
}

impl AddressBus {
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.buffer[0..0x8000].copy_from_slice(&rom[0..0x8000]);
        self.mbc.load_rom(rom);
    }

    pub fn read_byte(&mut self, index: u16) -> u8 {
        match index {
            // Delegate reads to the PPU.
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
            | WX => {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.read_byte(index)
            }

            // Delegate reads to the timers
            DIV | TIMA => self.timers.read_byte(index),

            // TODO: Delegate MBC bank switches.
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
            VRAM_START..VRAM_END | OAM_START..OAM_END => {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.write_byte(index, value, self.clock);
                self.ppu_est_next_intr();
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

                self.ppu
                    .oam
                    .copy_from_slice(&self.buffer[src_start..src_end]);
            }

            // Certain I/O addresses only use certain bits. Bits which go unused are pulled high.
            // See Appendix B: https://gekkio.fi/files/gb-docs/gbctr.pdf
            JOYP => {
                // Lower nibble is read-only, only set select bits.
                // See: https://gbdev.io/pandocs/Joypad_Input.html#ff00--p1joyp-joypad
                let written = Joyp::from_bits(value);
                self.buffer[index as usize] = Joyp::from_bits(self.buffer[index as usize])
                    .with_select_buttons(written.select_buttons())
                    .with_select_dpad(written.select_dpad())
                    .into_bits();

                self.update_joypad();
            }
            SC => self.buffer[index as usize] = value | 0b0111_1110,
            TAC => self.buffer[index as usize] = value | 0b1111_1000,

            // Delegate writes to the timers.
            DIV | TIMA => self.timers.write_byte(index, value, self.clock),

            IF => self.buffer[index as usize] = value | 0b1110_0000,

            NR10 => self.buffer[index as usize] = value | 0b1000_0000,
            NR30 => self.buffer[index as usize] = value | 0b0111_1111,
            NR32 => self.buffer[index as usize] = value | 0b1001_1111,
            NR41 => self.buffer[index as usize] = value | 0b1100_0000,
            NR44 => self.buffer[index as usize] = value | 0b0011_1111,
            NR52 => self.buffer[index as usize] = value | 0b0111_0000,

            // Delegate PPU registers to the PPU.
            LY => (),
            // Still needed until I can update interrupts without passing in all memory :(.
            STAT | LYC => {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.write_byte(index, value, self.clock);

                if !self.ppu.is_disabled() {
                    self.ppu
                        .update_stat_interrupt(&mut self.buffer[IF as usize]);
                }

                self.ppu_est_next_intr();
            }
            LCDC | SCY | SCX | BGP | OBP0 | OBP1 | WY | WX => {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.write_byte(index, value, self.clock);
                self.ppu_est_next_intr();
            }

            // There is *nothing* at these addresses, so they don't have names.
            // Their bits are always pulled high.
            0xFF03 | 0xFF08..0xFF0F | 0xFF15 | 0xFF1F | 0xFF27..0xFF30 | 0xFF4C..0xFF80 => {
                self.buffer[index as usize] = value | 0b1111_1111;
            }
            IE => {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.buffer[index as usize] = value;
                self.ppu_est_next_intr();
            }
            _ => self.buffer[index as usize] = value,
        }
    }

    fn ppu_est_next_intr(&mut self) {
        self.next_interrupt = self
            .ppu
            .predict_next_interrupt(self.clock, InterruptBits::from(self.buffer[IE as usize]));
    }

    pub fn half_increment_timers(&mut self) {
        self.clock += 2;
        if self.next_interrupt <= self.clock {
            self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
            self.ppu_est_next_intr();
        }

        if !self.half_ticked {
            self.half_ticked = true;
            return;
        }
        self.half_ticked = false;

        self.timers.update_timer_modulo(self.buffer[TMA as usize]);
        self.timers.update_timer_control(self.buffer[TAC as usize]);

        self.timers.increment(1);

        if self.timers.process_interrupt() {
            self.buffer[IF as usize] |= 0b0000_0100;
        }
    }

    pub fn increment_timers(&mut self, m_cycles: u16) {
        self.clock += u64::from(m_cycles) * 4;
        if self.next_interrupt <= self.clock {
            self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
            self.ppu_est_next_intr();
        }

        self.timers.update_timer_modulo(self.buffer[TMA as usize]);
        self.timers.update_timer_control(self.buffer[TAC as usize]);

        self.timers.increment(m_cycles);

        if self.timers.process_interrupt() {
            self.buffer[IF as usize] |= 0b0000_0100;
        }
    }

    pub(crate) fn update_joypad(&mut self) {
        let mut joypad = Joyp::from_bits(self.buffer[JOYP as usize]);
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
        self.buffer[JOYP as usize] = joypad.into_bits();
    }
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            buffer: vec![0; MEM_MAP_SIZE].into_boxed_slice().try_into().unwrap(),
            timers: Timers::default(),
            ppu: Ppu::default(),
            mbc: Mbc::default(),
            half_ticked: false,
            buttons_held: ButtonsHeld::default(),
            clock: 0,
            next_interrupt: 0,
        }
    }
}

impl PostBoot for AddressBus {
    fn post_boot_mgb() -> Self {
        Self {
            buffer: hw_constants::post_boot_hwio(),
            timers: Timers::post_boot_mgb(),
            ppu: Ppu::post_boot_mgb(),
            // TODO: Might be worth running the boot rom to calculate clock and next_interrupt...
            clock: 391, // Needed for the PPU to catch up.
            ..Default::default()
        }
    }
}
