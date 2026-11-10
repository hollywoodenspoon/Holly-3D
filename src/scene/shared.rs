//! Copy-on-write shared data for undo snapshots: cloning a `Shared<T>` only
//! bumps an `Arc` count, and the first write after a clone copies the data
//! (`Arc::make_mut`). Undo checkpoints therefore cost one pointer per mesh
//! that the edit does not touch.

use std::ops::{Deref, DerefMut};
use std::sync::Arc;

#[derive(Default)]
pub struct Shared<T>(Arc<T>);

impl<T> Shared<T> {
    pub fn new(v: T) -> Self {
        Self(Arc::new(v))
    }

    /// True when both handles point at the same allocation (no copy yet).
    pub fn ptr_eq(a: &Self, b: &Self) -> bool {
        Arc::ptr_eq(&a.0, &b.0)
    }
}

impl<T> Clone for Shared<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> From<T> for Shared<T> {
    fn from(v: T) -> Self {
        Self::new(v)
    }
}

impl<T> Deref for Shared<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: Clone> DerefMut for Shared<T> {
    fn deref_mut(&mut self) -> &mut T {
        Arc::make_mut(&mut self.0)
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Shared<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Shared<T> {
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.0, &o.0) || *self.0 == *o.0
    }
}

impl<T: serde::Serialize> serde::Serialize for Shared<T> {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Shared<T> {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        T::deserialize(d).map(Self::new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_share_until_written() {
        let a = Shared::new(vec![1, 2, 3]);
        let mut b = a.clone();
        assert!(Shared::ptr_eq(&a, &b));
        b.push(4);
        assert!(!Shared::ptr_eq(&a, &b));
        assert_eq!((a.len(), b.len()), (3, 4));
        assert_eq!(a, Shared::new(vec![1, 2, 3]));
    }
}
