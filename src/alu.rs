use crate::interval::{AbstractInterval, MayBeFlag};
use crate::symbolic::{LatticeVMSymbolicEntry, LatticeVMSymbolicExpr};

pub enum OpALU {
    Add,
    Sub,
    Mul,
    And,
    Or,
    Xor,
    Lt,
}

pub fn reconstruct_symbolic_word(
    row: &[LatticeVMSymbolicExpr],
    base: usize,
) -> LatticeVMSymbolicExpr {
    let mut val = LatticeVMSymbolicExpr::Constant(AbstractInterval::zero());
    let mut mul = 1_i64;
    for i in 0..4 {
        let rm = LatticeVMSymbolicExpr::Mul(
            Box::new(row[base + i].clone()),
            Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                mul,
            ))),
        );
        val = LatticeVMSymbolicExpr::Add(Box::new(val.clone()), Box::new(rm));
        mul *= 256;
    }
    val
}

pub fn get_alu_constraint(
    a: &[LatticeVMSymbolicExpr],
    b: &[LatticeVMSymbolicExpr],
    c: &[LatticeVMSymbolicExpr],
    op: &OpALU,
) -> LatticeVMSymbolicExpr {
    let a_word = reconstruct_symbolic_word(a, 0);
    let b_word = reconstruct_symbolic_word(b, 0);
    let c_word = reconstruct_symbolic_word(c, 0);

    match op {
        OpALU::Add => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Add(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Sub => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Sub(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Mul => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Add(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::And => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::And(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Or => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Or(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Xor => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Xor(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
        OpALU::Lt => LatticeVMSymbolicExpr::Sub(
            Box::new(a_word),
            Box::new(LatticeVMSymbolicExpr::Xor(
                Box::new(b_word.clone()),
                Box::new(c_word),
            )),
        ),
    }
}
