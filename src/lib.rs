//! Finite tool recovery contracts and independently checked policies.
//!
//! ```
//! use contingram::{analyze, compile, parse_contract, verify, CheckLimits, SearchLimits};
//! # fn main() -> Result<(), contingram::Error> {
//! let input = br#"{
//!   "schema_version":1,"name":"finished",
//!   "variables":[{"name":"effect","values":["done"]}],
//!   "initial":{"op":"true"},"safe":{"op":"true"},
//!   "goal":{"op":"true"},"actions":[]
//! }"#;
//! let model = compile(&parse_contract(input)?)?;
//! let report = analyze(&model, SearchLimits::default())?;
//! verify(&model, &report, CheckLimits::default())?;
//! # Ok(()) }
//! ```
#![forbid(unsafe_code)]

mod certificate;
mod contract;
mod follow;
mod limits;
mod model;
mod solve;
mod verify;

pub use certificate::*;
pub use contract::{parse_contract, Contract};
pub use follow::{follow, Decision};
pub use limits::{CheckLimits, SearchLimits};
pub use model::{compile, Action, Edge, Ir, Model, Origin, Variable};
pub use solve::analyze;
pub use verify::{verify, Verified};

/// Maximum accepted contract bytes.
pub const MAX_CONTRACT_BYTES: usize = 1024 * 1024;
/// Maximum accepted or emitted report bytes.
pub const MAX_REPORT_BYTES: usize = 8 * 1024 * 1024;

/// A stable error category and a bounded, payload-free diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}

impl Error {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}

pub(crate) fn label(s: &str) -> Result<(), Error> {
    if s.is_empty()
        || s.len() > 64
        || !s
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err(Error::new(
            "invalid_label",
            "labels require 1..64 ASCII letters, digits, underscores or hyphens",
        ));
    }
    Ok(())
}

pub(crate) fn json_error(e: serde_json::Error) -> Error {
    Error::new(
        "invalid_json",
        format!(
            "invalid JSON structure at line {}, column {}",
            e.line(),
            e.column()
        ),
    )
}

pub mod telemetry;
