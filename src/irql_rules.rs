use rustc_lint::LateLintPass;
use rustc_session::{declare_tool_lint, impl_lint_pass};

declare_tool_lint! {
    pub klint::IRQL_RULES,
    Deny,
    "Generates an error if IRQL rules are not respected"
}

pub struct IrqlRules;

impl_lint_pass!(IrqlRules => [IRQL_RULES]);

impl LateLintPass<'_> for IrqlRules {}
