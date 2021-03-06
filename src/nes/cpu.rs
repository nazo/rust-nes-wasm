use super::vmem;
use web_sys::console;

mod opcode;

pub struct Cpu {
    pub reg_a: u8,
    pub reg_x: u8,
    pub reg_y: u8,
    pub reg_s: u8,
    pub reg_p: u8,
    pub reg_pc: u16,
    pub cycle: i16,
}

pub fn new_cpu() -> Cpu {
    return Cpu {
        reg_a: 0,
        reg_x: 0,
        reg_y: 0,
        reg_s: 0xFD,
        reg_p: REG_P_FLAG_I | REG_P_FLAG_R,
        reg_pc: 0x8000,
        cycle: 0,
    };
}

const REG_P_MASK_N: u8 = 0x7F;
const REG_P_MASK_V: u8 = 0xBF;
const REG_P_MASK_R: u8 = 0xDF;
const REG_P_MASK_B: u8 = 0xEF;
const REG_P_MASK_D: u8 = 0xF7;
const REG_P_MASK_I: u8 = 0xFB;
const REG_P_MASK_Z: u8 = 0xFD;
const REG_P_MASK_C: u8 = 0xFE;

const REG_P_FLAG_N: u8 = 0x80;
const REG_P_FLAG_V: u8 = 0x40;
const REG_P_FLAG_R: u8 = 0x20;
const REG_P_FLAG_B: u8 = 0x10;
const REG_P_FLAG_D: u8 = 0x08;
const REG_P_FLAG_I: u8 = 0x04;
const REG_P_FLAG_Z: u8 = 0x02;
const REG_P_FLAG_C: u8 = 0x01;

pub fn reset(cpu: &mut Cpu, mem: &mut vmem::Vmem) {
    cpu.reg_pc = vmem::read_mem_word(mem, 0xFFFC);
    cpu.reg_p = cpu.reg_p | REG_P_FLAG_I;
}

fn fetch_pc_byte(cpu: &mut Cpu, mem: &mut vmem::Vmem) -> u8 {
    let data = vmem::read_mem(mem, cpu.reg_pc);
    cpu.reg_pc += 1;
    return data;
}

fn fetch_pc_word(cpu: &mut Cpu, mem: &mut vmem::Vmem) -> u16 {
    let data = vmem::read_mem_word(mem, cpu.reg_pc);
    cpu.reg_pc += 2;
    return data;
}

fn read_word(data: &[u8], p: u16) -> u16 {
    return ((data[(p + 1) as usize] as u16) << 8) | data[p as usize] as u16;
}

fn write_word(data: &mut Vec<u8>, p: u16, v: u16) {
    data[p as usize] = (v & 0xFF) as u8;
    data[(p + 1) as usize] = ((v & 0xFF00) >> 8) as u8;
}

fn stack_push_byte(cpu: &mut Cpu, mem: &mut vmem::Vmem, data: u8) {
    let stack_addr = 0x0100 | (cpu.reg_s as u16);
    // println!("stack push byte p={:04X} v={:04X}", stack_addr, data);
    vmem::write_mem(mem, stack_addr, data);
    cpu.reg_s = cpu.reg_s.wrapping_sub(1);
}

fn stack_pop_byte(cpu: &mut Cpu, mem: &mut vmem::Vmem) -> u8 {
    cpu.reg_s = cpu.reg_s.wrapping_add(1);

    let stack_addr = 0x0100 | (cpu.reg_s as u16);
    let data = vmem::read_mem(mem, 0x0100 | (cpu.reg_s as u16));

    // println!("stack pop byte p={:04X} v={:04X}", stack_addr, data);
    return data;
}

fn stack_push_word(cpu: &mut Cpu, mem: &mut vmem::Vmem, data: u16) {
    let stack_addr = 0x0100 | (cpu.reg_s as u16);
    // println!("stack push word p={:04X} v={:04X}", stack_addr, data);
    vmem::write_mem(mem, 0x0100 | (cpu.reg_s as u16), ((data & 0xFF00) >> 8) as u8);
    cpu.reg_s = cpu.reg_s.wrapping_sub(1);
    vmem::write_mem(mem, 0x0100 | (cpu.reg_s as u16), (data & 0xFF) as u8);
    cpu.reg_s = cpu.reg_s.wrapping_sub(1);
}

