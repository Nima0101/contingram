//! Manual policies are checked directly; analyze is deliberately not imported.
use contingram::{
    compile, parse_contract, verify, Certificate, CheckLimits, Model, Node, Report, Stats, Status,
};
use serde_json::Value;

// Concrete universal execution, reporting safety and completion separately.
// A decision is selected by the public DAG node, never the hidden world.
fn execute(model: &Model, certificate: &Certificate, horizon: u32) -> (bool, bool) {
    let mut pending: Vec<_> = model
        .ir()
        .initial
        .iter()
        .map(|&s| (s, certificate.root, horizon))
        .collect();
    let (mut safe, mut complete) = (true, true);
    while let Some((world, node, remaining)) = pending.pop() {
        safe &= model.ir().safe.contains(&world);
        match &certificate.nodes[node] {
            Node::Goal {} => complete &= model.ir().goal.contains(&world),
            Node::Choose { action, branches } if remaining > 0 => {
                let edges: Vec<_> = model
                    .ir()
                    .actions
                    .iter()
                    .find(|a| &a.id == action)
                    .unwrap()
                    .edges
                    .iter()
                    .filter(|e| e.from == world)
                    .collect();
                if edges.is_empty() {
                    complete = false;
                }
                for edge in edges {
                    safe &= model.ir().safe.contains(&edge.to);
                    if let Some(branch) =
                        branches.iter().find(|b| b.observation == edge.observation)
                    {
                        pending.push((edge.to, branch.next, remaining - 1));
                    } else {
                        complete = false;
                    }
                }
            }
            _ => complete = false,
        }
    }
    (safe, complete)
}

#[test]
fn three_manual_policies_and_naive_baselines() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let manifest: Value =
        serde_json::from_slice(include_bytes!("../examples/manual/manifest.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let model = compile(
            &parse_contract(
                &std::fs::read(
                    root.join("heldout")
                        .join(case["contract"].as_str().unwrap()),
                )
                .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let manual: Value = serde_json::from_slice(
            &std::fs::read(root.join("manual").join(case["file"].as_str().unwrap())).unwrap(),
        )
        .unwrap();
        let report = Report {
            schema_version: 1,
            model_sha256: model.digest().into(),
            horizon: manual["horizon"].as_u64().unwrap() as u32,
            status: Status::PolicyFound,
            stats: Stats::default(),
            certificate: Some(serde_json::from_value(manual["certificate"].clone()).unwrap()),
            limit: None,
        };
        verify(&model, &report, CheckLimits::default()).unwrap();
        assert_eq!(
            execute(&model, report.certificate.as_ref().unwrap(), report.horizon),
            (true, true)
        );
        let mut stop = report.clone();
        stop.certificate = Some(Certificate {
            root: 0,
            nodes: vec![Node::Goal {}],
        });
        assert_eq!(
            execute(&model, stop.certificate.as_ref().unwrap(), stop.horizon),
            (true, false)
        );
        assert!(verify(&model, &stop, CheckLimits::default()).is_err());
        let mut retry = report;
        retry.certificate = Some(serde_json::from_value(serde_json::json!({"root":1,"nodes":[{"kind":"goal"},{"kind":"choose","action":case["naive_action"],"branches":[{"observation":case["naive_observation"],"next":0}]}]})).unwrap());
        assert_eq!(
            execute(&model, retry.certificate.as_ref().unwrap(), retry.horizon),
            (false, false)
        );
        assert!(verify(&model, &retry, CheckLimits::default()).is_err());
    }
}

#[test]
fn stronger_manual_tables_are_rejected_after_capability_regressions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    for (policy, weaker) in [
        ("stale-receipt-refresh", "stale-receipt-sticky"),
        ("identity-expired-ledger", "identity-expired-blind"),
        ("partial-stage-status", "partial-object-status"),
    ] {
        let model = compile(
            &parse_contract(
                &std::fs::read(root.join("heldout").join(format!("{weaker}.json"))).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let manual: Value = serde_json::from_slice(
            &std::fs::read(root.join("manual").join(format!("{policy}.json"))).unwrap(),
        )
        .unwrap();
        // Rebind intentionally so rejection proves semantic failure, not just a hash mismatch.
        let report = Report {
            schema_version: 1,
            model_sha256: model.digest().into(),
            horizon: manual["horizon"].as_u64().unwrap() as u32,
            status: Status::PolicyFound,
            stats: Stats::default(),
            certificate: Some(serde_json::from_value(manual["certificate"].clone()).unwrap()),
            limit: None,
        };
        assert!(
            verify(&model, &report, CheckLimits::default()).is_err(),
            "{policy} unexpectedly survived {weaker}"
        );
    }
}
