//! Ported from: packages/core/src/global.ts (sebelumnya `src/global/index.ts`
//! di MASTER_PLAN; lokasi asli pindah ke package core).

pub mod flag;
pub mod global;

pub use global::{ensure_dirs, make, path, reset_for_tests, Interface, InterfaceOverride, Paths};