fn stack_pop_word(cpu: &mut Cpu, mem: &mut vmem::Vmem) -> u16 {
    let mut data: u16;
    cpu.reg_s = cpu.reg_s.wrapping_add(1);
    let stack_addr = 0x0100 | (cpu.reg_s as u16);
    data = vmem::read_mem(mem, 0x0100 | (cpu.reg_s as u16)) as u16;
    cpu.reg_s = cpu.reg_s.wrapping_add(1);
    data = data | ((vmem::read_mem(mem, 0x0100 | (cpu.reg_s as u16)) as u16) << 8);

    // println!("stack pop word p={:04X} v={:04X}", stack_addr, data);
    return data;
}

pub fn run(cpu: &mut Cpu, mem: &mut vmem::Vmem) {
    cpu.cycle = cpu.cycle - 1;
    if cpu.cycle > 0 {
        return;
    }
    cpu.reg_p = cpu.reg_p | REG_P_FLAG_R;

    // println!("pc: {:04X}", cpu.reg_pc);

    let pc = cpu.reg_pc;
    let code = fetch_pc_byte(cpu, mem);
    let op = &opcode::OPCODE_TABLE[code as usize];

    // println!("{:04X}  {}                       A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}", pc, opcode::OPCODE_DEBUG_SYMBOL[code as usize], cpu.reg_a, cpu.reg_x, cpu.reg_y, cpu.reg_p, cpu.reg_s);
    console::log_1(&format!("{:04X} {:02X} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}", pc, code, cpu.reg_a, cpu.reg_x, cpu.reg_y, cpu.reg_p, cpu.reg_s).into());

    // opcode::debug_opcode(code);
    exec_instructions(cpu, mem, op);
    cpu.cycle = op.cycles as i16;
}

fn read_by_addressing(cpu: &mut Cpu, mem: &mut vmem::Vmem, op: &opcode::Opcode) -> u16 {
    match op.addressing {
        opcode::ADDRESSING_IMPLIED => {
        }
        opcode::ADDRESSING_IMMEDIATE => {
            return fetch_pc_byte(cpu, mem) as u16;
        }
        opcode::ADDRESSING_ZEROPAGE => {
            return fetch_pc_byte(cpu, mem) as u16;
        }
        opcode::ADDRESSING_ZEROPAGE_X => {
            let data = fetch_pc_byte(cpu, mem);
            return data.wrapping_add(cpu.reg_x) as u16;
        }
        opcode::ADDRESSING_ZEROPAGE_Y => {
            let data = fetch_pc_byte(cpu, mem);
            return data.wrapping_add(cpu.reg_y) as u16;
        }
        opcode::ADDRESSING_ABSOLUTE => {
            return fetch_pc_word(cpu, mem);
        }
        opcode::ADDRESSING_ABSOLUTE_X => {
            let data = fetch_pc_word(cpu, mem);
            return data.wrapping_add(cpu.reg_x as u16);
        }
        opcode::ADDRESSING_ABSOLUTE_Y => {
            let data = fetch_pc_word(cpu, mem);
            return data.wrapping_add(cpu.reg_y as u16);
        }
        opcode::ADDRESSING_INDIRECT_X => {
            let fetch = fetch_pc_byte(cpu, mem);
            let mut addr = vmem::read_mem(mem, fetch.wrapping_add(cpu.reg_x) as u16) as u16;
            addr = addr | ((vmem::read_mem(mem, fetch.wrapping_add(cpu.reg_x).wrapping_add(1) as u16) as u16) << 8);
            return addr;
        }
        opcode::ADDRESSING_INDIRECT_Y => {
            let fetch = fetch_pc_byte(cpu, mem);
            let mut addr = (vmem::read_mem(mem, fetch.wrapping_add(1) as u16) as u16) << 8;
            addr = addr | vmem::read_mem(mem, fetch as u16) as u16;
            return addr.wrapping_add(cpu.reg_y as u16);
        }
        opcode::ADDRESSING_INDIRECT => {
            let mut fetch = fetch_pc_word(cpu, mem);
            let mut data = vmem::read_mem(mem, fetch) as u16;
            fetch = (fetch & 0xFF00) | (((fetch & 0xFF) as u8).wrapping_add(1) as u16);
            data = data | ((vmem::read_mem(mem, fetch) as u16) << 8);
            return data;
        }
        opcode::ADDRESSING_ACCUMULATOR => {
            return cpu.reg_a as u16;
        }
        opcode::ADDRESSING_RELATIVE => {
            return fetch_pc_byte(cpu, mem) as u16;
        }
        _ => {
        }
    }
    return 0;
}

