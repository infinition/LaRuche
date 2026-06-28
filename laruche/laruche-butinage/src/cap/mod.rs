//! `cap`: the tool's **policy**: where it goes, when it stops.
//!
//! Everything that decides lives here, isolated from the loop and **unit-tested**:
//! - [`vigie`]: watches for sterile loops (pure controller, no side effects).
//! - [`boussole`]: the only continuation function (`cap()`).
//! - [`jauge`]: the context budget in real tokens (coming with the engine).

pub mod boussole;
pub mod jauge;
pub mod vigie;
