use super::{IrqlRequirement, IrqlValue};
use crate::ctxt::AnalysisCtxt;
use crate::error::Error;
use crate::use_site::{UseSite, UseSiteKind};
use rustc_middle::mir::{
    BasicBlock, Body, Location, Statement, Terminator, TerminatorEdges, TerminatorKind,
};
use rustc_middle::ty::{self, Instance, TypingEnv};
use rustc_mir_dataflow::JoinSemiLattice;
use rustc_mir_dataflow::{fmt::DebugWithContext, Analysis};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IrqlState {
    pub current: IrqlValue,
    pub stack: Vec<IrqlValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrqlStateOrError {
    Ok(IrqlState),
    Error,
}

impl JoinSemiLattice for IrqlStateOrError {
    fn join(&mut self, other: &Self) -> bool {
        match (self, other) {
            (IrqlStateOrError::Error, _) | (_, IrqlStateOrError::Error) => {
                *self = IrqlStateOrError::Error;
                true
            }
            (IrqlStateOrError::Ok(a), IrqlStateOrError::Ok(b)) => {
                // Conservative join: if current IRQL differs, mark error
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
        location: Location,
    ) -> TerminatorEdges<'mir, 'tcx> {
        use rustc_middle::mir::TerminatorKind::*;

        let IrqlStateOrError::Ok(irql) = state else {
            return terminator.edges();
        };

        match &terminator.kind {
            Call { func, args, .. } => {
                let function_ty = func.ty(self.body, self.checker.tcx);
                let callee_ty = self.instance.instantiate_mir_and_normalize_erasing_regions(
                    self.checker.tcx,
                    self.typing_env,
                    ty::EarlyBinder::bind(function_ty),
                );

                if let ty::FnDef(def_id, substs) = *callee_ty.kind() {
                    let annotation = self.checker.irql_annotation(def_id);

                    // Check require
                    if let Some(IrqlRequirement::Call(req_range)) = annotation.requirement {
                        if irql.current < req_range.low
                            || req_range.high.map_or(false, |h| irql.current > h)
                        {
                            *state = IrqlStateOrError::Error;
                            return terminator.edges();
                        }
                    }

                    // Raise
                    if let Some(new_level) = annotation.raise {
                        irql.stack.push(irql.current);
                        irql.current = new_level;
                    }

                    // KeRaiseIrql
                    if self.checker.is_special_function(def_id, "KeRaiseIrql") {
                        if let Some(raise_to) = extract_irql_from_args(args) {
                            irql.stack.push(irql.current);
                            irql.current = raise_to;
                        }
                    }

                    // KeLowerIrql
                    if self.checker.is_special_function(def_id, "KeLowerIrql") {
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

            _ => {}
        }

        terminator.edges()
    }
}
