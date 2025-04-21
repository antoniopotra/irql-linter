pub mod annotation;
pub mod dataflow;
pub mod raise;
pub mod requirement;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encodable, Decodable, Clone, Copy)]
pub struct IrqlValue {
    pub value: u32,
}

impl IrqlValue {
    fn passive_level() -> IrqlValue {
        IrqlValue { value: 0 }
    }

    fn apc_level() -> IrqlValue {
        IrqlValue { value: 1 }
    }

    fn dispatch_level() -> IrqlValue {
        IrqlValue { value: 2 }
    }

    fn high_level() -> IrqlValue {
        IrqlValue { value: 31 }
    }
}

#[derive(Debug, Encodable, Decodable, Clone, Copy, PartialEq, Eq)]
pub struct IrqlRange {
    pub low: IrqlValue,
    pub high: Option<IrqlValue>,
}

impl IrqlRange {
    pub fn unbounded() -> IrqlRange {
        IrqlRange {
            low: IrqlValue::passive_level(),
            high: Some(IrqlValue::high_level()),
        }
    }
}

#[derive(Debug, Clone, Copy, Encodable, Decodable)]
pub enum IrqlRequirement {
    Call(IrqlRange),
    Permanent(IrqlRange),
}
