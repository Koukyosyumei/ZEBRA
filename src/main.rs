use p3_mersenne_31::Mersenne31;
use p3_uni_stark::{get_symbolic_constraints, SymbolicExpression};

use latticevm::{p3_to_tv::convert_p3_expr, solver::solve, test_data::FibonacciAir};
use rand::{rngs::StdRng, SeedableRng};
use zkm_core_machine::CpuChip;
use zkm_stark::ZKM_PROOF_NUM_PV_ELTS;

fn main() -> Result<(), ()> {
    let prime = 2_u32.pow(31) - 2_u32.pow(24) + 1; //2_u32.pow(31) - 1;

    let mut rng = StdRng::seed_from_u64(42);

    let num_steps = 1; // Choose the number of Fibonacci steps
                       //let final_value = 21; // Choose the final Fibonacci value
                       /*
                       let air = FibonacciAir {
                           num_steps,
                           final_value,
                       };
                       */
    let air = CpuChip::default();
    let symbolic_constraints: Vec<SymbolicExpression<Mersenne31>> =
        get_symbolic_constraints(&air, 0, ZKM_PROOF_NUM_PV_ELTS);
    println!("#symbolic_constraints: {}", symbolic_constraints.len());

    let mut tv_constraints = vec![];
    for sc in symbolic_constraints {
        println!("{}", convert_p3_expr::<Mersenne31>(&sc));
        tv_constraints.push(convert_p3_expr::<Mersenne31>(&sc));
    }

    /*
        [1, 0, 0, 0, 0, 0, 4, 8, 0, 29, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0]
    CpuCols { shard: 1, clk_16bit_limb: 0, clk_8bit_limb: 0, shard_to_send: 0, clk_to_send: 0, pc: 0, next_pc: 4, next_next_pc: 8, instruction: InstructionCols { opcode: 0, op_a: 29, op_b: Word([0, 0, 0, 0]), op_c: Word([5, 0, 0, 0]), op_a_0: 0, imm_b: 0, imm_c: 1 }, num_extra_cycles: 0, is_memory: 0, is_rw_a: 0, is_write_hi: 0, is_halt: 0, is_sequential: 1, op_a_value: Word([5, 0, 0, 0]), hi_or_prev_a: Word([0, 0, 0, 0]), op_a_access: MemoryReadWriteCols { prev_value: Word([0, 0, 0, 0]), access: MemoryAccessCols { value: Word([5, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, op_b_access: MemoryReadCols { access: MemoryAccessCols { value: Word([0, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, op_c_access: MemoryReadCols { access: MemoryAccessCols { value: Word([5, 0, 0, 0]), prev_shard: 0, prev_clk: 0, compare_clk: 0, diff_16bit_limb: 0, diff_8bit_limb: 0 } }, is_real: 1, op_a_immutable: 0 }
         */

    let result = solve(
        &tv_constraints,
        ZKM_PROOF_NUM_PV_ELTS,
        num_steps,
        ZKM_PROOF_NUM_PV_ELTS,
        1,
        prime,
        &mut rng,
    );
    if let Some(trace) = result {
        println!("Find SAT assignment: {}", trace);
    } else {
        println!("Couln't Find SAT assignment");
    }

    Ok(())
}
