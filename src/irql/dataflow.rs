use super::{IrqlRequirement, IrqlValue};
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::mir::{
    BasicBlock, Body, Const, ConstOperand, ConstValue, Location, Operand, Statement, Terminator,
    TerminatorEdges,
};
use rustc_middle::ty::{self, Instance, TypingEnv};
use rustc_mir_dataflow::{Analysis, JoinSemiLattice};
use rustc_span::source_map::Spanned;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IrqlState {
    pub current: IrqlValue,
    pub stack: Vec<IrqlValue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrqlStateOrError {
    Ok(IrqlState),
    Error,
}

impl JoinSemiLattice for IrqlStateOrError {
    fn join(&mut self, other: &IrqlStateOrError) -> bool {
        match (&self, other) {
            (IrqlStateOrError::Error, _) | (_, IrqlStateOrError::Error) => {
                *self = IrqlStateOrError::Error;
                true
            }
            (IrqlStateOrError::Ok(a), IrqlStateOrError::Ok(b)) => {
                if a.current != b.current {
                    *self = IrqlStateOrError::Error;
                    true
                } else {
                    false
                }
            }
        }
    }
}

pub struct IrqlTrackingAnalysis<'mir, 'tcx, 'checker> {
    pub checker: &'checker crate::ctxt::AnalysisCtxt<'tcx>,
    pub body: &'mir Body<'tcx>,
    pub typing_env: TypingEnv<'tcx>,
    pub instance: Instance<'tcx>,
}

impl<'tcx> Analysis<'tcx> for IrqlTrackingAnalysis<'_, 'tcx, '_> {
    type Domain = IrqlStateOrError;
    const NAME: &'static str = "irql rules";

    fn bottom_value(&self, _body: &Body<'tcx>) -> Self::Domain {
        IrqlStateOrError::Ok(IrqlState {
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

        let IrqlStateOrError::Ok(irql) = state else {
            return terminator.edges();
        };

        if let Call { func, args, .. } = &terminator.kind {
            let function_ty = func.ty(self.body, self.checker.tcx);
            let function_ty = self.instance.instantiate_mir_and_normalize_erasing_regions(
                self.checker.tcx,
                self.typing_env,
                ty::EarlyBinder::bind(function_ty),
            );

            if let ty::FnDef(def_id, _) = *function_ty.kind() {
                let annotation = self.checker.irql_annotation(def_id);

                if annotation.requirement.is_some() {
                    let requirement = match annotation.requirement.unwrap() {
                        IrqlRequirement::Call(requirement) => requirement,
                        IrqlRequirement::Permanent(requirement) => requirement,
                    };
                    if irql.current < requirement.low
                        || requirement.high.is_some_and(|h| irql.current > h)
                    {
                        *state = IrqlStateOrError::Error;
                        return terminator.edges();
                    }
                }

                if let Some(new_level) = annotation.raise {
                    irql.stack.push(irql.current);
                    irql.current = new_level;
                }

                if self.checker.tcx.item_name(def_id).as_str() == "KeRaiseIrql" {
                    if let Some(raises) = extract_irql_from_args(args) {
                        irql.stack.push(irql.current);
                        irql.current = raises;
                    }
                }

                if self.checker.tcx.item_name(def_id).as_str() == "KeLowerIrql" {
                    if irql.stack.is_empty() {
                        *state = IrqlStateOrError::Error;
                        return terminator.edges();
                    }
                    let expected = irql.stack.pop().unwrap();

                    if let Some(lower) = extract_irql_from_args(args) {
                        if lower != expected {
                            *state = IrqlStateOrError::Error;
                            return terminator.edges();
                        }
                        irql.current = lower;
                    }
                }
            }
        }

        terminator.edges()
    }

    fn apply_call_return_effect(
        &mut self,
        state: &mut Self::Domain,
        _block: BasicBlock,
        _return_places: rustc_middle::mir::CallReturnPlaces<'_, 'tcx>,
    ) {
        let IrqlStateOrError::Ok(state) = state else {
            return;
        };

        if !state.stack.is_empty()
            && self
                .checker
                .irql_annotation(self.instance.def_id())
                .raise
                .is_none()
        {
            println!("Function raises IRQL but does not lower it. Consider lowering the IRQL or adding an explicit `raise` annotation.");
        }
    }
}

fn extract_irql_from_args(args: &[Spanned<Operand<'_>>]) -> Option<IrqlValue> {
    let Spanned {
        node:
            Operand::Constant(box ConstOperand {
                const_: Const::Val(ConstValue::Scalar(Scalar::Int(scalar_int)), _),
                ..
            }),
        ..
    } = args[0]
    else {
        return None;
    };

    Some(IrqlValue {
        value: scalar_int.to_u32(),
    })
}
