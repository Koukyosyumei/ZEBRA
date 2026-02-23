use crate::interval::AbstractInterval;
use crate::symbolic::LatticeVMSymbolicExpr as LExpr;
use crate::wordop::reconstruct_symbolic_word;

#[derive(Debug)]
pub enum ControFLowOp {
    BEQ,
    BGT,
    BGE,
    BGEU,
    BLE,
    BLT,
    BLTU,
    BNE,
    JAL,
    JALR,
    Jumpi,
}

pub fn get_control_flow_constraint(
    pc: &LExpr,
    next_pc: &LExpr,
    a: &[LExpr; 4],
    b: &[LExpr; 4],
    c: &[LExpr; 4],
    op: &ControFLowOp,
    default_step: i128,
) -> Vec<LExpr> {
    let a_box = a.clone().map(|f| Box::new(f));
    let b_box = b.clone().map(|f| Box::new(f));
    let c_box = c.clone().map(|f| Box::new(f));
    //let a_expr = reconstruct_symbolic_word(a, 0);
    let b_expr = reconstruct_symbolic_word(b, 0);
    let c_expr = reconstruct_symbolic_word(c, 0);

    match op {
        ControFLowOp::BEQ => vec![
            LExpr::WhenNonZero(
                Box::new(LExpr::WordEq(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenZero(
                Box::new(LExpr::WordEq(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BNE => vec![
            LExpr::WhenNonZero(
                Box::new(LExpr::WordNEq(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenZero(
                Box::new(LExpr::WordNEq(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BGE => vec![
            LExpr::WhenZero(
                Box::new(LExpr::WordSLt(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenNonZero(
                Box::new(LExpr::WordSLt(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BGT => vec![
            LExpr::WhenZero(
                Box::new(LExpr::WordSLe(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenNonZero(
                Box::new(LExpr::WordSLe(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BLE => vec![
            LExpr::WhenNonZero(
                Box::new(LExpr::WordSLe(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenZero(
                Box::new(LExpr::WordSLe(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BLT => vec![
            LExpr::WhenNonZero(
                Box::new(LExpr::WordSLt(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenZero(
                Box::new(LExpr::WordSLt(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BGEU => vec![
            LExpr::WhenZero(
                Box::new(LExpr::WordLt(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenNonZero(
                Box::new(LExpr::WordLt(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::BLTU => vec![
            LExpr::WhenNonZero(
                Box::new(LExpr::WordLt(a_box.clone(), b_box.clone())),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(c_expr))),
                )),
            ),
            LExpr::WhenZero(
                Box::new(LExpr::WordLt(a_box, b_box)),
                Box::new(LExpr::Sub(
                    Box::new(next_pc.clone()),
                    Box::new(LExpr::Add(
                        Box::new(pc.clone()),
                        Box::new(LExpr::Constant(AbstractInterval::from_i128(default_step))),
                    )),
                )),
            ),
        ],
        ControFLowOp::Jumpi => vec![LExpr::Sub(Box::new(next_pc.clone()), Box::new(b_expr))],
        ControFLowOp::JAL => vec![LExpr::Sub(
            Box::new(next_pc.clone()),
            Box::new(LExpr::Add(Box::new(pc.clone()), Box::new(b_expr))),
        )],
        ControFLowOp::JALR => vec![LExpr::Sub(
            Box::new(next_pc.clone()),
            Box::new(LExpr::WordAdd(b_box, c_box)),
        )],
    }
}
