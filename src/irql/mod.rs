pub mod annotation;
pub mod check;
pub mod dataflow;
pub mod utils;

use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Encodable, Decodable, Clone, Copy)]
pub struct IrqlValue {
    pub value: u32,
}

impl Display for IrqlValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
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

    pub fn contains(&self, irql_value: IrqlValue) -> bool {
        if irql_value < self.low {
            return false;
        }

        if let Some(high) = self.high
            && irql_value > high
        {
            return false;
        }

        true
    }
}

impl Display for IrqlRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.high {
            Some(high) => write!(f, "[{}, {}]", self.low, high),
            None => write!(f, "{}", self.low),
        }
    }
}

#[derive(Debug, Clone, Copy, Encodable, Decodable, PartialEq, Eq)]
pub enum IrqlRequirement {
    Call(IrqlRange),
    Permanent(IrqlRange),
}

impl IrqlRequirement {
    pub fn unbounded() -> IrqlRequirement {
        IrqlRequirement::Call(IrqlRange::unbounded())
    }

    pub fn range(&self) -> IrqlRange {
        match self {
            IrqlRequirement::Call(irql_range) => *irql_range,
            IrqlRequirement::Permanent(irql_range) => *irql_range,
        }
    }
}
