use rustc_errors::ErrorGuaranteed;

use crate::irql::IrqlValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encodable, Decodable)]
pub enum Error {
    TooGeneric,
    IrqlImpossibleJoin,
    IrqlStackUnderflow {
        span: rustc_span::Span,
    },
    IrqlLoweringMismatch {
        expected: IrqlValue,
        actual: IrqlValue,
        span: rustc_span::Span,
    },
    Error(ErrorGuaranteed),
}
