pub struct Memory {
    pub wram: Vec<u8>,
    pub ext_ram: Vec<u8>,
    pub backup_ram: Vec<u8>,
    pub program_rom: Vec<u8>,
}

pub fn new_memory(rom_data: &Vec<u8>) -> Memory {
    return Memory {
        wram: vec![0; 0x0800],
        ext_ram: vec![0; 0x1FE0],
        backup_ram: vec![0; 0x2000],
        program_rom: rom_data.clone(),
    };
}

pub fn read_mem(mem: &mut Memory, addr: u16) -> u8 {
    let mut value = 0u8;
    if addr < 0x0800 {
        value = mem.wram[addr as usize];
    } else if addr < 0x2000 {
        // unused
    } else if addr < 0x2008 || addr == 0x4014 {
        // ppu
    } else if addr < 0x4000 {
        // unused
    } else if addr < 0x4020 {
        // io
    } else if addr < 0x6000 {
        // ext ram
        value = mem.ext_ram[(addr - 0x4020) as usize];
    } else if addr < 0x8000 {
        // backup rom
        value = mem.backup_ram[(addr - 0x6000) as usize];
    } else if addr <= 0xC000 {
        // program rom
        value = mem.program_rom[(addr - 0x8000) as usize];
    } else {
        // program rom
        if mem.program_rom.len() == 0x4000 {
            value = mem.program_rom[(addr - 0xC000) as usize];
        } else {
            value = mem.program_rom[(addr - 0x8000) as usize];
        }
    }
    // println!("read {:04X?} value:{:02X}", addr, value);
    return value;
}

pub fn write_mem(mem: &mut Memory, addr: u16, value: u8) {
    // println!("write {:04X} value:{:02X}", addr, value);
    if addr < 0x0800 {
        mem.wram[addr as usize] = value;
    } else if addr < 0x2000 {
        // unused
    } else if addr < 0x2008 || addr == 0x4014 {
        // ppu
    } else if addr < 0x4000 {
        // unused
    } else if addr < 0x4020 {
    } else if addr < 0x6000 {
        mem.ext_ram[(addr - 0x4020) as usize] = value;
    } else if addr < 0x8000 {
        mem.backup_ram[(addr - 0x6000) as usize] = value;
    } else if addr < 0x8000 {
    } else {
        // program rom
        // read only
    }
}