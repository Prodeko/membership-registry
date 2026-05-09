/// A three-valued patch over an optional field. Distinguishes "leave the
/// existing value alone" from "set to nothing" — both of which the standard
/// `Option<T>` collapses into `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Patch<T> {
    /// Field was omitted; preserve the existing value.
    #[default]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leave_preserves_some() {
        assert_eq!(Patch::<i32>::Leave.apply(Some(42)), Some(42));
    }

    #[test]
    fn leave_preserves_none() {
        assert_eq!(Patch::<i32>::Leave.apply(None), None);
    }

    #[test]
    fn set_replaces_some() {
        assert_eq!(Patch::Set(7).apply(Some(42)), Some(7));
    }

    #[test]
    fn set_replaces_none() {
        assert_eq!(Patch::Set(7).apply(None), Some(7));
    }

    #[test]
    fn clear_drops_some() {
        assert_eq!(Patch::<i32>::Clear.apply(Some(42)), None);
    }

    #[test]
    fn clear_drops_none() {
        assert_eq!(Patch::<i32>::Clear.apply(None), None);
    }

    #[test]
    fn default_is_leave() {
        let p: Patch<i32> = Patch::default();
        assert_eq!(p.apply(Some(42)), Some(42));
    }
}
