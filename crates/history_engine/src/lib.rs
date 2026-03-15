#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UndoBudget {
    pub max_megabytes: usize,
}

impl Default for UndoBudget {
    fn default() -> Self {
        Self { max_megabytes: 512 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryStack<T> {
    budget: UndoBudget,
    undo_entries: Vec<T>,
    redo_entries: Vec<T>,
}

impl<T> Default for HistoryStack<T> {
    fn default() -> Self {
        Self {
            budget: UndoBudget::default(),
            undo_entries: Vec::new(),
            redo_entries: Vec::new(),
        }
    }
}

impl<T> HistoryStack<T> {
    pub fn new(budget: UndoBudget) -> Self {
        Self {
            budget,
            undo_entries: Vec::new(),
            redo_entries: Vec::new(),
        }
    }

    pub fn budget(&self) -> UndoBudget {
        self.budget
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_entries.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_entries.is_empty()
    }

    pub fn undo_len(&self) -> usize {
        self.undo_entries.len()
    }

    pub fn redo_len(&self) -> usize {
        self.redo_entries.len()
    }

    pub fn push(&mut self, entry: T) {
        self.undo_entries.push(entry);
        self.redo_entries.clear();
    }

    pub fn undo(&mut self) -> Option<&T> {
        let entry = self.undo_entries.pop()?;
        self.redo_entries.push(entry);
        self.redo_entries.last()
    }

    pub fn redo(&mut self) -> Option<&T> {
        let entry = self.redo_entries.pop()?;
        self.undo_entries.push(entry);
        self.undo_entries.last()
    }

    pub fn current_undo(&self) -> Option<&T> {
        self.undo_entries.last()
    }

    pub fn current_redo(&self) -> Option<&T> {
        self.redo_entries.last()
    }

    pub fn undo_entries(&self) -> &[T] {
        &self.undo_entries
    }

    pub fn redo_entries(&self) -> &[T] {
        &self.redo_entries
    }
}

#[cfg(test)]
mod tests {
    use super::{HistoryStack, UndoBudget};

    #[test]
    fn push_enables_undo_and_clears_redo() {
        let mut history = HistoryStack::new(UndoBudget { max_megabytes: 128 });
        history.push("stroke-1");
        history.push("stroke-2");
        let _ = history.undo();

        assert!(history.can_redo());

        history.push("stroke-3");

        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(history.current_undo(), Some(&"stroke-3"));
    }

    #[test]
    fn undo_moves_entry_to_redo_stack() {
        let mut history = HistoryStack::default();
        history.push("stroke-1");
        history.push("stroke-2");

        let undone = history.undo();

        assert_eq!(undone, Some(&"stroke-2"));
        assert_eq!(history.current_undo(), Some(&"stroke-1"));
        assert_eq!(history.current_redo(), Some(&"stroke-2"));
    }

    #[test]
    fn redo_restores_entry_to_undo_stack() {
        let mut history = HistoryStack::default();
        history.push("stroke-1");
        history.push("stroke-2");
        let _ = history.undo();

        let redone = history.redo();

        assert_eq!(redone, Some(&"stroke-2"));
        assert_eq!(history.current_undo(), Some(&"stroke-2"));
        assert!(!history.can_redo());
    }

    #[test]
    fn history_preserves_budget_configuration() {
        let history = HistoryStack::<&str>::new(UndoBudget { max_megabytes: 64 });

        assert_eq!(history.budget().max_megabytes, 64);
        assert_eq!(history.undo_len(), 0);
        assert_eq!(history.redo_len(), 0);
    }
}
