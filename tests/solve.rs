use contingram::{analyze, compile, parse_contract, SearchLimits, Status};
use serde_json::{json, Value};
fn fixture() -> Value {
    serde_json::from_slice(include_bytes!("../examples/contracts/receipt.json")).unwrap()
}
fn run(v: &Value, h: u32) -> contingram::Report {
    let m = compile(&parse_contract(&serde_json::to_vec(v).unwrap()).unwrap()).unwrap();
    analyze(
        &m,
        SearchLimits {
            horizon: h,
            ..Default::default()
        },
    )
    .unwrap()
}
#[test]
fn sensing_requires_two_decisions() {
    assert_eq!(run(&fixture(), 0).status, Status::NoPolicyWithinHorizon);
    assert_eq!(run(&fixture(), 1).status, Status::NoPolicyWithinHorizon);
    assert_eq!(run(&fixture(), 2).status, Status::PolicyFound);
}
#[test]
fn unsafe_goal_cannot_win() {
    let mut v = fixture();
    v["initial"] = json!({"op":"true"});
    v["goal"] = json!({"op":"true"});
    assert_eq!(run(&v, 2).status, Status::NoPolicyWithinHorizon);
}
#[test]
fn same_observation_retains_all_worlds() {
    let mut v = fixture();
    for o in v["actions"][1]["outcomes"].as_array_mut().unwrap() {
        o["observe"] = json!("same");
    }
    assert_eq!(run(&v, 8).status, Status::NoPolicyWithinHorizon);
}
#[test]
fn disabled_world_cannot_be_dropped() {
    let mut v = fixture();
    v["actions"].as_array_mut().unwrap().truncate(1);
    v["actions"][0]["enabled"] = json!({"op":"eq","var":"effect","value":"absent"});
    assert_eq!(run(&v, 2).status, Status::NoPolicyWithinHorizon);
}
#[test]
fn no_actions_and_universal_goal() {
    let mut v = fixture();
    v["actions"] = json!([]);
    assert_eq!(run(&v, 8).status, Status::NoPolicyWithinHorizon);
    v["goal"] = json!({"op":"true"});
    assert_eq!(run(&v, 0).status, Status::PolicyFound);
}
#[test]
fn exhaustion_has_no_certificate_and_is_never_loss() {
    let m = compile(&parse_contract(&serde_json::to_vec(&fixture()).unwrap()).unwrap()).unwrap();
    for cap in 0..100 {
        let r = analyze(
            &m,
            SearchLimits {
                horizon: 2,
                max_nodes: 100,
                max_work: cap,
            },
        )
        .unwrap();
        assert_ne!(r.status, Status::NoPolicyWithinHorizon);
        if r.status == Status::Unknown {
            assert!(r.certificate.is_none());
            assert!(r.limit.is_some());
        }
    }
    assert!(analyze(
        &m,
        SearchLimits {
            horizon: 33,
            ..Default::default()
        }
    )
    .is_err());
    assert_eq!(
        analyze(
            &m,
            SearchLimits {
                max_nodes: 0,
                ..Default::default()
            }
        )
        .unwrap()
        .status,
        Status::Unknown
    );
}
