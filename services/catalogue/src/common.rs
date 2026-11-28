#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    TooShort,
    TooLong,
    OutOfRange,
    InvalidFormat,
    TooFewItems,
    NotUnique,
}