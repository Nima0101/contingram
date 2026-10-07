use crate::{
    certificate::encoded_size, limits::validate_limits, verify, Branch, Certificate, CheckLimits,
    Error, Limit, Model, Node, Obstruction, Reason, Report, SearchLimits, Stats, Status,
    MAX_REPORT_BYTES,
};
use std::collections::{BTreeMap, BTreeSet};
type Key = (Vec<usize>, u32);
struct Search<'a> {
    model: &'a Model,
    limits: SearchLimits,
    stats: Stats,
    memo: BTreeMap<Key, (bool, usize)>,
    nodes: Vec<Node>,
    bytes: usize,
}
impl Search<'_> {
    fn work(&mut self) -> Result<(), Limit> {
        if self.stats.work >= self.limits.max_work {
            return Err(Limit {
                stage: "search_work".into(),
                cap: self.limits.max_work,
                consumed: self.stats.work,
            });
        }
        self.stats.work += 1;
        Ok(())
    }
    fn visit(&mut self, belief: Vec<usize>, h: u32) -> Result<(bool, usize), Limit> {
        let key = (belief.clone(), h);
        if let Some(answer) = self.memo.get(&key) {
            return Ok(*answer);
        }
        if self.stats.nodes >= self.limits.max_nodes {
            return Err(Limit {
                stage: "search_nodes".into(),
                cap: self.limits.max_nodes as u64,
                consumed: self.stats.nodes as u64,
            });
        }
        self.stats.nodes += 1;
        let mut safe = true;
        let mut goal = true;
        for s in &belief {
            self.work()?;
            safe &= self.model.ir.safe.binary_search(s).is_ok();
            goal &= self.model.ir.goal.binary_search(s).is_ok();
        }
        let (won, node) = if !safe {
            (false, Node::Unsafe {})
        } else if goal {
            (true, Node::Goal {})
        } else if h == 0 {
            (false, Node::Horizon {})
        } else {
            self.actions(&belief, h)?
        };
        let size = encoded_size(&node).unwrap_or(MAX_REPORT_BYTES + 1);
        if size > MAX_REPORT_BYTES - self.bytes {
            return Err(Limit {
                stage: "evidence_bytes".into(),
                cap: MAX_REPORT_BYTES as u64,
                consumed: self.bytes as u64,
            });
        }
        self.bytes += size;
        let id = self.nodes.len();
        self.nodes.push(node);
        self.memo.insert(key, (won, id));
        Ok((won, id))
    }
    fn actions(&mut self, belief: &[usize], h: u32) -> Result<(bool, Node), Limit> {
        let mut obstructions = vec![];
        for index in 0..self.model.ir.actions.len() {
            self.work()?;
            let action = &self.model.ir.actions[index];
            let mut enabled = BTreeSet::new();
            let mut unsafe_effect = false;
            let mut post: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
            for edge in &action.edges {
                self.work()?;
                if belief.binary_search(&edge.from).is_ok() {
                    enabled.insert(edge.from);
                    unsafe_effect |= self.model.ir.safe.binary_search(&edge.to).is_err();
                    post.entry(edge.observation.clone())
                        .or_default()
                        .insert(edge.to);
                }
            }
            let id = action.id.clone();
            let reason = if enabled.len() != belief.len() {
                Some(Reason::Disabled {})
            } else if unsafe_effect {
                Some(Reason::Unsafe {})
            } else {
                let mut branches = vec![];
                let mut failure = None;
                for (observation, posterior) in post {
                    self.work()?;
                    let (won, next) = self.visit(posterior.into_iter().collect(), h - 1)?;
                    if !won {
                        failure = Some(Reason::Branch { observation, next });
                        break;
                    }
                    branches.push(Branch { observation, next });
                }
                if failure.is_none() {
                    return Ok((
                        true,
                        Node::Choose {
                            action: id,
                            branches,
                        },
                    ));
                }
                failure
            };
            obstructions.push(Obstruction {
                action: id,
                reason: reason.expect("losing action"),
            });
        }
        Ok((
            false,
            Node::Refute {
                actions: obstructions,
            },
        ))
    }
}
fn extract(nodes: &[Node], root: usize) -> Certificate {
    let mut used = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(n) = pending.pop() {
        if used.insert(n) {
            pending.extend(nodes[n].children());
        }
    }
    let mapping: BTreeMap<_, _> = used
        .iter()
        .enumerate()
        .map(|(new, old)| (*old, new))
        .collect();
    let mut result = vec![];
    for old in used {
        let mut node = nodes[old].clone();
        match &mut node {
            Node::Choose { branches, .. } => {
                for b in branches {
                    b.next = mapping[&b.next];
                }
            }
            Node::Refute { actions } => {
                for a in actions {
                    if let Reason::Branch { next, .. } = &mut a.reason {
                        *next = mapping[next];
                    }
                }
            }
            _ => {}
        }
        result.push(node);
    }
    Certificate {
        root: mapping[&root],
        nodes: result,
    }
}
/// Deterministic strong bounded synthesis, with independently checked evidence.
pub fn analyze(model: &Model, limits: SearchLimits) -> Result<Report, Error> {
    analyze_checked(model, limits, CheckLimits::default())
}

