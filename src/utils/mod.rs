// helper utility functions and macros
use c64::cpu;
use c64::opcodes;
use std::io::prelude::*;
use std::fs::File;
use std::io::SeekFrom;
use std::path::Path;


// helper macros to easily extract references from Option<RefCell<...>>
macro_rules! as_ref {
    ($x:expr) => ($x.as_ref().unwrap().borrow_mut())
}

macro_rules! as_mut {
    ($x:expr) => ($x.as_mut().unwrap().borrow_mut())
}


// common helper functions
pub fn open_file(filename: &str, offset: u64) -> Vec<u8> {
    let path = Path::new(&filename);
    
    let mut file = match File::open(&path) {
        Err(why) => panic!("Couldn't open {}: {}", path.display(), why.to_string()),
        Ok(file) => file,
    };

    let mut file_data = Vec::<u8>::new();

    let _ = file.seek(SeekFrom::Start(offset));
    let result = file.read_to_end(&mut file_data);
    
    match result {
        Err(why)   => panic!("Error reading file: {}", why.to_string()),
        Ok(result) => println!("Read {}: {} bytes", path.display(), result),
    };    

    file_data
}


// set 8 consecutive buffer elements to single value for faster update of
// a single 8-pixel screen chunk
pub fn memset8(buffer: &mut [u32], start: usize, value: u32) {
    buffer[start]   = value;
    buffer[start+1] = buffer[start];
    buffer[start+2] = buffer[start];
    buffer[start+3] = buffer[start];
    buffer[start+4] = buffer[start];
    buffer[start+5] = buffer[start];
    buffer[start+6] = buffer[start];
    buffer[start+7] = buffer[start];
}


pub fn fetch_c64_color_rgba(idx: u8) -> u32 {
    // palette RGB values copied from WinVICE
    match idx & 0x0F {
        0x00  => 0x00000000,
        0x01  => 0x00FFFFFF,
        0x02  => 0x00894036,
        0x03  => 0x007ABFC7,
        0x04  => 0x008A46AE,
        0x05  => 0x0068A941,
        0x06  => 0x003E31A2,
        0x07  => 0x00D0DC71,
        0x08  => 0x00905F25,
        0x09  => 0x005C4700,
        0x0A  => 0x00BB776D,
        0x0B  => 0x00555555,
        0x0C  => 0x00808080,
        0x0D  => 0x00ACEA88,
        0x0E  => 0x007C70DA,
        0x0F  => 0x00ABABAB,
        _ => panic!("Unknown color!"),
    }
}


// instruction debugging
pub struct OpDebugger {
    pub jump_queue: Vec<u8>
}

impl OpDebugger {
    pub fn new() -> OpDebugger {
        OpDebugger {
            jump_queue: Vec::<u8>::new()
        }
    }
}


