use contingram::*;
use serde_json::{json, Value};
fn receipt() -> Model {
    compile(&parse_contract(include_bytes!("../examples/contracts/receipt.json")).unwrap()).unwrap()
}
fn positive() -> Report {
    analyze(
        &receipt(),
        SearchLimits {
            horizon: 2,
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn shared_goal_is_checked_under_each_belief() {
    let m = receipt();
    let mut r = positive();
    r.certificate = Some(Certificate {
        root: 1,
        nodes: vec![
            Node::Goal {},
            Node::Choose {
                action: "status".into(),
                branches: vec![
                    Branch {
                        observation: "absent-final".into(),
                        next: 0,
                    },
                    Branch {
                        observation: "committed".into(),
                        next: 0,
                    },
                ],
            },
        ],
    });
    assert_eq!(
        verify(&m, &r, CheckLimits::default()).unwrap_err().code,
        "invalid_certificate"
    );
}
#[test]
fn malformed_node_fields_duplicate_json_and_unreachable_evidence_rejected() {
    let r = positive();
    let m = receipt();
    let bytes = report_bytes(&r).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    assert!(parse_report(
        text.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"schema_version\":1",
            1
        )
        .as_bytes()
    )
    .is_err());
    assert!(parse_report(
        text.replace("\"kind\":\"goal\"", "\"kind\":\"goal\",\"secret\":true")
            .as_bytes()
    )
    .is_err());
    let mut v = serde_json::to_value(&r).unwrap();
    let nodes = v["certificate"]["nodes"].as_array_mut().unwrap();
    nodes.insert(0, json!({"kind":"goal"}));
    for node in nodes.iter_mut().skip(1) {
        if let Some(b) = node.get_mut("branches").and_then(Value::as_array_mut) {
            for branch in b {
                branch["next"] = json!(branch["next"].as_u64().unwrap() + 1);
            }
        }
    }
    v["certificate"]["root"] = json!(v["certificate"]["root"].as_u64().unwrap() + 1);
    assert!(verify(
        &m,
        &parse_report(&serde_json::to_vec(&v).unwrap()).unwrap(),
        CheckLimits::default()
    )
    .is_err());
    assert!(parse_report(&vec![b' '; MAX_REPORT_BYTES + 1]).is_err());
}
#[test]
fn forged_negative_reasons_and_observations_fail() {
    let m = receipt();
    let r = analyze(
        &m,
        SearchLimits {
            horizon: 1,
            ..Default::default()
        },
    )
    .unwrap();
    for reason in [
        json!({"kind":"disabled"}),
        json!({"kind":"unsafe"}),
        json!({"kind":"branch","observation":"not-admitted","next":0}),
    ] {
        let mut v = serde_json::to_value(&r).unwrap();
        let root = v["certificate"]["root"].as_u64().unwrap() as usize;
        let obstruction = v["certificate"]["nodes"][root]["actions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|a| a["action"] == "status")
            .unwrap();
        obstruction["reason"] = reason;
        assert!(verify(
            &m,
            &parse_report(&serde_json::to_vec(&v).unwrap()).unwrap(),
            CheckLimits::default()
        )
        .is_err());
    }
}
#[test]
fn exact_work_and_context_boundaries_and_losing_cutoffs() {
    let m = receipt();
    let r = positive();
    let checked = verify(&m, &r, CheckLimits::default()).unwrap();
    verify(
        &m,
        &r,
        CheckLimits {
            max_nodes: checked.stats.nodes,
            max_work: checked.stats.work,
        },
    )
    .unwrap();
    for limits in [
        CheckLimits {
            max_nodes: checked.stats.nodes - 1,
            max_work: checked.stats.work,
        },
        CheckLimits {
            max_nodes: checked.stats.nodes,
            max_work: checked.stats.work - 1,
        },
    ] {
        assert_eq!(
            verify(&m, &r, limits).unwrap_err().code,
            "verification_limit"
        );
    }
    let full = analyze(
        &m,
        SearchLimits {
            horizon: 1,
            ..Default::default()
        },
    )
    .unwrap();
    for work in 0..=full.stats.work {
        let r = analyze(
            &m,
            SearchLimits {
                horizon: 1,
                max_work: work,
                ..Default::default()
            },
        )
        .unwrap();
        if r.status == Status::Unknown {
            assert!(r.certificate.is_none());
            assert!(r.limit.is_some());
        } else {
            assert_eq!(r.status, Status::NoPolicyWithinHorizon);
            verify(&m, &r, CheckLimits::default()).unwrap();
        }
    }
    assert!(verify(
        &m,
        &r,
        CheckLimits {
            max_nodes: 50_001,
            max_work: 1
        }
    )
    .is_err());
}
#[test]
fn byte_fuzz_smoke_reaches_parsing_and_semantics() {
    let seed = include_bytes!("../examples/contracts/receipt.json");
    let mut random = 0xc071_6a4du64;
    for i in 0..2000 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut data = seed.to_vec();
        if i % 3 == 0 {
            data.truncate(random as usize % data.len());
        } else {
            let index = random as usize % data.len();
            data[index] = (random >> 32) as u8;
        }
        if let Ok(c) = parse_contract(&data) {
            if let Ok(m) = compile(&c) {
                let r = analyze(
                    &m,
                    SearchLimits {
                        horizon: 3,
                        max_nodes: 100,
                        max_work: 1000,
                    },
                )
                .unwrap();
                if r.status != Status::Unknown {
                    verify(&m, &r, CheckLimits::default()).unwrap();
                }
            }
        }
        let _ = parse_report(&data);
    }
}
#[test]
fn error_diagnostics_do_not_echo_modeled_identifiers() {
    let mut v: Value =
        serde_json::from_slice(include_bytes!("../examples/contracts/receipt.json")).unwrap();
    v["goal"] = json!({"op":"eq","var":"SECRET_SENTINEL","value":"SECRET_SENTINEL"});
    let e = parse_contract(&serde_json::to_vec(&v).unwrap()).unwrap_err();
    assert!(!e.to_string().contains("SECRET_SENTINEL"));
}