fn exec_instructions(cpu: &mut Cpu, mem: &mut vmem::Vmem, op: &opcode::Opcode) {
    let mut data = read_by_addressing(cpu, mem, op);
    let relative = (data as u8) as i8;
    console::log_1(&format!("data {:04X}", data).into());
    match op.code {
        opcode::OPCODE_LDA => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_a = data as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_LDX => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_x = data as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_x & REG_P_FLAG_N) | (if cpu.reg_x == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_LDY => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_y = data as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_y & REG_P_FLAG_N) | (if cpu.reg_y == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_LAX => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_x = data as u8;
            cpu.reg_a = cpu.reg_x;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_STA => {
            vmem::write_mem(mem, data, cpu.reg_a);
        }
        opcode::OPCODE_STX => {
            vmem::write_mem(mem, data, cpu.reg_x);
        }
        opcode::OPCODE_STY => {
            vmem::write_mem(mem, data, cpu.reg_y);
        }
        opcode::OPCODE_TAX => {
            cpu.reg_x = cpu.reg_a;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_x & REG_P_FLAG_N) | (if cpu.reg_x == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_TAY => {
            cpu.reg_y = cpu.reg_a;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_y & REG_P_FLAG_N) | (if cpu.reg_y == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_TSX => {
            cpu.reg_x = cpu.reg_s;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_x & REG_P_FLAG_N) | (if cpu.reg_x == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_TXA => {
            cpu.reg_a = cpu.reg_x;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_TXS => {
            cpu.reg_s = cpu.reg_x;
            // println!("TXS reg_s = {:04X}", cpu.reg_x)
        }
        opcode::OPCODE_TYA => {
            cpu.reg_a = cpu.reg_y;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_ADC => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            let result = (cpu.reg_a as u16).wrapping_add(data).wrapping_add((cpu.reg_p & REG_P_FLAG_C) as u16);
            let reg_a = (result & 0xFF) as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_V & REG_P_MASK_Z & REG_P_MASK_C)) | (reg_a & REG_P_FLAG_N) | (if reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | (if result > 0xFF { REG_P_FLAG_C } else { 0 }) | (((!(cpu.reg_a ^ (data as u8)) & (cpu.reg_a ^ reg_a)) & REG_P_FLAG_N) >> 1);
            cpu.reg_a = reg_a;
        }
        opcode::OPCODE_SBC => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            let result = (cpu.reg_a as i16).wrapping_sub(data as i16).wrapping_sub(1 - (cpu.reg_p & REG_P_FLAG_C) as i16);
            let reg_a = (result & 0xFF) as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_V & REG_P_MASK_Z & REG_P_MASK_C)) | (reg_a & REG_P_FLAG_N) | (if reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | (if result < 0 { 0 } else { REG_P_FLAG_C }) | ((((cpu.reg_a ^ (data as u8)) & (cpu.reg_a ^ reg_a)) & REG_P_FLAG_N) >> 1);
            cpu.reg_a = reg_a;
        }
        opcode::OPCODE_AND => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_a = cpu.reg_a & (data as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_ASL => {
            let mut shift: u8;
            let remain: u8;
            if op.addressing != opcode::ADDRESSING_ACCUMULATOR {
                let addr = data as u16;
                shift = vmem::read_mem(mem, addr) as u8;
                remain = (shift & REG_P_FLAG_N) >> 7;
                shift = shift << 1;
                vmem::write_mem(mem, addr, shift);
            } else {
                shift = cpu.reg_a;
                remain = (shift & REG_P_FLAG_N) >> 7;
                shift = shift << 1;
                cpu.reg_a = shift;
            }
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (shift & REG_P_FLAG_N) | (if shift == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_BIT => {
            let test = vmem::read_mem(mem, data as u16) as u8;
            let zflag = (if (cpu.reg_a & test) == 0 { REG_P_FLAG_Z } else { 0 }) as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_V & REG_P_MASK_Z)) | (test & REG_P_FLAG_N) | zflag | (test & REG_P_FLAG_V);
        }
        opcode::OPCODE_CMP => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            let result = cpu.reg_a.wrapping_sub(data as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (result & REG_P_FLAG_N) | (if result == 0 { REG_P_FLAG_Z } else { 0 }) | (if cpu.reg_a < (data as u8) { 0 } else { 1 });
        }
        opcode::OPCODE_CPX => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            let result = cpu.reg_x.wrapping_sub(data as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (result & REG_P_FLAG_N) | (if result == 0 { REG_P_FLAG_Z } else { 0 }) | (if cpu.reg_x < (data as u8) { 0 } else { 1 });
        }
        opcode::OPCODE_CPY => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            let result = cpu.reg_y.wrapping_sub(data as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (result & REG_P_FLAG_N) | (if result == 0 { REG_P_FLAG_Z } else { 0 }) | (if cpu.reg_y < (data as u8) { 0 } else { 1 });
        }
        opcode::OPCODE_INC => {
            let mut value = vmem::read_mem(mem, data as u16) as u8;
            value = value.wrapping_add(1);
            vmem::write_mem(mem, data, value);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (value & REG_P_FLAG_N) | (if value == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_DEC => {
            let mut value = vmem::read_mem(mem, data as u16) as u8;
            value = value.wrapping_sub(1);
            vmem::write_mem(mem, data, value);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (value & REG_P_FLAG_N) | (if value == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_DEX => {
            cpu.reg_x = cpu.reg_x.wrapping_sub(1);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_x & REG_P_FLAG_N) | (if cpu.reg_x == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_DEY => {
            cpu.reg_y = cpu.reg_y.wrapping_sub(1);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_y & REG_P_FLAG_N) | (if cpu.reg_y == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_EOR => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_a = cpu.reg_a ^ (data as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_INX => {
            cpu.reg_x = cpu.reg_x.wrapping_add(1);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_x & REG_P_FLAG_N) | (if cpu.reg_x == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_INY => {
            cpu.reg_y = cpu.reg_y.wrapping_add(1);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_y & REG_P_FLAG_N) | (if cpu.reg_y == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_ISC => {
            let mut value = vmem::read_mem(mem, data as u16) as u8;
            value = value.wrapping_add(1);
            vmem::write_mem(mem, data, value);
            let result = (cpu.reg_a as i16).wrapping_sub(value as i16).wrapping_sub(1 - (cpu.reg_p & REG_P_FLAG_C) as i16);
            let reg_a = (result & 0xFF) as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_V & REG_P_MASK_Z & REG_P_MASK_C)) | (reg_a & REG_P_FLAG_N) | (if reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | (if result < 0 { 0 } else { REG_P_FLAG_C }) | ((((cpu.reg_a ^ (data as u8)) & (cpu.reg_a ^ reg_a)) & REG_P_FLAG_N) >> 1);
            cpu.reg_a = reg_a;
        }
        opcode::OPCODE_LSR => {
            let mut shift: u8;
            let remain: u8;
            if op.addressing != opcode::ADDRESSING_ACCUMULATOR {
                let addr = data as u16;
                shift = vmem::read_mem(mem, addr) as u8;
                remain = shift & 1;
                shift = shift >> 1;
                vmem::write_mem(mem, addr, shift);
            } else {
                shift = cpu.reg_a;
                remain = shift & 1;
                shift = shift >> 1;
                cpu.reg_a = shift;
            }
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (shift & REG_P_FLAG_N) | (if shift == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_ORA => {
            if op.addressing != opcode::ADDRESSING_IMMEDIATE {
                data = vmem::read_mem(mem, data as u16) as u16;
            }
            cpu.reg_a = cpu.reg_a | (data as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_ROL => {
            let mut shift: u8;
            let remain: u8;
            if op.addressing != opcode::ADDRESSING_ACCUMULATOR {
                let addr = data as u16;
                shift = vmem::read_mem(mem, addr) as u8;
                remain = (shift & 0x80) >> 7;
                shift = shift << 1 | (cpu.reg_p & REG_P_FLAG_C);
                vmem::write_mem(mem, addr, shift);
            } else {
                shift = cpu.reg_a;
                remain = (shift & 0x80) >> 7;
                shift = shift << 1 | (cpu.reg_p & REG_P_FLAG_C);
                cpu.reg_a = shift;
            }
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (shift & REG_P_FLAG_N) | (if shift == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_ROR => {
            let mut shift: u8;
            let remain: u8;
            if op.addressing != opcode::ADDRESSING_ACCUMULATOR {
                let addr = data as u16;
                shift = vmem::read_mem(mem, addr) as u8;
                remain = shift & 1;
                shift = shift >> 1 | ((cpu.reg_p & REG_P_FLAG_C) << 7);
                vmem::write_mem(mem, addr, shift);
            } else {
                shift = cpu.reg_a;
                remain = shift & 1;
                shift = shift >> 1 | ((cpu.reg_p & REG_P_FLAG_C) << 7);
                cpu.reg_a = shift;
            }
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (shift & REG_P_FLAG_N) | (if shift == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_PHA => {
            stack_push_byte(cpu, mem, cpu.reg_a);
        }
        opcode::OPCODE_PHP => {
            stack_push_byte(cpu, mem, cpu.reg_p | REG_P_FLAG_B);
        }
        opcode::OPCODE_PLA => {
            cpu.reg_a = stack_pop_byte(cpu, mem);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 });
        }
        opcode::OPCODE_PLP => {
            cpu.reg_p = (stack_pop_byte(cpu, mem) & REG_P_MASK_B) | (cpu.reg_p & REG_P_FLAG_B);
        }
        opcode::OPCODE_BEQ => {
            if (cpu.reg_p & REG_P_FLAG_Z) != 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BNE => {
            if (cpu.reg_p & REG_P_FLAG_Z) == 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BMI => {
            if (cpu.reg_p & REG_P_FLAG_N) != 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BPL => {
            if (cpu.reg_p & REG_P_FLAG_N) == 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BVS => {
            if (cpu.reg_p & REG_P_FLAG_V) != 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BVC => {
            if (cpu.reg_p & REG_P_FLAG_V) == 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BCS => {
            if (cpu.reg_p & REG_P_FLAG_C) != 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_BCC => {
            if (cpu.reg_p & REG_P_FLAG_C) == 0 {
                cpu.reg_pc = ((cpu.reg_pc as i32) + (relative as i32)) as u16;
            }
        }
        opcode::OPCODE_JMP => {
            cpu.reg_pc = data;
        }
        opcode::OPCODE_JSR => {
            let reg_pc = cpu.reg_pc;
            stack_push_word(cpu, mem, reg_pc - 1);
            cpu.reg_pc = data;
        }
        opcode::OPCODE_RTS => {
            let addr = stack_pop_word(cpu, mem) + 1;
            cpu.reg_pc = addr;
        }
        opcode::OPCODE_SEI => {
            cpu.reg_p = cpu.reg_p | REG_P_FLAG_I;
        }
        opcode::OPCODE_CLI => {
            cpu.reg_p = cpu.reg_p & REG_P_MASK_I;
        }
        opcode::OPCODE_CLD => {
            cpu.reg_p = cpu.reg_p & REG_P_MASK_D;
        }
        opcode::OPCODE_CLV => {
            cpu.reg_p = cpu.reg_p & REG_P_MASK_V;
        }
        opcode::OPCODE_SEC => {
            cpu.reg_p = cpu.reg_p | REG_P_FLAG_C;
        }
        opcode::OPCODE_SED => {
            cpu.reg_p = cpu.reg_p | REG_P_FLAG_D;
        }
        opcode::OPCODE_CLC => {
            cpu.reg_p = cpu.reg_p & REG_P_MASK_C;
        }
        opcode::OPCODE_BRK => {
            stack_push_word(cpu, mem, cpu.reg_pc);
            stack_push_byte(cpu, mem, cpu.reg_p);
            cpu.reg_pc = vmem::read_mem_word(mem, 0xFFFE);
            cpu.reg_p = cpu.reg_p | REG_P_FLAG_B;
        }
        opcode::OPCODE_RTI => {
            cpu.reg_p = stack_pop_byte(cpu, mem);
            cpu.reg_pc = stack_pop_word(cpu, mem);
        }
        opcode::OPCODE_SAX => {
            vmem::write_mem(mem, data, cpu.reg_a & cpu.reg_x);
        }
        opcode::OPCODE_DCP => {
            let mut value = vmem::read_mem(mem, data as u16) as u8;
            value = value.wrapping_sub(1);
            vmem::write_mem(mem, data, value);
            let result = cpu.reg_a.wrapping_sub(value as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (result & REG_P_FLAG_N) | (if result == 0 { REG_P_FLAG_Z } else { 0 }) | (if cpu.reg_a < (value as u8) { 0 } else { REG_P_FLAG_C });
        }
        opcode::OPCODE_SLO => {
            let addr = data as u16;
            data = vmem::read_mem(mem, addr) as u16;
            let original = data as u8;
            let remain = (original & REG_P_FLAG_N) >> 7;
            let shift = original << 1;
            vmem::write_mem(mem, addr, shift);
            cpu.reg_a = cpu.reg_a | shift;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_RLA => {
            let addr = data as u16;
            data = vmem::read_mem(mem, addr) as u16;
            let original = data as u8;
            let remain = (original & 0x80) >> 7;
            let shift = original << 1 | (cpu.reg_p & REG_P_FLAG_C);
            vmem::write_mem(mem, addr, shift);
            cpu.reg_a = cpu.reg_a & (shift as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_SRE => {
            let addr = data as u16;
            data = vmem::read_mem(mem, addr) as u16;
            let original = data as u8;
            let remain = original & 1;
            let shift = original >> 1;
            vmem::write_mem(mem, addr, shift);
            cpu.reg_a = cpu.reg_a ^ (shift as u8);
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_Z & REG_P_MASK_C)) | (cpu.reg_a & REG_P_FLAG_N) | (if cpu.reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | remain;
        }
        opcode::OPCODE_RRA => {
            let addr = data as u16;
            data = vmem::read_mem(mem, addr) as u16;
            let original = data as u8;
            let remain = original & 1;
            let shift = original >> 1 | ((cpu.reg_p & REG_P_FLAG_C) << 7);
            vmem::write_mem(mem, addr, shift);
            let result = (cpu.reg_a as u16).wrapping_add(shift as u16).wrapping_add(remain as u16);
            let reg_a = (result & 0xFF) as u8;
            cpu.reg_p = (cpu.reg_p & (REG_P_MASK_N & REG_P_MASK_V & REG_P_MASK_Z & REG_P_MASK_C)) | (reg_a & REG_P_FLAG_N) | (if reg_a == 0 { REG_P_FLAG_Z } else { 0 }) | (if result > 0xFF { REG_P_FLAG_C } else { 0 }) | (((!(cpu.reg_a ^ (shift as u8)) & (cpu.reg_a ^ reg_a)) & REG_P_FLAG_N) >> 1);
            cpu.reg_a = reg_a;
        }
        opcode::OPCODE_NOP => {
            // nop
        }
        opcode::OPCODE_KIL => {
            // nop
        }
        opcode::OPCODE_LAS => {
            // nop?
        }
        _ => {
            panic!("not implemented");
        }
    }
}