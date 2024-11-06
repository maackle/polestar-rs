use std::convert::Infallible;

pub trait IsTerminal {
    fn is_terminal(&self) -> bool;
}

impl<T> IsTerminal for Option<T> {
    fn is_terminal(&self) -> bool {
        self.is_none()
    }
}

impl IsTerminal for Infallible {
    fn is_terminal(&self) -> bool {
        false
    }
}

#[derive(
    Debug,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    derive_more::From,
    derive_more::Deref,
    derive_more::DerefMut,
)]
pub struct NoTerminal<T>(T);

impl<T> IsTerminal for NoTerminal<T> {
    fn is_terminal(&self) -> bool {
        false
    }
}
