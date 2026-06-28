//! `cap`: the tool's **policy**: where it goes, when it stops.
//!
//! Everything that decides lives here, isolated from the loop and **unit-tested**:
//! - [`vigie`]: watches for sterile loops (pure controller, no side effects).
//! - [`boussole`]: the only continuation function (`cap()`).
//! - [`jauge`]: the context budget in real tokens (coming with the engine).
//! - [`reine`]: the outer supervisor that judges results and decides revisions.

pub mod boussole;
pub mod jauge;
pub mod reine;
pub mod vigie;
