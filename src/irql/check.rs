use super::dataflow::{IrqlComputation, IrqlStateOrError};
use crate::ctxt::AnalysisCtxt;
use crate::error::Error;
use rustc_middle::mir::{Body, TerminatorKind};
use rustc_middle::ty::{EarlyBinder, FnDef, Instance, TypingEnv};
use rustc_mir_dataflow::Analysis;

impl<'tcx> AnalysisCtxt<'tcx> {
    pub fn check_irql(
        &self,
        typing_env: TypingEnv<'tcx>,
        instance: Instance<'tcx>,
        body: &Body<'tcx>,
    ) {
        let mut irql_computation = IrqlComputation {
            checker: self,
            body,
            typing_env,
            instance,
        }
        .iterate_to_fixpoint(self.tcx, body, None)
        .into_results_cursor(body);

        for (bb, block_data) in rustc_middle::mir::traversal::reachable(body) {
            if block_data.is_cleanup {
                continue;
            }

            irql_computation.seek_to_block_start(bb);
            let irql_state_or_error = irql_computation.get();

            match irql_state_or_error {
                IrqlStateOrError::Error(error) => match error {
                    Error::TooGeneric => {}
                    Error::IrqlImpossibleJoin {
                        left_value,
                        right_value,
                        left_span,
                        right_span,
                    } => {
                        let mut diag = self
                            .dcx()
                            .struct_err("IRQL join error at control-flow merge");
                        diag.span_note(
                            *left_span,
                            format!("This branch ends with IRQL level `{}`", left_value),
                        );
                        diag.span_note(
                            *right_span,
                            format!("This branch ends with IRQL level `{}`", right_value),
                        );
                        diag.help("All control-flow paths must end with the same IRQL level to be joinable.");
                        diag.emit();
                    }
                    Error::IrqlStackUnderflow { span } => {
                        let mut diag = self.dcx().struct_err("IRQl stack underflow error");
                        diag.span_note(
                            *span,
                            "KeLowerIrql called but there was no previous raise.",
                        );
                        diag.help("Ensure you raise IRQL before lowering.");
                        diag.emit();
                    }
                    Error::IrqlLoweringMismatch {
                        expected,
                        actual,
                        span,
                    } => {
                        let mut diag = self.dcx().struct_err("IRQl lowering error");
                        diag.span_note(
                            *span,
                            format!(
                                "KeLowerIrql lowered IRQL to {}, but expected {}.",
                                actual, expected
                            ),
                        );
                        diag.help("Ensure you lower to the original IRQL level before the previous raise.");
                        diag.emit();
                    }
                    Error::IrqlOutsideOfPermanentRequirement {
                        change_to,
                        permanent_range,
                        span,
                    } => {
                        let mut diag = self.dcx().struct_err("IRQl permanent requirement error");
                        diag.span_note(
                            *span,
                            format!(
                                "IRQL changed to {}, outside permanent requirement {}.",
                                change_to, permanent_range
                            ),
                        );
                        diag.help(
                            "Ensure the IRQL level does not go oustide the permanent bounds.",
                        );
                        diag.emit();
                    }
                    Error::Guaranteed(_) => {}
                },
                IrqlStateOrError::IrqlState(irql_state) => {
                    let terminator = block_data.terminator();
                    let TerminatorKind::Call { func, .. } = &terminator.kind else {
                        continue;
                    };

                    let function_ty = func.ty(body, self.tcx);
                    let function_ty = instance.instantiate_mir_and_normalize_erasing_regions(
                        self.tcx,
                        typing_env,
                        EarlyBinder::bind(function_ty),
                    );

                    let FnDef(def_id, _) = *function_ty.kind() else {
                        continue;
                    };

                    let Some(range) = self
                        .irql_annotation(def_id)
                        .requirement
                        .map(|requirement| requirement.range())
                    else {
                        continue;
                    };

                    if !range.contains(irql_state.current) {
                        let span = terminator.source_info.span;
                        let mut diag = self.dcx().struct_err("IRQl call requirement error");
                        diag.span_note(
                            span,
                            format!(
                                "IRQL is {} when calling `{}`, but it requires {}",
                                irql_state.current,
                                self.tcx.item_name(def_id),
                                range
                            ),
                        );
                        diag.help("Ensure IRQL is raised to the correct level before this call.");
                        diag.emit();
                    }
                }
            }
        }

        irql_computation.seek_to_block_start(body.basic_blocks.last_index().unwrap());
        let IrqlStateOrError::IrqlState(final_state) = irql_computation.get() else {
            return;
        };

        let annotation = self.irql_annotation(instance.def_id());

        if !final_state.stack.is_empty() && annotation.raise.is_none() {
            let mut diag = self.dcx().struct_err("IRQl raise not caught error");
            diag.span_note(
                body.span,
                "Function raises IRQL, but does not lower it before returning.",
            );
            diag.help(format!("Either lower the IRQL before returning or add a `#[klint::irql(raise = {})]` annotation.", final_state.current));
            diag.emit();
        }

        let Some(declared_raise) = annotation.raise else {
            return;
        };

        if final_state.current != declared_raise {
            let mut diag = self.dcx().struct_err("IRQl annotation mismatch error");
            diag.span_note(
                body.span,
                format!(
                    "Function is annotated to raise IRQL to {}, but ends at IRQL {}.",
                    declared_raise, final_state.current
                ),
            );
            diag.help("Ensure the raise annotation and the final IRQL values match.");
            diag.emit();
        }
    }
}
