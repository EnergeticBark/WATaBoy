pub trait Addressable {
    fn read_byte(&self, index: u16, cpu_clock: u64) -> u8;
    fn write_byte(&mut self, index: u16, value: u8, cpu_clock: u64);
}
