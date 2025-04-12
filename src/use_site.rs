use rustc_middle::ty::{Instance, PseudoCanonicalInput};
use rustc_span::Span;

#[derive(Debug)]
pub struct UseSite<'tcx> {
    pub instance: PseudoCanonicalInput<'tcx, Instance<'tcx>>,
    pub kind: UseSiteKind,
}

#[derive(Debug)]
pub enum UseSiteKind {
    Call(Span),
    Drop {
        /// Span that causes the drop.
        drop_span: Span,
        /// Span of the place being dropped.
        place_span: Span,
    },
    PointerCoercion(Span),
    Vtable(Span),
}
