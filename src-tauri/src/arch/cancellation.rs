use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// CancellationToken
// ---------------------------------------------------------------------------

/// A token that can be triggered to signal cancellation to one or more
/// consumers.  Cloning shares the same underlying signal.
#[derive(Clone, Debug)]
pub struct CancellationToken {
    inner: Arc<AtomicBool>,
}

impl CancellationToken {
    /// Create a new token in the non-cancelled state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Signal cancellation.  All consumers checking this token (or its
    /// clones) will observe `is_cancelled() == true`.
    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    /// Returns `true` when cancellation has been signalled.
    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    /// Create a child token that shares the same underlying signal.
    /// Cancelling the parent cancels all children.
    pub fn child(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }

    /// Reset the token to the non-cancelled state.
    /// ⚠  Use with care — all clones are affected.
    pub fn reset(&self) {
        self.inner.store(false, Ordering::SeqCst);
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// CancellationScope
// ---------------------------------------------------------------------------

/// Manages a group of [`CancellationToken`]s that are all cancelled when the
/// scope is dropped (RAII-style).  Useful for tying cancellation to a lexical
/// scope or a request lifecycle.
pub struct CancellationScope {
    root: CancellationToken,
    children: Vec<CancellationToken>,
}

impl CancellationScope {
    /// Create a new empty scope with a fresh root token.
    pub fn new() -> Self {
        Self {
            root: CancellationToken::new(),
            children: Vec::new(),
        }
    }

    /// Return the root token for this scope.
    pub fn token(&self) -> CancellationToken {
        self.root.clone()
    }

    /// Spawn a child token.  When the scope is cancelled (or dropped), all
    /// children are cancelled too.
    pub fn spawn_child(&mut self) -> CancellationToken {
        let child = self.root.child();
        self.children.push(child.clone());
        child
    }

    /// Cancel the root token and all children.
    pub fn cancel_all(&self) {
        self.root.cancel();
        for child in &self.children {
            child.cancel();
        }
    }

    /// Check whether the scope has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.root.is_cancelled()
    }
}

impl Default for CancellationScope {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for CancellationScope {
    fn drop(&mut self) {
        self.cancel_all();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_not_cancelled_by_default() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn test_token_cancel() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn test_token_child_shares_signal() {
        let parent = CancellationToken::new();
        let child = parent.child();
        parent.cancel();
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_scope_cancels_on_drop() {
        let child;
        {
            let mut scope = CancellationScope::new();
            child = scope.spawn_child();
            assert!(!child.is_cancelled());
        }
        assert!(child.is_cancelled());
    }

    #[test]
    fn test_scope_cancel_all() {
        let mut scope = CancellationScope::new();
        let c1 = scope.spawn_child();
        let c2 = scope.spawn_child();
        scope.cancel_all();
        assert!(c1.is_cancelled());
        assert!(c2.is_cancelled());
    }
}
