use super::IrqlValue;
use crate::error::Error;
use crate::irql::{utils, IrqlRequirement};
use rustc_middle::mir::interpret::Scalar;
use rustc_middle::mir::{
    BasicBlock, Body, Const, ConstValue, Local, Location, Operand, Rvalue, Statement,
    StatementKind, Terminator, TerminatorEdges,
};
use rustc_middle::ty::{self, Instance, TypingEnv};
use rustc_mir_dataflow::{fmt::DebugWithContext, Analysis, JoinSemiLattice};
use rustc_span::DUMMY_SP;
use std::collections::HashMap;

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
    pub known_values: HashMap<Local, IrqlValue>,
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
            (_, IrqlStateOrError::Error(e)) => {
                *self = IrqlStateOrError::Error(*e);
                true
            }
            (IrqlStateOrError::IrqlState(a), IrqlStateOrError::IrqlState(b)) => {
                if a.stack.is_empty() {
                    *self = IrqlStateOrError::IrqlState(b.clone());
                    return true;
                }

                // TODO: Is this possible?
                if b.stack.is_empty() {
                    return false;
                }

                if a.current != b.current {
                    let left_span = a.stack.last().map_or(DUMMY_SP, |e| e.span);
                    let right_span = b.stack.last().map_or(DUMMY_SP, |e| e.span);

                    *self = IrqlStateOrError::Error(Error::IrqlImpossibleJoin {
                        left_value: a.current,
                        right_value: b.current,
                        left_span,
                        right_span,
                    });

                    return true;
                }

                false
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
            known_values: HashMap::new(),
        })
    }

    fn initialize_start_block(&self, body: &Body<'tcx>, state: &mut Self::Domain) {
        // TODO: Issue found during testing - No entry point is set yet => Can generate a false positive in one place and none in another.
        *state = self.bottom_value(body);
    }

    fn apply_primary_statement_effect(
        &mut self,
        state: &mut Self::Domain,
        statement: &Statement<'tcx>,
        _location: Location,
    ) {
        let IrqlStateOrError::IrqlState(irql_state) = state else {
            return;
        };

        if let StatementKind::Assign(box (place, Rvalue::Use(Operand::Constant(c)))) =
            &statement.kind
        {
            if let Some(local) = place.as_local() {
                if let Const::Val(ConstValue::Scalar(Scalar::Int(scalar)), _) = c.const_ {
                    irql_state.known_values.insert(
                        local,
                        IrqlValue {
                            value: scalar.to_u32(),
                        },
                    );
                }
            }
        }
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
            let Some(new_level) = utils::extract_irql_from_args(irql_state, args, 0) else {
                return terminator.edges();
            };

            if let Some(IrqlRequirement::Permanent(range)) =
                self.checker.irql_annotation(def_id).requirement
            {
                if !range.contains(new_level) {
                    *state = IrqlStateOrError::Error(Error::IrqlOutsideOfPermanentRequirement {
                        change_to: new_level,
                        permanent_range: range,
                        span: terminator.source_info.span,
                    });
                    return terminator.edges();
                }
            }

            irql_state.stack.push(IrqlStackEntry::from_irql_value(
                irql_state.current,
                terminator,
            ));
            irql_state.current = new_level;
        } else if name == "KeLowerIrql" {
            let Some(target_level) = utils::extract_irql_from_args(irql_state, args, 0) else {
                return terminator.edges();
            };

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

            if let Some(IrqlRequirement::Permanent(range)) =
                self.checker.irql_annotation(def_id).requirement
            {
                if !range.contains(target_level) {
                    *state = IrqlStateOrError::Error(Error::IrqlOutsideOfPermanentRequirement {
                        change_to: target_level,
                        permanent_range: range,
                        span: terminator.source_info.span,
                    });
                    return terminator.edges();
                }
            }

            irql_state.current = target_level;
        } else {
            let Some(new_level) = self.checker.irql_annotation(def_id).raise else {
                return terminator.edges();
            };

            if let Some(IrqlRequirement::Permanent(range)) =
                self.checker.irql_annotation(def_id).requirement
            {
                if !range.contains(new_level) {
                    *state = IrqlStateOrError::Error(Error::IrqlOutsideOfPermanentRequirement {
                        change_to: new_level,
                        permanent_range: range,
                        span: terminator.source_info.span,
                    });
                    return terminator.edges();
                }
            }

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
