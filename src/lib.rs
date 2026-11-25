//! Offline analysis of GCP billing-export CSV files.
//!
//! The binary `gcp-billing-top` is a thin wrapper around this library.

pub mod aggregate;
pub mod cli;
pub mod error;
pub mod ingest;
pub mod json;
pub mod labels;
pub mod money;
pub mod month;
pub mod report;
