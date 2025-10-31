use sm83_interp::cpu::Cpu;

pub fn read_ascii_from_tile_map(cpu: &Cpu) -> Vec<String> {
    let (lines, _) = cpu.memory.buffer[0x9800..0x9C00].as_chunks::<32>();
    lines
        .iter()
        .map(|line| str::from_utf8(line))
        .map(|result| result.unwrap().to_owned())
        .collect()
}

pub fn execute_until(cpu: &mut Cpu, final_pc: u16) {
    while cpu.registers.pc != final_pc {
        cpu.execute().unwrap();
        cpu.handle_interrupts();
    }
}
