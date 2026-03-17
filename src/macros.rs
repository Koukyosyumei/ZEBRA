#[macro_export]
macro_rules! impl_p3_to_tv_conversion {
    (
        $SymbolicExpression:path, // SymbolicExpression列挙型へのフルパス
        $SymbolicVariable:path,   // SymbolicVariable構造体へのフルパス
        $PairCol:path,
        $VirtualPairCol:path,
        $PrimeField32:path,
        $convert_var_fn:path,     // 変数変換関数のパス
        $one_expr:expr
    ) => {
        // 型をエイリアスとしてインポート（バリアントへのアクセスに使用）
        use $PairCol as GeneralPairCol;
        use $PrimeField32 as GeneralPrimeField32;
        use $SymbolicExpression as GenericSymbolicExpression;
        use $SymbolicVariable as GenericSymbolicVariable;
        use $VirtualPairCol as GeneralVirtualPairCol;

        // PairColの変換（共通部分）
        pub fn convert_p3_paircol(pair_col: &GeneralPairCol) -> ZEBRASymbolicVal {
            match pair_col {
                PairCol::Main(index) => ZEBRASymbolicVal {
                    entry: ZEBRASymbolicEntry::Main { is_curr: true },
                    index: *index,
                },
                _ => todo!("This PairCol variant is not yet supported in ZEBRA conversion"),
            }
        }

        // VirtualPairColの変換
        fn get_weighted_var<F: GeneralPrimeField32>(
            paircol: &GeneralPairCol,
            weight: &F,
        ) -> ZEBRASymbolicExpr {
            ZEBRASymbolicExpr::Mul(
                Box::new(ZEBRASymbolicExpr::Variable(convert_p3_paircol(paircol))),
                Box::new(ZEBRASymbolicExpr::Constant(
                    AbstractInterval::from_i128(weight.as_canonical_u32() as i128),
                )),
            )
        }

        pub fn convert_p3_virtual_pair_col<F: GeneralPrimeField32>(
            vpair: &GeneralVirtualPairCol<F>,
        ) -> ZEBRASymbolicExpr {
            if vpair.column_weights.is_empty() {
                ZEBRASymbolicExpr::Constant(AbstractInterval {
                    lo: vpair.constant.as_canonical_u32() as i128,
                    hi: vpair.constant.as_canonical_u32() as i128,
                })
            } else {
                let mut expr = ZEBRASymbolicExpr::Constant(AbstractInterval {
                    lo: vpair.constant.as_canonical_u32() as i128,
                    hi: vpair.constant.as_canonical_u32() as i128,
                });
                let one: F = $one_expr;
                for (paircol, w) in vpair.column_weights.iter() {
                    if (w.clone() + one).as_canonical_u32() == 0 {
                        expr = ZEBRASymbolicExpr::Sub(
                            Box::new(expr.clone()),
                            Box::new(get_weighted_var(paircol, &one)),
                        );
                    } else {
                        expr = ZEBRASymbolicExpr::Add(
                            Box::new(expr.clone()),
                            Box::new(get_weighted_var(paircol, w)),
                        );
                    }
                }
                expr
            }
        }

        // 式の変換ロジック
        pub fn convert_p3_expr<F>(expr: &GenericSymbolicExpression<F>) -> ZEBRASymbolicExpr
        where
            F: GeneralPrimeField32,
        {
            match expr {
                GenericSymbolicExpression::Variable(v) => {
                    ZEBRASymbolicExpr::Variable($convert_var_fn(v))
                }
                GenericSymbolicExpression::IsFirstRow => ZEBRASymbolicExpr::IsFirstRow,
                GenericSymbolicExpression::IsLastRow => ZEBRASymbolicExpr::IsLastRow,
                GenericSymbolicExpression::IsTransition => ZEBRASymbolicExpr::IsTransition,
                GenericSymbolicExpression::Constant(v) => {
                    ZEBRASymbolicExpr::Constant(AbstractInterval {
                        lo: v.as_canonical_u32() as i128,
                        hi: v.as_canonical_u32() as i128,
                    })
                }
                GenericSymbolicExpression::Add { x, y, .. } => ZEBRASymbolicExpr::Add(
                    Box::new(convert_p3_expr(x)),
                    Box::new(convert_p3_expr(y)),
                ),
                GenericSymbolicExpression::Sub { x, y, .. } => ZEBRASymbolicExpr::Sub(
                    Box::new(convert_p3_expr(x)),
                    Box::new(convert_p3_expr(y)),
                ),
                GenericSymbolicExpression::Neg { x, .. } => {
                    ZEBRASymbolicExpr::Neg(Box::new(convert_p3_expr(x)))
                }
                GenericSymbolicExpression::Mul { x, y, .. } => ZEBRASymbolicExpr::Mul(
                    Box::new(convert_p3_expr(x)),
                    Box::new(convert_p3_expr(y)),
                ),
            }
        }
    };
}