// Explicit budgets here let tests exercise mandatory-check exhaustion. The
// public API always uses the fixed hard checker ceilings.
fn analyze_checked(
    model: &Model,
    limits: SearchLimits,
    check_limits: CheckLimits,
) -> Result<Report, Error> {
    validate_limits(limits.max_nodes, limits.max_work)?;
    if limits.horizon > 32 {
        return Err(Error::new("invalid_limits", "horizon exceeds 32"));
    }
    let mut search = Search {
        model,
        limits,
        stats: Stats::default(),
        memo: BTreeMap::new(),
        nodes: vec![],
        bytes: 0,
    };
    let outcome = search.visit(model.ir.initial.clone(), limits.horizon);
    let mut report = Report {
        schema_version: 1,
        model_sha256: model.digest().into(),
        horizon: limits.horizon,
        status: Status::Unknown,
        stats: search.stats.clone(),
        certificate: None,
        limit: None,
    };
    match outcome {
        Err(limit) => report.limit = Some(limit),
        Ok((won, root)) => {
            report.status = if won {
                Status::PolicyFound
            } else {
                Status::NoPolicyWithinHorizon
            };
            report.certificate = Some(extract(&search.nodes, root));
            if let Err(consumed) = crate::certificate::counted_size(&report) {
                report.status = Status::Unknown;
                report.certificate = None;
                report.limit = Some(Limit {
                    stage: "output_bytes".into(),
                    cap: MAX_REPORT_BYTES as u64,
                    consumed: consumed as u64,
                });
            } else if let Err(e) = verify(model, &report, check_limits) {
                if e.code != "verification_limit" {
                    return Err(Error::new(
                        "internal_check",
                        "synthesized certificate rejected",
                    ));
                }
                report.status = Status::Unknown;
                report.certificate = None;
                let cap = if e.message == "checker contexts exhausted" {
                    check_limits.max_nodes as u64
                } else {
                    check_limits.max_work
                };
                report.limit = Some(Limit {
                    stage: "verification_limit".into(),
                    cap,
                    consumed: cap,
                });
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mandatory_check_exhaustion_becomes_unknown_without_evidence() {
        let model = crate::compile(
            &crate::parse_contract(include_bytes!("../examples/contracts/receipt.json")).unwrap(),
        )
        .unwrap();
        for check in [
            CheckLimits {
                max_nodes: 0,
                max_work: 10_000_000,
            },
            CheckLimits {
                max_nodes: 50_000,
                max_work: 0,
            },
        ] {
            let report = analyze_checked(
                &model,
                SearchLimits {
                    horizon: 2,
                    ..Default::default()
                },
                check,
            )
            .unwrap();
            assert_eq!(report.status, Status::Unknown);
            assert!(report.certificate.is_none());
            let limit = report.limit.unwrap();
            assert_eq!(limit.stage, "verification_limit");
            assert_eq!(limit.cap, 0);
            assert_eq!(limit.consumed, 0);
        }
    }
}
