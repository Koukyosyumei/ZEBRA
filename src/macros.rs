#[macro_export]
macro_rules! impl_p3_to_tv_conversion {
    (
        $SymbolicExpression:path, // SymbolicExpression列挙型へのフルパス
        $SymbolicVariable:path,   // SymbolicVariable構造体へのフルパス
        $PairCol:path,
        $VirtualPairCol:path,
        $PrimeField32:path,
        $p3_air:path,            // p3_air クレート/モジュールへのパス
        $p3_field:path,          // p3_field クレート/モジュールへのパス
        $convert_var_fn:path     // 変数変換関数のパス
    ) => {
        // 型をエイリアスとしてインポート（バリアントへのアクセスに使用）
        use $PairCol as GeneralPairCol;
        use $PrimeField32 as GeneralPrimeField32;
        use $SymbolicExpression as GenericSymbolicExpression;
        use $SymbolicVariable as GenericSymbolicVariable;
        use $VirtualPairCol as GeneralVirtualPairCol;

        // PairColの変換（共通部分）
        pub fn convert_p3_paircol(pair_col: &GeneralPairCol) -> LatticeVMSymbolicVal {
            match pair_col {
                PairCol::Main(index) => LatticeVMSymbolicVal {
                    entry: LatticeVMSymbolicEntry::Main { is_curr: true },
                    index: *index,
                },
                _ => todo!("This PairCol variant is not yet supported in LatticeVM conversion"),
            }
        }

        // VirtualPairColの変換
        fn get_weighted_var<F: GeneralPrimeField32>(
            paircol: &GeneralPairCol,
            weight: &F,
        ) -> LatticeVMSymbolicExpr {
            LatticeVMSymbolicExpr::Mul(
                Box::new(LatticeVMSymbolicExpr::Variable(convert_p3_paircol(paircol))),
                Box::new(LatticeVMSymbolicExpr::Constant(AbstractInterval::from_i64(
                    weight.as_canonical_u32() as i64,
                ))),
            )
        }

        pub fn convert_p3_virtual_pair_col<F: GeneralPrimeField32>(
            vpair: &GeneralVirtualPairCol<F>,
        ) -> LatticeVMSymbolicExpr {
            if vpair.column_weights.is_empty() {
                LatticeVMSymbolicExpr::Constant(AbstractInterval {
                    lo: vpair.constant.as_canonical_u32() as i64,
                    hi: vpair.constant.as_canonical_u32() as i64,
                })
            } else {
                let mut expr = LatticeVMSymbolicExpr::Constant(AbstractInterval {
                    lo: vpair.constant.as_canonical_u32() as i64,
                    hi: vpair.constant.as_canonical_u32() as i64,
                });
                for (paircol, w) in vpair.column_weights.iter() {
                    if (w.clone() + F::one()).as_canonical_u32() == 0 {
                        expr = LatticeVMSymbolicExpr::Sub(
                            Box::new(expr.clone()),
                            Box::new(get_weighted_var(paircol, &F::one())),
                        );
                    } else {
                        expr = LatticeVMSymbolicExpr::Add(
                            Box::new(expr.clone()),
                            Box::new(get_weighted_var(paircol, w)),
                        );
                    }
                }
                expr
            }
        }

        // 式の変換ロジック
        pub fn convert_p3_expr<F>(expr: &GenericSymbolicExpression<F>) -> LatticeVMSymbolicExpr
        where
            F: GeneralPrimeField32,
        {
            match expr {
                GenericSymbolicExpression::Variable(v) => {
                    LatticeVMSymbolicExpr::Variable($convert_var_fn(v))
                }
                GenericSymbolicExpression::IsFirstRow => LatticeVMSymbolicExpr::IsFirstRow,
                GenericSymbolicExpression::IsLastRow => LatticeVMSymbolicExpr::IsLastRow,
                GenericSymbolicExpression::IsTransition => LatticeVMSymbolicExpr::IsTransition,
                GenericSymbolicExpression::Constant(v) => {
                    LatticeVMSymbolicExpr::Constant(AbstractInterval {
                        lo: v.as_canonical_u32() as i64,
                        hi: v.as_canonical_u32() as i64,
                    })
                }
                GenericSymbolicExpression::Add { x, y, .. } => LatticeVMSymbolicExpr::Add(
                    Box::new(convert_p3_expr(x)),
                    Box::new(convert_p3_expr(y)),
                ),
                GenericSymbolicExpression::Sub { x, y, .. } => LatticeVMSymbolicExpr::Sub(
                    Box::new(convert_p3_expr(x)),
                    Box::new(convert_p3_expr(y)),
                ),
                GenericSymbolicExpression::Neg { x, .. } => {
                    LatticeVMSymbolicExpr::Neg(Box::new(convert_p3_expr(x)))
                }
                GenericSymbolicExpression::Mul { x, y, .. } => LatticeVMSymbolicExpr::Mul(
                    Box::new(convert_p3_expr(x)),
                    Box::new(convert_p3_expr(y)),
                ),
            }
        }
    };
}
