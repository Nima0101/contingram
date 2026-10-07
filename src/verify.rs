use crate::{
    certificate::encoded_size, limits::validate_limits, Certificate, CheckLimits, Error, Model,
    Node, Reason, Report, Stats, Status,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize)]
pub struct Verified {
    pub status: Status,
    pub stats: Stats,
}
fn invalid() -> Error {
    Error::new(
        "invalid_certificate",
        "evidence does not establish the claimed bounded result",
    )
}
type Relation = (bool, bool, BTreeMap<String, Vec<usize>>);

struct Checker<'a> {
    model: &'a Model,
    cert: &'a Certificate,
    limits: CheckLimits,
    stats: Stats,
    contexts: BTreeSet<(usize, Vec<usize>, u32, bool)>,
    reached: BTreeSet<usize>,
}
impl Checker<'_> {
    fn charge(&mut self) -> Result<(), Error> {
        if self.stats.work >= self.limits.max_work {
            return Err(Error::new("verification_limit", "checker work exhausted"));
        }
        self.stats.work += 1;
        Ok(())
    }
    // This traversal deliberately does not use search's outcome grouping or memo table.
    fn relation(&mut self, action: &str, belief: &[usize]) -> Result<Relation, Error> {
        self.charge()?;
        let a = self
            .model
            .ir
            .actions
            .iter()
            .find(|a| a.id == action)
            .ok_or_else(invalid)?;
        let mut destinations: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut available = vec![false; self.model.ir.states.len()];
        let mut safe = true;
        for edge in &a.edges {
            self.charge()?;
            if belief.contains(&edge.from) {
                available[edge.from] = true;
                safe &= self.model.ir.safe.contains(&edge.to);
                destinations
                    .entry(edge.observation.clone())
                    .or_default()
                    .push(edge.to);
            }
        }
        for states in destinations.values_mut() {
            states.sort_unstable();
            states.dedup();
        }
        Ok((belief.iter().all(|s| available[*s]), safe, destinations))
    }
    fn visit(
        &mut self,
        id: usize,
        belief: Vec<usize>,
        h: u32,
        positive: bool,
    ) -> Result<(), Error> {
        let key = (id, belief.clone(), h, positive);
        if self.contexts.contains(&key) {
            return Ok(());
        }
        if self.stats.nodes >= self.limits.max_nodes {
            return Err(Error::new(
                "verification_limit",
                "checker contexts exhausted",
            ));
        }
        self.stats.nodes += 1;
        self.reached.insert(id);
        if belief.is_empty() {
            return Err(invalid());
        }
        let mut safe = true;
        let mut goal = true;
        for s in &belief {
            self.charge()?;
            safe &= self.model.ir.safe.contains(s);
            goal &= self.model.ir.goal.contains(s);
        }
        match &self.cert.nodes[id] {
            Node::Goal {} if positive && safe && goal => {}
            Node::Unsafe {} if !positive && !safe => {}
            Node::Horizon {} if !positive && safe && !goal && h == 0 => {}
            Node::Choose { action, branches } if positive && safe && h > 0 => {
                let (enabled, effects_safe, mut post) = self.relation(action, &belief)?;
                if !enabled || !effects_safe || post.is_empty() || branches.len() != post.len() {
                    return Err(invalid());
                }
                for branch in branches {
                    self.charge()?;
                    let child = post.remove(&branch.observation).ok_or_else(invalid)?;
                    self.visit(branch.next, child, h - 1, true)?;
                }
            }
            Node::Refute { actions } if !positive && safe && !goal && h > 0 => {
                if actions.len() != self.model.ir.actions.len() {
                    return Err(invalid());
                }
                let mut seen = BTreeSet::new();
                for obstruction in actions {
                    self.charge()?;
                    if !seen.insert(&obstruction.action) {
                        return Err(invalid());
                    }
                    let (enabled, effects_safe, mut post) =
                        self.relation(&obstruction.action, &belief)?;
                    match &obstruction.reason {
                        Reason::Disabled {} if !enabled => {}
                        Reason::Unsafe {} if !effects_safe => {}
                        Reason::Branch { observation, next } => {
                            let child = post.remove(observation).ok_or_else(invalid)?;
                            self.visit(*next, child, h - 1, false)?;
                        }
                        _ => return Err(invalid()),
                    }
                }
            }
            _ => return Err(invalid()),
        }
        self.contexts.insert(key);
        Ok(())
    }
}
/// Check both polarities independently from the model's initial belief.
/// A digest binds semantics; it does not authenticate the contract author.
pub fn verify(model: &Model, report: &Report, limits: CheckLimits) -> Result<Verified, Error> {
    validate_limits(limits.max_nodes, limits.max_work)?;
    if report.schema_version != 1 || report.horizon > 32 {
        return Err(Error::new(
            "unsupported_version",
            "invalid report version or horizon",
        ));
    }
    if report.model_sha256 != model.digest() {
        return Err(Error::new(
            "model_mismatch",
            "report is bound to another model",
        ));
    }
    if report.status == Status::Unknown || report.certificate.is_none() {
        return Err(Error::new(
            "no_certificate",
            "complete evidence is required",
        ));
    }
    if report.limit.is_some() {
        return Err(invalid());
    }
    let cert = report.certificate.as_ref().ok_or_else(invalid)?;
    if cert.nodes.is_empty() || cert.nodes.len() > 50_000 || cert.root != cert.nodes.len() - 1 {
        return Err(invalid());
    }
    let mut preflight_work = 0u64;
    let mut charge = || {
        if preflight_work >= limits.max_work {
            return Err(Error::new("verification_limit", "checker work exhausted"));
        }
        preflight_work += 1;
        Ok(())
    };
    // Bound direct Rust values as well as parsed reports, before semantic traversal.
    for (id, node) in cert.nodes.iter().enumerate() {
        charge()?;
        match node {
            Node::Choose { action, branches } => {
                crate::label(action)?;
                if branches.len() > 512 {
                    return Err(invalid());
                }
                for b in branches {
                    charge()?;
                    crate::label(&b.observation)?;
                    if b.next >= id {
                        return Err(invalid());
                    }
                }
            }
            Node::Refute { actions } => {
                if actions.len() > 32 {
                    return Err(invalid());
                }
                for a in actions {
                    charge()?;
                    crate::label(&a.action)?;
                    if let Reason::Branch { observation, next } = &a.reason {
                        crate::label(observation)?;
                        if *next >= id {
                            return Err(invalid());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    encoded_size(report)?;
    let mut checker = Checker {
        model,
        cert,
        limits,
        stats: Stats {
            nodes: 0,
            work: preflight_work,
        },
        contexts: BTreeSet::new(),
        reached: BTreeSet::new(),
    };
    checker.visit(
        cert.root,
        model.ir.initial.clone(),
        report.horizon,
        report.status == Status::PolicyFound,
    )?;
    if checker.reached.len() != cert.nodes.len() {
        return Err(invalid());
    }
    Ok(Verified {
        status: report.status,
        stats: checker.stats,
    })
}
