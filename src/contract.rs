use crate::{label, Error, MAX_CONTRACT_BYTES};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Immutable, structurally validated finite tool contract.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct Contract(pub(crate) RawContract);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RawContract {
    pub schema_version: u32,
    pub name: String,
    pub variables: Vec<crate::Variable>,
    pub initial: Expr,
    pub safe: Expr,
    pub goal: Expr,
    pub actions: Vec<Tool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tool {
    pub id: String,
    pub enabled: Expr,
    pub outcomes: Vec<Outcome>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Outcome {
    pub id: String,
    pub when: Expr,
    pub set: Vec<Assignment>,
    pub observe: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Assignment {
    pub var: String,
    pub value: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Expr {
    True {},
    False {},
    Eq { var: String, value: String },
    All { args: Vec<Expr> },
    Any { args: Vec<Expr> },
    Not { arg: Box<Expr> },
}

/// Parse data-only JSON; unknown and duplicate fields are errors.
pub fn parse_contract(bytes: &[u8]) -> Result<Contract, Error> {
    if bytes.len() > MAX_CONTRACT_BYTES {
        return Err(Error::new("input_limit", "contract exceeds 1 MiB"));
    }
    let raw: RawContract = serde_json::from_slice(bytes).map_err(crate::json_error)?;
    validate(&raw)?;
    Ok(Contract(raw))
}

fn validate(raw: &RawContract) -> Result<(), Error> {
    if raw.schema_version != 1 {
        return Err(Error::new(
            "unsupported_version",
            "contract schema_version must be 1",
        ));
    }
    label(&raw.name)?;
    if raw.variables.is_empty() || raw.variables.len() > 12 || raw.actions.len() > 32 {
        return Err(Error::new(
            "model_limit",
            "require 1..12 variables and at most 32 actions",
        ));
    }
    let mut vars = BTreeMap::new();
    let mut product = 1usize;
    for v in &raw.variables {
        label(&v.name)?;
        if v.values.is_empty() || v.values.len() > 16 {
            return Err(Error::new(
                "model_limit",
                "variable domain requires 1..16 values",
            ));
        }
        let mut values = BTreeSet::new();
        for value in &v.values {
            label(value)?;
            if !values.insert(value.as_str()) {
                return Err(Error::new("duplicate_id", "duplicate domain value"));
            }
        }
        if vars.insert(v.name.as_str(), values).is_some() {
            return Err(Error::new("duplicate_id", "duplicate variable"));
        }
        product = product
            .checked_mul(v.values.len())
            .ok_or_else(|| Error::new("model_limit", "state product overflow"))?;
        if product > 256 {
            return Err(Error::new(
                "model_limit",
                "Cartesian product exceeds 256 worlds",
            ));
        }
    }
    let mut nodes = 0;
    for expr in [&raw.initial, &raw.safe, &raw.goal] {
        check_expr(expr, &vars, &mut nodes)?;
    }
    let mut tools = BTreeSet::new();
    let mut outcomes = 0;
    for tool in &raw.actions {
        label(&tool.id)?;
        if !tools.insert(&tool.id) {
            return Err(Error::new("duplicate_id", "duplicate action"));
        }
        check_expr(&tool.enabled, &vars, &mut nodes)?;
        outcomes += tool.outcomes.len();
        if outcomes > 512 {
            return Err(Error::new("model_limit", "more than 512 outcomes"));
        }
        let mut ids = BTreeSet::new();
        for outcome in &tool.outcomes {
            label(&outcome.id)?;
            label(&outcome.observe)?;
            if !ids.insert(&outcome.id) {
                return Err(Error::new("duplicate_id", "duplicate outcome"));
            }
            check_expr(&outcome.when, &vars, &mut nodes)?;
            if outcome.set.len() > raw.variables.len() {
                return Err(Error::new("invalid_assignment", "too many assignments"));
            }
            let mut assigned = BTreeSet::new();
            for assignment in &outcome.set {
                reference(&assignment.var, &assignment.value, &vars)?;
                if !assigned.insert(&assignment.var) {
                    return Err(Error::new(
                        "invalid_assignment",
                        "duplicate assignment to variable",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn reference(var: &str, value: &str, vars: &BTreeMap<&str, BTreeSet<&str>>) -> Result<(), Error> {
    label(var)?;
    label(value)?;
    if !vars.get(var).is_some_and(|values| values.contains(value)) {
        return Err(Error::new(
            "invalid_reference",
            "undeclared variable or value",
        ));
    }
    Ok(())
}

fn check_expr(
    expr: &Expr,
    vars: &BTreeMap<&str, BTreeSet<&str>>,
    nodes: &mut usize,
) -> Result<(), Error> {
    let mut stack = vec![(expr, 1usize)];
    while let Some((expr, depth)) = stack.pop() {
        *nodes += 1;
        if *nodes > 4096 || depth > 16 {
            return Err(Error::new(
                "predicate_limit",
                "predicate depth exceeds 16 or total nodes exceeds 4096",
            ));
        }
        match expr {
            Expr::Eq { var, value } => reference(var, value, vars)?,
            Expr::All { args } | Expr::Any { args } => {
                if args.len() > 4096 {
                    return Err(Error::new(
                        "predicate_limit",
                        "too many predicate arguments",
                    ));
                }
                stack.extend(args.iter().map(|a| (a, depth + 1)));
            }
            Expr::Not { arg } => stack.push((arg, depth + 1)),
            Expr::True {} | Expr::False {} => {}
        }
    }
    Ok(())
}

impl Expr {
    pub(crate) fn eval(
        &self,
        state: &[u16],
        vars: &[crate::Variable],
        work: &mut u64,
    ) -> Result<bool, Error> {
        if *work >= 1_000_000 {
            return Err(Error::new(
                "compile_limit",
                "lowering exceeds 1000000 predicate evaluations",
            ));
        }
        *work += 1;
        Ok(match self {
            Self::True {} => true,
            Self::False {} => false,
            Self::Eq { var, value } => {
                let index = vars
                    .binary_search_by(|v| v.name.as_str().cmp(var))
                    .expect("validated variable");
                vars[index].values[usize::from(state[index])] == *value
            }
            Self::All { args } => {
                let mut yes = true;
                for arg in args {
                    if !arg.eval(state, vars, work)? {
                        yes = false;
                        break;
                    }
                }
                yes
            }
            Self::Any { args } => {
                let mut yes = false;
                for arg in args {
                    if arg.eval(state, vars, work)? {
                        yes = true;
                        break;
                    }
                }
                yes
            }
            Self::Not { arg } => !arg.eval(state, vars, work)?,
        })
    }
}
