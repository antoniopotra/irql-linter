use rustc_lint::LateLintPass;
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_tool_lint! {
    pub klint::IRQL_VIOLATION,
    Deny,
    "Generates an error if Windows IRQL rules are violated"
}

pub struct IrqlViolation;

impl_lint_pass!(IrqlViolation => [IRQL_VIOLATION]);

impl LateLintPass<'_> for IrqlViolation {}
