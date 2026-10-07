use crate::{Error, MAX_REPORT_BYTES};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    PolicyFound,
    NoPolicyWithinHorizon,
    Unknown,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stats {
    pub nodes: usize,
    pub work: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limit {
    pub stage: String,
    pub cap: u64,
    pub consumed: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_version: u32,
    pub model_sha256: String,
    pub horizon: u32,
    pub status: Status,
    pub stats: Stats,
    pub certificate: Option<Certificate>,
    pub limit: Option<Limit>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    pub root: usize,
    pub nodes: Vec<Node>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Node {
    Goal {},
    Unsafe {},
    Horizon {},
    Choose {
        action: String,
        branches: Vec<Branch>,
    },
    Refute {
        actions: Vec<Obstruction>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Branch {
    pub observation: String,
    pub next: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Obstruction {
    pub action: String,
    pub reason: Reason,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Reason {
    Disabled {},
    Unsafe {},
    Branch { observation: String, next: usize },
}

impl Node {
    pub(crate) fn children(&self) -> Vec<usize> {
        match self {
            Self::Choose { branches, .. } => branches.iter().map(|b| b.next).collect(),
            Self::Refute { actions } => actions
                .iter()
                .filter_map(|a| match a.reason {
                    Reason::Branch { next, .. } => Some(next),
                    _ => None,
                })
                .collect(),
            _ => vec![],
        }
    }
}
/// Parse a bounded report. Semantic validity requires `verify` with its model.
pub fn parse_report(bytes: &[u8]) -> Result<Report, Error> {
    if bytes.len() > MAX_REPORT_BYTES {
        return Err(Error::new("input_limit", "report exceeds 8 MiB"));
    }
    serde_json::from_slice(bytes).map_err(crate::json_error)
}
struct Counter(usize);
impl std::io::Write for Counter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MAX_REPORT_BYTES - self.0 {
            return Err(std::io::Error::other("report limit"));
        }
        self.0 += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub(crate) fn counted_size(value: &impl Serialize) -> Result<usize, usize> {
    let mut counter = Counter(0);
    match serde_json::to_writer(&mut counter, value) {
        Ok(()) => Ok(counter.0),
        Err(_) => Err(counter.0),
    }
}
pub(crate) fn encoded_size(value: &impl Serialize) -> Result<usize, Error> {
    counted_size(value).map_err(|_| Error::new("output_limit", "report exceeds 8 MiB"))
}
/// Serialize within the report byte limit before allocating the output buffer.
pub fn report_bytes(report: &Report) -> Result<Vec<u8>, Error> {
    encoded_size(report)?;
    serde_json::to_vec(report).map_err(crate::json_error)
}
