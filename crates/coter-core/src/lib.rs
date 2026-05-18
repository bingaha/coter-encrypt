//! Core business logic for Coter Encrypt.
//!
//! This crate must stay independent from Tauri and desktop shell APIs. The
//! Tauri application should call into this crate for reusable business logic,
//! while UI, windows, dialogs, and permissions remain in `src-tauri`.

pub mod crypto;
pub mod executor;
pub mod expression;
pub mod removed_module;
pub mod project_store;
pub mod schema;
pub mod validation;
