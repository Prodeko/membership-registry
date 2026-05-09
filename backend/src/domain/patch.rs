/// A three-valued patch over an optional field. Distinguishes "leave the
/// existing value alone" from "set to nothing" — both of which the standard
/// `Option<T>` collapses into `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Patch<T> {
    /// Field was omitted; preserve the existing value.
    Leave,
    /// Field was supplied with a value; replace.
    Set(T),
    /// Field was supplied as null; clear the existing value.
    Clear,
}

impl<T> Patch<T> {
    /// Apply this patch to a current `Option<T>`, returning the new value.
    pub fn apply(self, current: Option<T>) -> Option<T> {
        match self {
            Self::Leave => current,
            Self::Set(v) => Some(v),
            Self::Clear => None,
        }
    }
}

impl<T> Default for Patch<T> {
    fn default() -> Self {
        Self::Leave
    }
}
