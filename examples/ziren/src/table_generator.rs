use std::io;

use p3_air::BaseAir;
use p3_field::PrimeField32;

use zkm_core_executor::events::MemoryAccessPosition;
use zkm_core_executor::events::MemoryReadRecord;
use zkm_core_executor::events::MemoryRecordEnum;
use zkm_core_executor::events::MemoryWriteRecord;
use zkm_core_executor::Register;
use zkm_core_executor::{
    ExecutionState, Executor, Instruction, MemoryAccessRecord, Opcode, Program,
};
use zkm_core_machine::mips::MipsAir;
use zkm_core_machine::utils::trace_checkpoint;
use zkm_core_machine::utils::ZKMCoreProverError;
use zkm_stark::koala_bear_poseidon2::KoalaBearPoseidon2;
use zkm_stark::ZKMCoreOpts;
use zkm_stark::{CpuProver, MachineProver};

pub fn u32_to_opcode(value: u32) -> Option<Opcode> {
    match value {
        0 => Some(Opcode::ADD),
        1 => Some(Opcode::SUB),
        2 => Some(Opcode::MULT),
        3 => Some(Opcode::MULTU),
        4 => Some(Opcode::MUL),
        5 => Some(Opcode::DIV),
        6 => Some(Opcode::DIVU),
        7 => Some(Opcode::SLL),
        8 => Some(Opcode::SRL),
        9 => Some(Opcode::SRA),
        10 => Some(Opcode::ROR),
        11 => Some(Opcode::SLT),
        12 => Some(Opcode::SLTU),
        13 => Some(Opcode::AND),
        14 => Some(Opcode::OR),
        15 => Some(Opcode::XOR),
        16 => Some(Opcode::NOR),
        17 => Some(Opcode::CLZ),
        18 => Some(Opcode::CLO),
        19 => Some(Opcode::BEQ),
        20 => Some(Opcode::BGEZ),
        21 => Some(Opcode::BGTZ),
        22 => Some(Opcode::BLEZ),
        23 => Some(Opcode::BLTZ),
        24 => Some(Opcode::BNE),
        25 => Some(Opcode::Jump),
        26 => Some(Opcode::Jumpi),
        27 => Some(Opcode::JumpDirect),
        28 => Some(Opcode::LB),
        29 => Some(Opcode::LBU),
        30 => Some(Opcode::LH),
        31 => Some(Opcode::LHU),
        32 => Some(Opcode::LW),
        33 => Some(Opcode::LWL),
        34 => Some(Opcode::LWR),
        35 => Some(Opcode::LL),
        36 => Some(Opcode::SB),
        37 => Some(Opcode::SH),
        38 => Some(Opcode::SW),
        39 => Some(Opcode::SWL),
        40 => Some(Opcode::SWR),
        41 => Some(Opcode::SC),
        42 => Some(Opcode::SYSCALL),
        43 => Some(Opcode::MEQ),
        44 => Some(Opcode::MNE),
        45 => Some(Opcode::TEQ),
        46 => Some(Opcode::SEXT),
        47 => Some(Opcode::WSBH),
        48 => Some(Opcode::EXT),
        49 => Some(Opcode::MADDU),
        50 => Some(Opcode::MSUBU),
        51 => Some(Opcode::INS),
        52 => Some(Opcode::MOD),
        53 => Some(Opcode::MODU),
        54 => Some(Opcode::MADD),
        55 => Some(Opcode::MSUB),
        0xff => Some(Opcode::UNIMPL),
        _ => None,
    }
}

pub fn emit_events(executor: &mut Executor, row: &Vec<u32>) {
    let opcode = u32_to_opcode(row[8]).unwrap();
    let op_a = row[9];
    let op_b = row[10] | (row[11] << 8) | (row[12] << 16) | (row[13] << 24);
    let op_c = row[14] | (row[15] << 8) | (row[16] << 16) | (row[17] << 24);
    let imm_b = row[19];
    let imm_c = row[20];
    let instruction = Instruction::new(
        opcode,
        op_a.try_into().unwrap(),
        op_b,
        op_c,
        imm_b == 1,
        imm_c == 1,
    );

    let clk = row[1] + row[2] * 2_u32.pow(16);
    let pc = row[5];
    let next_pc = row[6];
    let next_next_pc = row[7];

    let hi_or_prev_a = row[31] | (row[32] << 8) | (row[33] << 16) | (row[34] << 24);
    let a = row[39] | (row[40] << 8) | (row[41] << 16) | (row[42] << 24);
    let prev_value = row[35] | (row[36] << 8) | (row[37] << 16) | (row[38] << 24);
    let b = row[48] | (row[49] << 8) | (row[50] << 16) | (row[51] << 24);
    let c = row[57] | (row[58] << 8) | (row[59] << 16) | (row[60] << 24);

    let op_a_access = MemoryWriteRecord {
        value: a,
        shard: row[0],
        timestamp: clk,
        prev_value: prev_value,
        prev_shard: row[43],
        prev_timestamp: row[44],
    };

    let op_b_access = MemoryReadRecord {
        value: b,
        shard: row[0],
        timestamp: clk,
        prev_shard: row[52],
        prev_timestamp: row[53],
    };

    let op_c_access = MemoryReadRecord {
        value: c,
        shard: row[0],
        timestamp: clk,
        prev_shard: row[61],
        prev_timestamp: row[62],
    };

    match opcode {
        Opcode::SYSCALL => {
            executor.rw(Register::V0, a, MemoryAccessPosition::A);
        }
        Opcode::ADD
        | Opcode::SUB
        | Opcode::MULT
        | Opcode::MULTU
        | Opcode::MUL
        | Opcode::DIV
        | Opcode::DIVU
        | Opcode::SLL
        | Opcode::SRL
        | Opcode::SRA
        | Opcode::ROR
        | Opcode::SLT
        | Opcode::SLTU
        | Opcode::AND
        | Opcode::OR
        | Opcode::XOR
        | Opcode::NOR
        | Opcode::CLZ
        | Opcode::CLO
        | Opcode::MOD
        | Opcode::MODU => {
            if !instruction.imm_c {
                //let (rs1, rs2) = (
                //    (instruction.op_b as u8).into(),
                //    (instruction.op_c as u8).into(),
                //);
                let c = executor.mr_cpu(op_b, MemoryAccessPosition::C);
                let b = executor.mr_cpu(op_c, MemoryAccessPosition::B);
            } else if !instruction.imm_b && instruction.imm_c {
                let rs1 = (instruction.op_b as u8).into();
                executor.rr(rs1, MemoryAccessPosition::B);
            }

            let rd = instruction.op_a.into();
            let hi = if instruction.opcode.is_use_lo_hi_alu() {
                executor.rw(Register::LO, a, MemoryAccessPosition::A);
                executor.rw(Register::HI, hi_or_prev_a, MemoryAccessPosition::HI);
            } else {
                executor.rw(rd, a, MemoryAccessPosition::A);
            };
        }
        _ => {}
    }

    if instruction.is_alu_instruction() {
        executor.emit_alu_event(
            clk,
            instruction.opcode,
            Some(hi_or_prev_a),
            a,
            b,
            c,
            executor.memory_accesses.hi,
        );
    }

    for (_, event) in executor.local_memory_access.drain() {
        executor.record.cpu_local_memory_access.push(event);
    }
}
