//! Snapshot-based Undo / Redo (Ctrl+Z / Ctrl+Y).

#[derive(Clone, Debug)]
pub struct History<T> {
    undo: Vec<(String, T)>,
    redo: Vec<(String, T)>,
    limit: usize,
    /// Checkpoints ever pushed (autosave compares it to spot new edits).
    pushes: u64,
}

impl<T> Default for History<T> {
    fn default() -> Self {
        Self { undo: vec![], redo: vec![], limit: 64, pushes: 0 }
    }
}

impl<T> History<T> {
    pub fn with_limit(limit: usize) -> Self {
        Self { limit: limit.max(1), ..Self::default() }
    }

    /// Preferences -> General -> Undo steps; drops the oldest steps if needed.
    pub fn set_limit(&mut self, limit: usize) {
        self.limit = limit.max(1);
        let extra = self.undo.len().saturating_sub(self.limit);
        self.undo.drain(..extra);
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn pushes(&self) -> u64 {
        self.pushes
    }

    /// Records the state *before* an edit.
    pub fn push(&mut self, label: impl Into<String>, before: T) {
        self.undo.push((label.into(), before));
        self.pushes += 1;
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Drops the most recent checkpoint (used when an edit is cancelled).
    pub fn discard_last(&mut self) -> Option<T> {
        self.undo.pop().map(|x| x.1)
    }

    pub fn undo(&mut self, current: T) -> Result<(String, T), T> {
        match self.undo.pop() {
            Some((label, prev)) => {
                self.redo.push((label.clone(), current));
                Ok((label, prev))
            }
            None => Err(current),
        }
    }

    pub fn redo(&mut self, current: T) -> Result<(String, T), T> {
        match self.redo.pop() {
            Some((label, next)) => {
                self.undo.push((label.clone(), current));
                Ok((label, next))
            }
            None => Err(current),
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_redo_roundtrip() {
        let mut h = History::default();
        let mut state = 1;
        h.push("a", state);
        state = 2;
        h.push("b", state);
        state = 3;
        let (_, s) = h.undo(state).unwrap();
        assert_eq!(s, 2);
        let (_, s) = h.undo(s).unwrap();
        assert_eq!(s, 1);
        assert!(h.undo(s).is_err());
        let (_, s) = h.redo(1).unwrap();
        assert_eq!(s, 2);
        h.push("c", s);
        assert!(!h.can_redo());
    }
}
