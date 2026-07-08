mod waking_counters;
#[cfg(feature = "waking-counters")]
use waking_counters::WakingCounter;

use std::hint::cold_path;

use crate::addressable::Addressable;
use crate::cpu::InterruptBits;
use crate::dma::Dma;
use crate::joypad::{ButtonsHeld, Joyp};
use crate::mbc::MbcDispatcher;
use crate::ppu::Ppu;
use crate::timers::Timers;

use hw_constants::io_regs::{
    BANK, BGP, DIV, IF, JOYP, LCDC, LY, LYC, NR10, NR30, NR32, NR41, NR44, NR52, OBP0, OBP1, SC,
    SCX, SCY, STAT, TAC, TIMA, TMA, WX, WY,
};
use hw_constants::{
    DMA, ECHO_END, ECHO_START, IE, MEM_MAP_SIZE, OAM_END, OAM_START, PostBoot, VRAM_END, VRAM_START,
};
use log::info;
use rkyv::{Archive, Deserialize, Serialize, with::Skip};

#[derive(Archive, Deserialize, Serialize)]
pub struct AddressBus {
    pub buffer: Box<[u8; MEM_MAP_SIZE]>,
    pub timers: Timers,
    #[rkyv(with = Skip)]
    pub ppu: Ppu,
    #[rkyv(with = Skip)]
    pub mbc: MbcDispatcher,
    #[rkyv(with = Skip)]
    pub buttons_held: ButtonsHeld,
    pub clock: u64,
    pub next_interrupt: u64,
    #[rkyv(with = Skip)]
    pub dma: Dma,
    #[cfg(feature = "waking-counters")]
    pub waking_reads: WakingCounter,
    #[cfg(feature = "waking-counters")]
    pub waking_writes: WakingCounter,
}

/// See: <https://gbdev.io/pandocs/Memory_Map.html#echo-ram>
fn echo_to_wram(index: u16) -> u16 {
    let lower_13 = index & 0b0001_1111_1111_1111;
    0xC000 + lower_13
}