// output current instruction and CPU register status in a neat, readable fashion
pub fn debug_instruction(opcode: u8, cpu: &mut cpu::CPU) {
    cpu.prev_pc = cpu.pc;
    let prev_pc = cpu.prev_pc;
    
    let operand_hex: String;
    let operand: String;
    let mut extra_cycle = false;
    let debug_loops = true; // if true, every loop will be unrolled in the debug output

    // RTS? pop from queue to continue logging
    if !debug_loops {
        match cpu.instruction.opcode
        {
            opcodes::Op::RTS => {
                let _ = cpu.op_debugger.jump_queue.pop();
                return;
            },
            opcodes::Op::JSR => {
                if !cpu.op_debugger.jump_queue.is_empty() {
                    cpu.op_debugger.jump_queue.push(opcode);
                    return;
                }
            },
            _ => {
                if !cpu.op_debugger.jump_queue.is_empty() {
                    return;
                }
            }
        }
    }

    // instruction opcode and arglist formatting based on addressing mode
    match cpu.instruction.addr_mode {
        opcodes::AddrMode::Implied => {
            operand_hex = format!("       ");
            operand = format!("       ");
        },
        opcodes::AddrMode::Accumulator => {
            operand_hex = format!("       ");
            operand = format!("A      ");
        },
        opcodes::AddrMode::Immediate => {
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            operand = format!("#${:02X}   ", cpu.read_byte(prev_pc)); 
        },
        opcodes::AddrMode::Absolute => {
            operand_hex = format!(" {:02X} {:02X} ", cpu.read_byte(prev_pc), cpu.read_byte(prev_pc + 0x01));
            operand = format!("${:04X}  ", cpu.read_word_le(cpu.prev_pc));
        },
        opcodes::AddrMode::AbsoluteIndexedX(ec) => {
            extra_cycle = ec;
            operand_hex = format!(" {:02X} {:02X} ", cpu.read_byte(prev_pc), cpu.read_byte(prev_pc + 0x01));
            operand = format!("${:04X},X", cpu.read_word_le(cpu.prev_pc));
        },
        opcodes::AddrMode::AbsoluteIndexedY(ec) => {
            extra_cycle = ec;
            operand_hex = format!(" {:02X} {:02X} ", cpu.read_byte(prev_pc), cpu.read_byte(prev_pc + 0x01));
            operand = format!("${:04X},Y", cpu.read_word_le(cpu.prev_pc));
        },
        opcodes::AddrMode::Zeropage => {
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            operand = format!("${:02X}    ", cpu.read_byte(prev_pc));
        }, 
        opcodes::AddrMode::ZeropageIndexedX => {
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            operand = format!("${:02X},X  ", cpu.read_byte(prev_pc));
        },
        opcodes::AddrMode::ZeropageIndexedY => {
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            operand = format!("${:02X},Y  ", cpu.read_byte(prev_pc));
        },
        opcodes::AddrMode::Relative => {
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            let b: i8 = cpu.read_byte(prev_pc) as i8;
            operand = format!("${:04X}  ", ((cpu.prev_pc + 1) as i16 + b as i16) as u16);
        },
        opcodes::AddrMode::Indirect => {
            operand_hex = format!(" {:02X} {:02X} ", cpu.read_byte(prev_pc), cpu.read_byte(prev_pc + 0x01));
            operand = format!("(${:04X})", cpu.read_word_le(cpu.prev_pc));
        },
        opcodes::AddrMode::IndexedIndirectX => {
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            operand = format!("(${:02X},X)", cpu.read_byte(prev_pc));
        },
        opcodes::AddrMode::IndirectIndexedY(ec) => {
            extra_cycle = ec;
            operand_hex = format!(" {:02X}    ", cpu.read_byte(prev_pc));
            operand = format!("(${:02X}),Y", cpu.read_byte(prev_pc));
        },
    }

    // control latch bytes' status
    let byte0 = cpu.read_byte(0x0000);
    let byte1 = cpu.read_byte(0x0001);

    let mut total_cycles = cpu.instruction.cycles_to_fetch + cpu.instruction.cycles_to_run + cpu.instruction.cycles_to_rmw;
    let mut fetch_cycles = cpu.instruction.cycles_to_fetch;
    let mut extra_cycle_mark = "*";

    if !extra_cycle {
        extra_cycle_mark = " ";
        total_cycles += 1;
        fetch_cycles += 1;
    }
    
    let rmw_mark = if cpu.instruction.cycles_to_rmw > 0 { "+" } else { " " };

    println!("${:04X}: {:02X}{}{}{} {}  {}<- A: {:02X} X: {:02X} Y: {:02X} SP: {:02X} 00: {:02X} 01: {:02X} NV-BDIZC: [{:08b}] ({} cls, f: {}, r: {})", cpu.prev_pc - 1, opcode, operand_hex, extra_cycle_mark, cpu.instruction, operand,rmw_mark, cpu.a, cpu.x, cpu.y, cpu.sp, byte0, byte1, cpu.p, total_cycles, fetch_cycles, cpu.instruction.cycles_to_run);

    // JSR? push on queue to supress logging
    if !debug_loops {
        match cpu.instruction.opcode {
            opcodes::Op::JSR => cpu.op_debugger.jump_queue.push(opcode),
            _ => ()
        }
    }
}

