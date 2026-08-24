//! Sprint 10b: Interrupt/cancellation token untuk prompt loop.
//! Ported dari session/prompt.ts abort handling.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Cancellation token — set flag dari thread lain untuk menghentikan loop.
#[derive(Clone)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        CancellationToken {
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}
