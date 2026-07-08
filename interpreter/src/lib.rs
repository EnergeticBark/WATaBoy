#![feature(portable_simd)]
#![feature(iter_array_chunks)]

pub mod cpu;
pub mod joypad;
pub mod ppu;

pub mod addressable;
mod bus;
mod dma;
mod mbc;
pub mod timers;
