use super::{dataflow::IrqlState, IrqlValue};
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::mir::{Const, ConstOperand, ConstValue, Operand};
use rustc_span::source_map::Spanned;

pub fn extract_irql_from_args(
    state: &IrqlState,
    args: &[Spanned<Operand<'_>>],
    arg_index: usize,
) -> Option<IrqlValue> {
    match &args.get(arg_index)?.node {
        Operand::Constant(box ConstOperand {
            const_: Const::Val(ConstValue::Scalar(Scalar::Int(scalar)), _),
            ..
        }) => Some(IrqlValue {
            value: scalar.to_u32(),
        }),

        Operand::Move(place) | Operand::Copy(place) => {
            if let Some(local) = place.as_local() {
                if let Some(val) = state.known_values.get(&local) {
                    return Some(*val);
                } else {
                    println!("No known constant value for local variable {:?}", local);
                }
            }
            None
        }

        other => {
            println!(
                "Unhandled operand kind for IRQL extraction from arguments: {:?}",
                other
            );
            None
        }
    }
}
