use crate::irql::{IrqlRange, IrqlValue};
use rustc_errors::ErrorGuaranteed;
use rustc_span::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encodable, Decodable)]
pub enum Error {
    TooGeneric,
    IrqlImpossibleJoin {
        left_value: IrqlValue,
        right_value: IrqlValue,
        left_span: Span,
        right_span: Span,
    },
    IrqlStackUnderflow {
        span: Span,
    },
    IrqlLoweringMismatch {
        expected: IrqlValue,
        actual: IrqlValue,
        span: Span,
    },
    IrqlOutsideOfPermanentRequirement {
        change_to: IrqlValue,
        permanent_range: IrqlRange,
        span: Span,
    },
    Guaranteed(ErrorGuaranteed),
}
