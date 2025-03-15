#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct IrqlValue {
    pub value: u32,
}

#[derive(Debug)]
pub struct IrqlRange {
    pub low: IrqlValue,
    pub high: Option<IrqlValue>,
}
