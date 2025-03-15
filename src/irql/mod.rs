pub mod annotation;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encodable, Decodable, Clone, Copy)]
pub struct IrqlValue {
    pub value: u32,
}

#[derive(Debug, Encodable, Decodable, Clone, Copy)]
pub struct IrqlRange {
    pub low: IrqlValue,
    pub high: Option<IrqlValue>,
}
