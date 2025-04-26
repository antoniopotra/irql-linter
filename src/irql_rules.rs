use crate::ctxt::AnalysisCtxt;
use crate::irql::IrqlRequirement;
use rustc_hir::def_id::LocalDefId;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{GenericArgs, Instance, TyCtxt, TypingEnv};
use rustc_session::{declare_tool_lint, impl_lint_pass};
use rustc_span::Span;

declare_tool_lint! {
    pub klint::IRQL_RULES,
    Deny,
    "Generates an error if IRQL rules are not respected"
}

pub struct IrqlRules<'tcx> {
    cx: AnalysisCtxt<'tcx>,
}

impl<'tcx> IrqlRules<'tcx> {
    pub fn new(tcx: TyCtxt<'tcx>) -> IrqlRules<'tcx> {
        IrqlRules {
            cx: AnalysisCtxt::new(tcx),
        }
    }
}

impl_lint_pass!(IrqlRules<'_> => [IRQL_RULES]);

impl<'tcx> LateLintPass<'tcx> for IrqlRules<'tcx> {
    fn check_crate(&mut self, _: &LateContext<'tcx>) {
        // Skip checks for proc-macro crates.
        if self
            .cx
            .crate_types()
            .contains(&rustc_session::config::CrateType::ProcMacro)
        {
            return;
        }

        use rustc_hir::intravisit as hir_visit;
        use rustc_hir::*;

        struct FnAdtVisitor<'tcx, F, A> {
            tcx: TyCtxt<'tcx>,
            fn_callback: F,
            adt_callback: A,
        }

        impl<'tcx, F, A> hir_visit::Visitor<'tcx> for FnAdtVisitor<'tcx, F, A>
        where
            F: FnMut(LocalDefId),
            A: FnMut(LocalDefId),
        {
            type NestedFilter = rustc_middle::hir::nested_filter::All;

            /// Because lints are scoped lexically, we want to walk nested
            /// items in the context of the outer item, so enable
            /// deep-walking.
            fn nested_visit_map(&mut self) -> Self::Map {
                self.tcx.hir()
            }

            fn visit_item(&mut self, i: &'tcx Item<'tcx>) {
                match i.kind {
                    ItemKind::Struct(..) | ItemKind::Union(..) | ItemKind::Enum(..) => {
                        (self.adt_callback)(i.item_id().owner_id.def_id);
                    }
                    ItemKind::Trait(..) => {
                        // Not exactly an ADT, but we want to track drop_irql on traits as well.
                        (self.adt_callback)(i.item_id().owner_id.def_id);
                    }
                    _ => (),
                }
                hir_visit::walk_item(self, i);
            }

            fn visit_foreign_item(&mut self, i: &'tcx ForeignItem<'tcx>) {
                if let ForeignItemKind::Fn(..) = i.kind {
                    (self.fn_callback)(i.owner_id.def_id);
                }
                hir_visit::walk_foreign_item(self, i);
            }

            fn visit_trait_item(&mut self, ti: &'tcx TraitItem<'tcx>) {
                if let TraitItemKind::Fn(_, TraitFn::Required(_)) = ti.kind {
                    (self.fn_callback)(ti.owner_id.def_id);
                }
                hir_visit::walk_trait_item(self, ti)
            }

            fn visit_fn(
                &mut self,
                fk: hir_visit::FnKind<'tcx>,
                fd: &'tcx FnDecl<'tcx>,
                b: BodyId,
                _: Span,
                id: LocalDefId,
            ) {
                (self.fn_callback)(id);
                hir_visit::walk_fn(self, fk, fd, b, id)
            }
        }

        // Do this before the lint pass to ensure that errors, if any, are nicely sorted.
        self.cx
            .hir()
            .visit_all_item_likes_in_crate(&mut FnAdtVisitor {
                tcx: self.cx.tcx,
                fn_callback: |def_id: LocalDefId| {
                    let annotation = self.cx.irql_annotation(def_id.into());
                    self.cx
                        .sql_store::<crate::irql::annotation::irql_annotation>(
                            def_id.into(),
                            annotation,
                        );
                },
                adt_callback: |def_id: LocalDefId| {
                    let annotation = self.cx.drop_irql_annotation(def_id.into());
                    self.cx
                        .sql_store::<crate::irql::annotation::drop_irql_annotation>(
                            def_id.into(),
                            annotation,
                        );
                },
            });
    }

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

        let Some(raise) = function_irql.raise else {
            return;
        };
        let Some(IrqlRequirement::Permanent(requirement)) = caller_irql.requirement else {
            return;
        };

        if let Some(high) = requirement.high {
            if raise < requirement.low || raise > high {
                println!("Function which raises IRQL to {} called from function with permanent requirement in interval {} to {}", raise.value, requirement.low.value, high.value);
            }
        } else if raise != requirement.low {
            println!("Function which raises IRQL to {} called from function with permanent requirement {}", raise.value, requirement.low.value);
        }
    }

    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: rustc_hir::intravisit::FnKind<'tcx>,
        _: &'tcx rustc_hir::FnDecl<'tcx>,
        _body: &'tcx rustc_hir::Body<'tcx>,
        _span: rustc_span::Span,
        def_id: LocalDefId,
    ) {
        if crate::util::fn_has_unsatisfiable_preds(cx, def_id.to_def_id()) {
            return;
        }

        let identity = cx
            .tcx
            .erase_regions(GenericArgs::identity_for_item(cx.tcx, def_id));
        let instance = Instance::new(def_id.to_def_id(), identity);
        let env = TypingEnv::post_analysis(*self.cx, def_id);
        let body = cx.tcx.optimized_mir(instance.def_id());

        self.cx.check_irql(env, instance, body);
    }
}
