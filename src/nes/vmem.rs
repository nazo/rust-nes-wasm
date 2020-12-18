use super::memory;
use super::ppu;
use web_sys::console;

pub struct Vmem<'a, 'b> {
    pub mem: &'a mut memory::Memory,
    pub ppu: &'b mut ppu::Ppu,
}

pub fn new_vmem<'a, 'b>(mem: &'a mut memory::Memory, ppu: &'b mut ppu::Ppu) -> Vmem<'a, 'b> {
    return Vmem {
        mem: mem,
        ppu: ppu,
    };
}

pub fn read_mem_word(vmem: &mut Vmem, addr: u16) -> u16 {
    let data1 = read_mem(vmem, addr) as u16;
    let data2 = read_mem(vmem, addr + 1) as u16;
    return (data2 << 8) | data1;
}

pub fn write_mem_word(vmem: &mut Vmem, addr: u16, data: u16) {
    write_mem(vmem, addr, (data & 0xFF) as u8);
    write_mem(vmem, addr + 1, ((data & 0xFF00) >> 8) as u8);
}

pub fn read_mem(mem: &mut Vmem, addr: u16) -> u8 {
    let mut value = 0u8;
    if (addr >= 0x2000 && addr < 0x2008) || addr == 0x4014 {
        // ppu
        value = ppu::read_io(&mut mem.ppu, addr);
    } else {
        // cpu
        value = memory::read_mem(&mut mem.mem, addr);
    }
    // console::log_1(&format!("read {:04X?} value:{:02X}", addr, value).into());
    return value;
}

pub fn write_mem(mem: &mut Vmem, addr: u16, value: u8) {
    // console::log_1(&format!("write {:04X} value:{:02X}", addr, value).into());
    if (addr >= 0x2000 && addr < 0x2008) || addr == 0x4014 {
        // ppu
        ppu::write_io(&mut mem.ppu, addr, value);
    } else {
        // cpu
        memory::write_mem(&mut mem.mem, addr, value);
    }
}