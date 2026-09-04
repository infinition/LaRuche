//! Installable LaRuche Apps.
//!
//! This first vertical slice deliberately stops at a safe persistent registry:
//! manifests are strict, packages are discovered only below a fixed data root and
//! enable/disable state survives restarts. Asset serving, MCP Apps rendering and
//! backend supervision build on these invariants instead of inventing their own
//! package lookup later.

pub(crate) mod api;
pub(crate) mod assets;
mod installer;
mod model;
pub(crate) mod permissions;
mod registry;
pub(crate) mod storage;

pub(crate) use model::*;
pub(crate) use registry::*;
