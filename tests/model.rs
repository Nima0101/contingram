use contingram::{compile, parse_contract};
use serde_json::{json, Value};

fn receipt() -> Value {
    serde_json::from_slice(include_bytes!("../examples/contracts/receipt.json")).unwrap()
}

fn lower(value: &Value) -> contingram::Model {
    compile(&parse_contract(&serde_json::to_vec(value).unwrap()).unwrap()).unwrap()
}

#[test]
fn lowers_correlated_effects_and_observations() {
    let model = lower(&receipt());
    let ir = serde_json::to_value(model.ir()).unwrap();
    assert_eq!(ir["states"], json!([[0], [1], [2]]));
    assert_eq!(ir["initial"], json!([0, 1]));
    assert_eq!(ir["safe"], json!([0, 1]));
    assert_eq!(ir["goal"], json!([1]));
    assert_eq!(
        ir["actions"][0]["edges"],
        json!([
            {"from":0,"to":1,"observation":"ack"},
            {"from":1,"to":2,"observation":"ack"},
            {"from":2,"to":2,"observation":"ack"}
        ])
    );
}

#[test]
fn declaration_order_and_diagnostic_ids_do_not_change_binding() {
    let original = lower(&receipt());
    let mut changed = receipt();
    changed["variables"][0]["values"]
        .as_array_mut()
        .unwrap()
        .reverse();
    changed["actions"].as_array_mut().unwrap().reverse();
    changed["name"] = json!("new-title");
    changed["actions"][0]["outcomes"][0]["id"] = json!("new-origin");
    assert_eq!(original.digest(), lower(&changed).digest());
}

#[test]
fn disabled_actions_do_not_gain_edges_from_true_outcomes() {
    let mut input = receipt();
    input["actions"][0]["enabled"] = json!({"op":"false"});
    let model = lower(&input);
    assert!(
        serde_json::to_value(model.ir()).unwrap()["actions"][0]["edges"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn enabled_uncovered_world_is_rejected_even_if_unreachable() {
    let mut input = receipt();
    input["actions"][1]["outcomes"]
        .as_array_mut()
        .unwrap()
        .pop();
    let contract = parse_contract(&serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(compile(&contract).is_err());
}

#[test]
fn equivalent_outcomes_do_not_change_semantic_binding() {
    let input = receipt();
    let before = lower(&input);
    let mut after = input.clone();
    let mut duplicate = input["actions"][0]["outcomes"][0].clone();
    duplicate["id"] = json!("equivalent");
    after["actions"][0]["outcomes"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert_eq!(before.digest(), lower(&after).digest());
}
