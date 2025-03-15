use crate::ctxt::AnalysisCtxt;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyCtxt;
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_tool_lint! {
    pub klint::IRQL_RULES,
    Deny,
    "Generates an error if IRQL rules are not respected"
}

pub struct IrqlRules<'tcx> {
    cx: AnalysisCtxt<'tcx>,
}

impl<'tcx> IrqlRules<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> Self {
        Self {
            cx: AnalysisCtxt::new(tcx),
        }
    }
}

impl_lint_pass!(IrqlRules<'_> => [IRQL_RULES]);

impl<'tcx> LateLintPass<'tcx> for IrqlRules<'tcx> {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'_>) {
        match &expr.kind {
            rustc_hir::ExprKind::Call(_, _) | rustc_hir::ExprKind::MethodCall(_, _, _, _) => {
                let Some(function_def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id)
                else {
                    return;
                };
                let caller_def_id = cx.tcx.hir().enclosing_body_owner(expr.hir_id).to_def_id();

                let function_irql = self.cx.irql_annotation(function_def_id);
                let caller_irql = self.cx.irql_annotation(caller_def_id);
            }
            _ => {}
        }
    }
}
