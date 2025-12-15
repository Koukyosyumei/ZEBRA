use zkm_core_executor::{Instruction, Opcode, Program};

pub fn get_program_str(program: &Program) -> String {
    program
        .instructions
        .iter()
        .map(|inst| format!("{:?}\n", inst))
        .collect::<String>()
}
