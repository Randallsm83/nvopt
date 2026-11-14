//! Harness root. `src/gen/` does not exist in the skill: `scripts/check.sh` fills it with
//! the real nvopt `models.rs`, `nvops.rs` and `rtss.rs` (carriage returns removed, nothing
//! else changed) and appends one `mod props;` line to `nvops.rs` and `rtss.rs`, so the
//! property tests in `props/` run as child modules and reach the private functions.
#![allow(dead_code, unused_imports, unused_variables)]

#[path = "gen/models.rs"]
mod models;
#[path = "gen/nvops.rs"]
mod nvops;
#[path = "gen/rtss.rs"]
mod rtss;
