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
        let function_def_id = match &expr.kind {
            rustc_hir::ExprKind::Call(func, _) => {
                if let rustc_hir::ExprKind::Path(rustc_hir::QPath::Resolved(_, path)) = func.kind {
                    path.res.opt_def_id()
                } else {
                    None
                }
            }
            rustc_hir::ExprKind::MethodCall(_, _, _, _) => {
                cx.typeck_results().type_dependent_def_id(expr.hir_id)
            }
            _ => None,
        };

        let Some(function_def_id) = function_def_id else {
            return;
        };
        let caller_def_id = cx.tcx.hir().enclosing_body_owner(expr.hir_id).to_def_id();

        let function_irql = self.cx.irql_annotation(function_def_id);
        let caller_irql = self.cx.irql_annotation(caller_def_id);

        if let Some(on_return_value) = function_irql.on_return_value
            && let Some(permanent_requirement) = caller_irql.permanent_requirement
        {
            if let Some(high) = permanent_requirement.high {
                if on_return_value < permanent_requirement.low || on_return_value > high {
                    println!("Function which raises IRQL to {} called from function with permanent requirement in interval {} to {}", on_return_value.value, permanent_requirement.low.value, high.value);
                }
            } else if on_return_value != permanent_requirement.low {
                println!("Function which raises IRQL to {} called from function with permanent requirement {}", on_return_value.value, permanent_requirement.low.value);
            }
        }
    }
}
