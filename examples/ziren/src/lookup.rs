use p3_air::Air;
use p3_air::AirBuilder;
use p3_koala_bear::KoalaBear;
use p3_uni_stark::{SymbolicExpression, SymbolicVariable};

use zkm_stark::LookupBuilder;

use crate::p3_to_tv::convert_p3_expr;

pub fn inspect_lookup(builder: LookupBuilder<KoalaBear>) {
    /*
    let mut builder = LookupBuilder::<KoalaBear>::new(0, NUM_CPU_COLS);
    air.eval(&mut builder);
     */
    ///////////////////////////////////////////////////////
    let mut main = builder.main();
    let (sends, receives) = builder.lookups();

    for lookup in receives {
        print!("Receive values: ");
        for value in lookup.values {
            let expr = value.apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );
            print!("{}, ", convert_p3_expr::<KoalaBear>(&expr));
        }

        let multiplicity = lookup
            .multiplicity
            .apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );

        println!(
            "   multiplicity: {}",
            convert_p3_expr::<KoalaBear>(&multiplicity)
        );

        println!("  scope: {:?}, kind: {:?}", lookup.scope, lookup.kind);
    }

    for lookup in sends {
        print!("Send values: ");
        for value in lookup.values {
            let expr = value.apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );
            print!("{}, ", convert_p3_expr::<KoalaBear>(&expr));
        }

        let multiplicity = lookup
            .multiplicity
            .apply::<SymbolicExpression<KoalaBear>, SymbolicVariable<KoalaBear>>(
                &[],
                main.row_mut(0),
            );

        println!(
            "   multiplicity: {}",
            convert_p3_expr::<KoalaBear>(&multiplicity)
        );

        println!("  scope: {:?}, kind: {:?}", lookup.scope, lookup.kind);
    }
}
