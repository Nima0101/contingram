use crate::Error;
#[derive(Debug, Clone, Copy)]
pub struct SearchLimits {
    pub horizon: u32,
    pub max_nodes: usize,
    pub max_work: u64,
}
impl Default for SearchLimits {
    fn default() -> Self {
        Self {
            horizon: 8,
            max_nodes: 10_000,
            max_work: 1_000_000,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct CheckLimits {
    pub max_nodes: usize,
    pub max_work: u64,
}
impl Default for CheckLimits {
    fn default() -> Self {
        Self {
            max_nodes: 50_000,
            max_work: 10_000_000,
        }
    }
}
pub(crate) fn validate_limits(nodes: usize, work: u64) -> Result<(), Error> {
    if nodes > 50_000 || work > 10_000_000 {
        return Err(Error::new(
            "invalid_limits",
            "hard ceilings: 50000 nodes and 10000000 work",
        ));
    }
    Ok(())
}
