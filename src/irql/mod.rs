pub mod annotation;
pub mod on_call_requirement;
pub mod on_return_value;
pub mod permanenet_requirement;

use rustc_errors::ErrorGuaranteed;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encodable, Decodable, Clone, Copy)]
pub struct IrqlValue {
    pub value: u32,
}

#[derive(Debug, Encodable, Decodable, Clone, Copy)]
pub struct IrqlRange {
    pub low: IrqlValue,
    pub high: Option<IrqlValue>,
}

pub enum Error {
    TooGeneric,
    Error(ErrorGuaranteed),
}
