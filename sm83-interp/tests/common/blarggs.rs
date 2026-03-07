use sm83_interp::addressable::Addressable;
use sm83_interp::cpu::Cpu;

pub struct BlarggTest {
    pub rom: &'static [u8],
    /* Blargg's test roms will put themselves in an infinite loop after passing or failing.
      We stop and run our assertions once the program counter reaches this looping instruction.
      [We probably want to implement a timeout at some point.]
    */
    pub final_pc: u16,
}

fn read_ascii_from_tile_map(cpu: &Cpu) -> Vec<String> {
    let lines_buffer: Vec<u8> = (0x9800..0x9C00)
        .map(|i| cpu.memory.ppu.read_byte(i))
        .collect();
    let (lines, _) = lines_buffer.as_chunks::<32>();
    lines
        .iter()
        .map(|line| str::from_utf8(line))
        .map(|result| result.unwrap().to_owned())
        .collect()
}

fn execute_until(cpu: &mut Cpu, final_pc: u16) {
    while cpu.registers.pc != final_pc {
        cpu.execute().unwrap();
    }
}

pub fn run_blargg_test(cpu: &mut Cpu, test: &BlarggTest) -> Vec<String> {
    cpu.memory.load_rom(test.rom);
    execute_until(cpu, test.final_pc);

    read_ascii_from_tile_map(cpu)
}
