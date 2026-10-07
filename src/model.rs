use crate::{Contract, Error};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// A named finite domain, sorted in normalized models.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Variable {
    pub name: String,
    pub values: Vec<String>,
}
/// One correlated external effect and visible result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub observation: String,
}
/// Normalized action; no edge from a world means disabled there.
#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub id: String,
    pub edges: Vec<Edge>,
}
/// Canonical v1 semantics. Field order is part of its binding format.
#[derive(Debug, Clone, Serialize)]
pub struct Ir {
    pub semantic_version: u32,
    pub variables: Vec<Variable>,
    pub states: Vec<Vec<u16>>,
    pub initial: Vec<usize>,
    pub safe: Vec<usize>,
    pub goal: Vec<usize>,
    pub actions: Vec<Action>,
}
/// Authored origin of a lowered edge; excluded from semantic hash.
#[derive(Debug, Clone, Serialize)]
pub struct Origin {
    pub action: String,
    pub outcome: String,
    pub from: usize,
    pub to: usize,
    pub observation: String,
}
/// Validated, immutable finite model. Only checked lowering constructs one.
#[derive(Debug, Clone)]
pub struct Model {
    pub(crate) ir: Ir,
    digest: String,
    origins: Vec<Origin>,
}
impl Model {
    pub fn ir(&self) -> &Ir {
        &self.ir
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn origins(&self) -> &[Origin] {
        &self.origins
    }
    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.ir).expect("finite IR serializes")
    }
}

/// Lower finite predicates and simultaneous outcomes without observing hidden state.
pub fn compile(contract: &Contract) -> Result<Model, Error> {
    let raw = &contract.0;
    let mut variables = raw.variables.clone();
    variables.sort_by(|a, b| a.name.cmp(&b.name));
    for var in &mut variables {
        var.values.sort();
    }
    let mut states: Vec<Vec<u16>> = vec![vec![]];
    for var in &variables {
        let mut expanded = Vec::with_capacity(states.len() * var.values.len());
        for state in states {
            for value in 0..var.values.len() {
                let mut next = state.clone();
                next.push(u16::try_from(value).expect("bounded domain"));
                expanded.push(next);
            }
        }
        states = expanded;
    }
    let indices: BTreeMap<Vec<u16>, usize> = states
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, state)| (state, i))
        .collect();
    let mut ir = Ir {
        semantic_version: 1,
        variables,
        states,
        initial: vec![],
        safe: vec![],
        goal: vec![],
        actions: vec![],
    };
    let mut work = 0;
    for (index, state) in ir.states.iter().enumerate() {
        if raw.initial.eval(state, &ir.variables, &mut work)? {
            ir.initial.push(index);
        }
        if raw.safe.eval(state, &ir.variables, &mut work)? {
            ir.safe.push(index);
        }
        if raw.goal.eval(state, &ir.variables, &mut work)? {
            ir.goal.push(index);
        }
    }
    if ir.initial.is_empty() {
        return Err(Error::new(
            "empty_initial",
            "initial predicate selects no world",
        ));
    }
    let mut tools: Vec<_> = raw.actions.iter().collect();
    tools.sort_by(|a, b| a.id.cmp(&b.id));
    let mut origins = Vec::new();
    for tool in tools {
        let mut action = Action {
            id: tool.id.clone(),
            edges: vec![],
        };
        let mut outcomes: Vec<_> = tool.outcomes.iter().collect();
        outcomes.sort_by(|a, b| a.id.cmp(&b.id));
        for (from, state) in ir.states.iter().enumerate() {
            if !tool.enabled.eval(state, &ir.variables, &mut work)? {
                continue;
            }
            let before = action.edges.len();
            for outcome in &outcomes {
                if !outcome.when.eval(state, &ir.variables, &mut work)? {
                    continue;
                }
                if origins.len() >= 16384 {
                    return Err(Error::new(
                        "model_limit",
                        "more than 16384 expanded outcome edges",
                    ));
                }
                let mut next = state.clone();
                for assignment in &outcome.set {
                    let var = ir
                        .variables
                        .binary_search_by(|v| v.name.cmp(&assignment.var))
                        .expect("validated assignment");
                    next[var] = u16::try_from(
                        ir.variables[var]
                            .values
                            .binary_search(&assignment.value)
                            .expect("validated value"),
                    )
                    .expect("bounded domain");
                }
                let to = indices[&next];
                action.edges.push(Edge {
                    from,
                    to,
                    observation: outcome.observe.clone(),
                });
                origins.push(Origin {
                    action: tool.id.clone(),
                    outcome: outcome.id.clone(),
                    from,
                    to,
                    observation: outcome.observe.clone(),
                });
            }
            if action.edges.len() == before {
                return Err(Error::new(
                    "uncovered_action",
                    "an enabled action has no outcome in an enumerated world",
                ));
            }
        }
        action.edges.sort();
        action.edges.dedup();
        ir.actions.push(action);
    }
    let bytes = serde_json::to_vec(&ir).expect("finite IR serializes");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok(Model {
        ir,
        digest,
        origins,
    })
}
