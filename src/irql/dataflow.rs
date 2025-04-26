use super::IrqlValue;
use crate::error::Error;
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::mir::{
    BasicBlock, Body, Const, ConstOperand, ConstValue, Location, Operand, Statement, Terminator,
    TerminatorEdges,
};
use rustc_middle::ty::{self, Instance, TypingEnv};
use rustc_mir_dataflow::{fmt::DebugWithContext, Analysis, JoinSemiLattice};
use rustc_span::source_map::Spanned;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrqlStackEntry {
    pub value: IrqlValue,
    pub span: rustc_span::Span,
}

impl IrqlStackEntry {
    pub fn from_irql_value(value: IrqlValue, terminator: &Terminator) -> IrqlStackEntry {
        IrqlStackEntry {
            value,
            span: terminator.source_info.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrqlState {
    pub current: IrqlValue,
    pub stack: Vec<IrqlStackEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrqlStateOrError {
    IrqlState(IrqlState),
    Error(Error),
}

impl JoinSemiLattice for IrqlStateOrError {
    fn join(&mut self, other: &IrqlStateOrError) -> bool {
        match (&self, other) {
            (IrqlStateOrError::Error(_), _) => false,
            (_, IrqlStateOrError::Error(error)) => {
                *self = IrqlStateOrError::Error(*error);
                true
            }
            (IrqlStateOrError::IrqlState(a), IrqlStateOrError::IrqlState(b)) => {
                if a.current != b.current {
                    *self = IrqlStateOrError::Error(Error::IrqlImpossibleJoin);
                    true
                } else {
                    false
                }
            }
        }
    }
}

pub struct IrqlComputation<'mir, 'tcx, 'checker> {
    pub checker: &'checker crate::ctxt::AnalysisCtxt<'tcx>,
    pub body: &'mir Body<'tcx>,
    pub typing_env: TypingEnv<'tcx>,
    pub instance: Instance<'tcx>,
}

impl DebugWithContext<IrqlComputation<'_, '_, '_>> for IrqlStateOrError {}

impl<'tcx> Analysis<'tcx> for IrqlComputation<'_, 'tcx, '_> {
    type Domain = IrqlStateOrError;
    const NAME: &'static str = "irql rules";

    fn bottom_value(&self, _body: &Body<'tcx>) -> Self::Domain {
        IrqlStateOrError::IrqlState(IrqlState {
            current: IrqlValue::passive_level(),
            stack: vec![],
        })
    }

    fn initialize_start_block(&self, _body: &Body<'tcx>, state: &mut Self::Domain) {
        *state = self.bottom_value(_body);
    }

    fn apply_primary_statement_effect(
        &mut self,
        _state: &mut Self::Domain,
        _statement: &Statement<'tcx>,
        _location: Location,
    ) {
    }

    fn apply_primary_terminator_effect<'mir>(
        &mut self,
        state: &mut Self::Domain,
        terminator: &'mir Terminator<'tcx>,
        _location: Location,
    ) -> TerminatorEdges<'mir, 'tcx> {
        use rustc_middle::mir::TerminatorKind::*;

        let IrqlStateOrError::IrqlState(irql_state) = state else {
            return terminator.edges();
        };

        let Call { func, args, .. } = &terminator.kind else {
            return terminator.edges();
        };

        let function_ty = func.ty(self.body, self.checker.tcx);
        let function_ty = self.instance.instantiate_mir_and_normalize_erasing_regions(
            self.checker.tcx,
            self.typing_env,
            ty::EarlyBinder::bind(function_ty),
        );

        let ty::FnDef(def_id, _) = *function_ty.kind() else {
            return terminator.edges();
        };

        let name = self.checker.tcx.item_name(def_id);
        let name = name.as_str();

        if name == "KeRaiseIrql" {
            let new_level = extract_irql_from_args(args, 0);
            println!("raise {}", new_level.value);
            irql_state.stack.push(IrqlStackEntry::from_irql_value(
                irql_state.current,
                terminator,
            ));
            irql_state.current = new_level;
        } else if name == "KeLowerIrql" {
            let target_level = extract_irql_from_args(args, 0);
            println!("lower {}", target_level.value);
            let Some(previous) = irql_state.stack.pop() else {
                *state = IrqlStateOrError::Error(Error::IrqlStackUnderflow {
                    span: terminator.source_info.span,
                });
                return terminator.edges();
            };

            if previous.value != target_level {
                *state = IrqlStateOrError::Error(Error::IrqlLoweringMismatch {
                    expected: previous.value,
                    actual: target_level,
                    span: terminator.source_info.span,
                });
                return terminator.edges();
            }

            irql_state.current = target_level;
        } else {
            // Handle raise annotations
            let Some(new_level) = self.checker.irql_annotation(def_id).raise else {
                return terminator.edges();
            };
            println!("annotation {}", new_level.value);

            irql_state.stack.push(IrqlStackEntry::from_irql_value(
                irql_state.current,
                terminator,
            ));
            irql_state.current = new_level;
        }

        terminator.edges()
    }

    fn apply_call_return_effect(
        &mut self,
        _state: &mut Self::Domain,
        _block: BasicBlock,
        _return_places: rustc_middle::mir::CallReturnPlaces<'_, 'tcx>,
    ) {
    }
}

fn extract_irql_from_args(args: &[Spanned<Operand<'_>>], arg_index: usize) -> IrqlValue {
    let Spanned {
        node:
            Operand::Constant(box ConstOperand {
                const_: Const::Val(ConstValue::Scalar(Scalar::Int(scalar_int)), _),
                ..
            }),
        ..
    } = args[arg_index]
    else {
        panic!("Could not extract IrqlValue from argument with index {arg_index}.");
    };

    IrqlValue {
        value: scalar_int.to_u32(),
    }
}
