use rustc_errors::ErrorGuaranteed;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encodable, Decodable)]
pub enum Error {
    TooGeneric,
    Error(ErrorGuaranteed),
}
