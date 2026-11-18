//! PolyDB V1: deterministic, dependency-free foundation for a multi-model plugin database.
//! This is a research baseline, not a production-grade transactional database.
pub mod error;
pub mod model;
pub mod manifest;
pub mod process;
pub mod registry;
pub mod store;
pub mod wire;

pub use error::{DbError, Result};
pub use model::{Command, DataModel, Descriptor, Engine, Response};
pub use manifest::PluginManifest;
pub use process::ProcessEngine;
pub use registry::PluginManager;
pub use store::WalEngine;
