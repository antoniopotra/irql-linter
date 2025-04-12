pub mod annotation;
pub mod on_call_requirement;
pub mod on_return_value;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encodable, Decodable, Clone, Copy)]
pub struct IrqlValue {
    pub value: u32,
}

impl IrqlValue {
    fn minimum() -> IrqlValue {
        IrqlValue { value: 0 }
    }

    fn maximum() -> IrqlValue {
        IrqlValue { value: 31 }
    }
}

#[derive(Debug, Encodable, Decodable, Clone, Copy)]
pub struct IrqlRange {
    pub low: IrqlValue,
    pub high: Option<IrqlValue>,
}

impl IrqlRange {
    pub fn full() -> IrqlRange {
        IrqlRange {
            low: IrqlValue::minimum(),
            high: Some(IrqlValue::maximum()),
        }
    }
}
