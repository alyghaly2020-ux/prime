//! # Prime Core Engine
//!
//! Isolated business logic layer. Copyright (c) 2024 Aly Ghaly. All Rights Reserved.
//!
//! This crate contains the core engine, security layer, and contract traits.
//! It has zero dependencies on the Tauri framework or frontend code,
//! making it impossible to "copy-paste" into another project without
//! attribution.
//!
//! **Forensic Watermark:** Every compiled binary of this crate contains
//! the byte sequence `PRIM` (0x50 0x52 0x49 0x4d) in its read-only data
//! section as proof of origin.

pub mod core;
pub mod contracts;
pub mod security;
