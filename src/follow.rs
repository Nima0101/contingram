use crate::{verify, CheckLimits, Error, Model, Node, Report, Status};
use serde::Serialize;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Decision {
    Action { action: String },
    Goal {},
}
/// Follow a checked observation prefix without invoking any external tool.
pub fn follow(
    model: &Model,
    report: &Report,
    observations: &[String],
    limits: CheckLimits,
) -> Result<Decision, Error> {
    if observations.len() > report.horizon as usize || observations.len() > 32 {
        return Err(Error::new(
            "model_mismatch",
            "observation prefix exceeds horizon",
        ));
    }
    for o in observations {
        crate::label(o)?;
    }
    verify(model, report, limits)?;
    if report.status != Status::PolicyFound {
        return Err(Error::new(
            "no_policy",
            "following requires a positive policy",
        ));
    }
    let cert = report.certificate.as_ref().expect("verified certificate");
    let mut current = cert.root;
    for observation in observations {
        let Node::Choose { branches, .. } = &cert.nodes[current] else {
            return Err(Error::new(
                "model_mismatch",
                "observation after terminal goal",
            ));
        };
        current = branches
            .iter()
            .find(|b| b.observation == *observation)
            .ok_or_else(|| {
                Error::new(
                    "model_mismatch",
                    "observation is not admitted by the checked policy",
                )
            })?
            .next;
    }
    match &cert.nodes[current] {
        Node::Goal {} => Ok(Decision::Goal {}),
        Node::Choose { action, .. } => Ok(Decision::Action {
            action: action.clone(),
        }),
        _ => Err(Error::new("invalid_certificate", "non-policy node")),
    }
}