impl AddressBus {
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mbc = MbcDispatcher::from_rom(rom);
    }

    pub fn load_sram(&mut self, sram: &[u8]) {
        self.mbc.load_sram(sram);
    }

    pub fn dump_sram(&self) -> &[u8] {
        self.mbc.dump_sram()
    }

    fn read_special(&mut self, index: u16) -> u8 {
        match index {
            // Delegate reads to the PPU.
            VRAM_START..VRAM_END | LCDC | STAT | SCY | SCX | LYC | BGP | OBP0 | OBP1 | WY | WX => {
                #[cfg(feature = "waking-counters")]
                self.waking_reads.log_access(index);

                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.read_byte(index, self.clock)
            }

            OAM_START..OAM_END => {
                // Block OAM reads during DMA.
                if self.dma.is_running() {
                    return 0xFF;
                }

                #[cfg(feature = "waking-counters")]
                self.waking_reads.log_access(index);

                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.read_byte(index, self.clock)
            }

            LY => self.ppu.read_byte(index, self.clock),

            // Delegate reads to the timers
            DIV | TIMA | TMA | TAC => {
                #[cfg(feature = "waking-counters")]
                self.waking_reads.log_access(index);

                self.timers
                    .catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.timers.read_byte(index, self.clock)
            }

            IF => {
                #[cfg(feature = "waking-counters")]
                self.waking_reads.log_access(index);

                // TODO: I should probably catch up the PPU here too...
                // 3/25/25 update: Actually, I could *selectively* update components here based on which bits will actually change. >:3
                self.timers
                    .catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.buffer[index as usize]
            }

            ECHO_START..ECHO_END => {
                let wram_index = echo_to_wram(index) as usize;
                self.buffer[wram_index]
            }
            _ => self.buffer[index as usize],
        }
    }

    // This is incredibly hacky, but it prevents any stack frames from being created when index < VRAM_START.
    // Maybe see if there's a better way to do this? Keywords: "fast-mem" maybe?
    #[inline(never)]
    pub fn read_byte(&mut self, index: u16) -> u8 {
        match index {
            // Delegate reads to the MBC.
            ..VRAM_START | 0xA000..0xC000 => self.mbc.read_byte(index),
            // Delegate reads to the PPU/timers.
            _ => {
                cold_path();
                self.read_special(index)
            }
        }
    }

    pub fn write_byte(&mut self, index: u16, value: u8) {
        match index {
            // Delegate write in the ROM range and the SRAM range to the MBC.
            0x0000..0x8000 | 0xA000..0xC000 | BANK => {
                self.mbc.write_byte(index, value);
            }

            // Delegate writes to VRAM to the PPU.
            VRAM_START..VRAM_END => {
                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.write_byte(index, value, self.clock);
                self.ppu_est_next_intr();
            }

            // Delegate writes to VRAM to OAM the PPU.
            OAM_START..OAM_END => {
                // Block OAM writes during DMA.
                if self.dma.is_running() {
                    return;
                }

                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.write_byte(index, value, self.clock);
                self.ppu_est_next_intr();
            }

            ECHO_START..ECHO_END => {
                let wram_index = echo_to_wram(index) as usize;
                self.buffer[wram_index] = value;
            }

            // Initiate OAM transfer.
            DMA => {
                // Actually write the value to this address before starting the OAM DMA transfer.
                self.buffer[DMA as usize] = value;
                self.dma.start();
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

            // Delegate writes to the timers.
            DIV | TIMA | TMA | TAC => {
                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

                self.timers
                    .catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.timers.write_byte(index, value, self.clock);
                self.timers_est_next_intr();
            }

            IF => {
                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

                // If we don't catch up the components now, the value we're writing may get overwritten by a stale flag when we catch up later.
                // TODO: I should probably catch up the timers here too...
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.buffer[index as usize] = value | 0b1110_0000;
                self.ppu_est_next_intr();
            }

            NR10 => self.buffer[index as usize] = value | 0b1000_0000,
            NR30 => self.buffer[index as usize] = value | 0b0111_1111,
            NR32 => self.buffer[index as usize] = value | 0b1001_1111,
            NR41 => self.buffer[index as usize] = value | 0b1100_0000,
            NR44 => self.buffer[index as usize] = value | 0b0011_1111,
            NR52 => self.buffer[index as usize] = value | 0b0111_0000,

            // Delegate PPU registers to the PPU.
            LY => (),
            // Still needed until I can update interrupts without passing in all memory :(.

            // LCDC needs to update the STAT interrupt because enabling the PPU restarts the LY=LYC comparison check.
            // See: https://github.com/Gekkio/mooneye-test-suite/blob/443f6e1f2a8d83ad9da051cbb960311c5aaaea66/acceptance/ppu/stat_lyc_onoff.s#L169

            // TODO: HEY!!! Optimization idea: if we're writing a value that's identical to the current value,
            // we don't actually need to catch up the component, because nothing has changed.
            // Pokemon Blue updates the value of SCX, SCY, and WY on the title screen what seems several times per frame.
            LCDC | STAT | LYC => {
                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu.write_byte(index, value, self.clock);

                if !self.ppu.is_disabled() {
                    self.ppu
                        .update_stat_interrupt(&mut self.buffer[IF as usize]);
                }

                self.ppu_est_next_intr();
            }
            SCY | SCX | BGP | OBP0 | OBP1 | WY | WX => {
                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

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
                #[cfg(feature = "waking-counters")]
                self.waking_writes.log_access(index);

                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.timers
                    .catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.buffer[index as usize] = value;
                self.ppu_est_next_intr();
                self.timers_est_next_intr();
            }
            _ => self.buffer[index as usize] = value,
        }
    }

    fn ppu_est_next_intr(&mut self) {
        self.ppu
            .predict_next_interrupt(InterruptBits::from(self.buffer[IE as usize]));
        let next_ppu_interrupt =
            u64::min(self.ppu.next_vblank_interrupt, self.ppu.next_lcd_interrupt);
        self.next_interrupt = u64::min(self.timers.next_interrupt, next_ppu_interrupt);
    }

    fn timers_est_next_intr(&mut self) {
        self.timers
            .predict_next_interrupt(InterruptBits::from(self.buffer[IE as usize]));
        let next_ppu_interrupt =
            u64::min(self.ppu.next_vblank_interrupt, self.ppu.next_lcd_interrupt);
        self.next_interrupt = u64::min(self.timers.next_interrupt, next_ppu_interrupt);
    }

    fn oam_dma(&mut self) {
        // TODO: Maybe actually only move one byte per M-Cycle if it affects the PPU.
        // TODO: If so, also move this logic into the PPU.

        // Disable DMA running state so we don't get into a bus conflict with an existing DMA transfer.
        self.dma.running = false;

        let value = self.buffer[DMA as usize];
        let upper_byte = if value > 0xE0 { value - 0x20 } else { value };

        info!(target: "oam_events", "DMA Transfer from 0x{value:X}00!");
        let oam_size = 0xA0;
        let src_start = u16::from_le_bytes([0x00, upper_byte]);

        // TODO: use copy_from_slice from the correct region of memory, only use read_byte as a fallback.
        for i in 0..oam_size {
            self.ppu.oam[i as usize] = self.read_byte(src_start + i);
        }

        // Restore DMA running state.
        self.dma.running = true;
    }

    pub fn half_increment_timers(&mut self) {
        self.clock += 2;

        if self.dma.catch_up(self.clock) {
            self.oam_dma();
        }

        if self.next_interrupt <= self.clock {
            if self
                .ppu
                .next_vblank_interrupt
                .min(self.ppu.next_lcd_interrupt)
                <= self.clock
            {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu_est_next_intr();
            }

            if self.timers.next_interrupt <= self.clock {
                self.timers
                    .catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.timers_est_next_intr();
            }
        }
    }

    pub fn increment_timers(&mut self, m_cycles: u16) {
        self.clock += u64::from(m_cycles) * 4;

        if self.dma.catch_up(self.clock) {
            self.oam_dma();
        }

        if self.next_interrupt <= self.clock {
            if self
                .ppu
                .next_vblank_interrupt
                .min(self.ppu.next_lcd_interrupt)
                <= self.clock
            {
                self.ppu.catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.ppu_est_next_intr();
            }

            if self.timers.next_interrupt <= self.clock {
                self.timers
                    .catch_up(self.clock, &mut self.buffer[IF as usize]);
                self.timers_est_next_intr();
            }
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

    pub fn boot_rom_mounted(&self) -> bool {
        self.mbc.boot_rom_mounted
    }
}

impl Default for AddressBus {
    fn default() -> Self {
        Self {
            // TODO: Ughhh, make this all zeros again after I delegate SRAM reads to MBC. Has to be like this for Blargg's.
            buffer: hw_constants::post_boot_hwio(),
            //buffer: vec![0; MEM_MAP_SIZE].into_boxed_slice().try_into().unwrap(),
            timers: Timers::default(),
            ppu: Ppu::default(),
            mbc: MbcDispatcher::default(),
            buttons_held: ButtonsHeld::default(),
            clock: 0,
            next_interrupt: 0,
            dma: Dma::default(),
            #[cfg(feature = "waking-counters")]
            waking_reads: WakingCounter::default(),
            #[cfg(feature = "waking-counters")]
            waking_writes: WakingCounter::default(),
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
