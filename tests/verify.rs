use contingram::{
    analyze, compile, follow, parse_contract, parse_report, verify, CheckLimits, SearchLimits,
};
use serde_json::{json, Value};
fn case(h: u32) -> (contingram::Model, Value) {
    let m = compile(&parse_contract(include_bytes!("../examples/contracts/receipt.json")).unwrap())
        .unwrap();
    let r = analyze(
        &m,
        SearchLimits {
            horizon: h,
            ..Default::default()
        },
    )
    .unwrap();
    (m, serde_json::to_value(r).unwrap())
}
fn reject(m: &contingram::Model, v: &Value) {
    assert!(parse_report(&serde_json::to_vec(v).unwrap())
        .and_then(|r| verify(m, &r, CheckLimits::default()))
        .is_err());
}
#[test]
fn rejects_positive_and_negative_forgery() {
    for h in [1, 2] {
        let (m, v) = case(h);
        let mut x = v.clone();
        x["model_sha256"] = json!("0".repeat(64));
        reject(&m, &x);
        let mut x = v.clone();
        x["schema_version"] = json!(2);
        reject(&m, &x);
        let mut x = v.clone();
        x["certificate"]["root"] = json!(999999);
        reject(&m, &x);
        let mut x = v.clone();
        x["certificate"]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!({"kind":"goal"}));
        reject(&m, &x);
        let mut x = v.clone();
        let root = x["certificate"]["root"].as_u64().unwrap() as usize;
        x["certificate"]["nodes"][root] = json!({"kind":"goal"});
        reject(&m, &x);
    }
}
#[test]
fn branch_and_action_coverage_is_exact() {
    let (m, mut v) = case(2);
    let n = v["certificate"]["root"].as_u64().unwrap() as usize;
    v["certificate"]["nodes"][n]["branches"]
        .as_array_mut()
        .unwrap()
        .pop();
    reject(&m, &v);
    let (m, mut v) = case(1);
    let n = v["certificate"]["root"].as_u64().unwrap() as usize;
    v["certificate"]["nodes"][n]["actions"]
        .as_array_mut()
        .unwrap()
        .pop();
    reject(&m, &v);
}
#[test]
fn cycles_wrong_polarity_and_zero_horizon_are_rejected() {
    let (m, v) = case(2);
    let n = v["certificate"]["root"].as_u64().unwrap() as usize;
    let mut x = v.clone();
    x["certificate"]["nodes"][n]["branches"][0]["next"] = json!(n);
    reject(&m, &x);
    let mut x = v.clone();
    x["status"] = json!("no_policy_within_horizon");
    reject(&m, &x);
    let mut x = v;
    x["horizon"] = json!(0);
    reject(&m, &x);
}
#[test]
fn explicit_check_limits_are_distinct_and_follow_rechecks() {
    let (m, v) = case(2);
    let r = parse_report(&serde_json::to_vec(&v).unwrap()).unwrap();
    assert_eq!(
        verify(
            &m,
            &r,
            CheckLimits {
                max_nodes: 0,
                max_work: 100
            }
        )
        .unwrap_err()
        .code,
        "verification_limit"
    );
    assert_eq!(
        follow(&m, &r, &["impossible".into()], CheckLimits::default())
            .unwrap_err()
            .code,
        "model_mismatch"
    );
    assert!(follow(
        &m,
        &r,
        &["absent-final".into(), "ack".into()],
        CheckLimits::default()
    )
    .is_ok());
    assert!(follow(
        &m,
        &r,
        &["committed".into(), "ack".into()],
        CheckLimits::default()
    )
    .is_err());
}
