pub mod annotation;
pub mod on_call_requirement;
pub mod on_return_value;

use rustc_errors::ErrorGuaranteed;
use rustc_middle::ty::PseudoCanonicalInput;

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

#[derive(Clone, Encodable, Decodable)]
pub enum Error {
    TooGeneric,
    Error(ErrorGuaranteed),
}

pub struct PolyDisplay<'a, 'tcx, T>(pub &'a PseudoCanonicalInput<'tcx, T>);

impl<T> std::fmt::Display for PolyDisplay<'_, '_, T>
where
    T: std::fmt::Display + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PseudoCanonicalInput { typing_env, value } = self.0;
        write!(f, "{}", value)?;
        if !typing_env.param_env.caller_bounds().is_empty() {
            write!(f, " where ")?;
            for (i, predicate) in typing_env.param_env.caller_bounds().iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", predicate)?;
            }
        }
        Ok(())
    }
}
