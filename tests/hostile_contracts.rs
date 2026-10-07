use contingram::{compile, parse_contract};
use serde_json::{json, Value};

fn base() -> Value {
    serde_json::from_slice(include_bytes!("../examples/contracts/receipt.json")).unwrap()
}
fn rejects(v: &Value) -> bool {
    parse_contract(&serde_json::to_vec(v).unwrap())
        .and_then(|c| compile(&c))
        .is_err()
}

#[test]
fn rejects_invalid_references_even_in_disabled_outcomes() {
    let mut v = base();
    v["actions"][0]["enabled"] = json!({"op":"false"});
    v["actions"][0]["outcomes"][0]["set"][0]["value"] = json!("undeclared");
    assert!(rejects(&v));
}

#[test]
fn duplicate_assignments_are_not_last_write_wins() {
    let mut v = base();
    v["actions"][0]["outcomes"][0]["set"] =
        json!([{"var":"effect","value":"done"},{"var":"effect","value":"absent"}]);
    assert!(rejects(&v));
}

#[test]
fn no_empty_initial_belief_or_empty_domain() {
    let mut v = base();
    v["initial"] = json!({"op":"false"});
    assert!(rejects(&v));
    let mut v = base();
    v["variables"][0]["values"] = json!([]);
    assert!(rejects(&v));
}

#[test]
fn cannot_bypass_product_bound_with_small_individual_domains() {
    let mut v = base();
    for n in 0..9 {
        v["variables"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":format!("bit{n}"),"values":["off","on"]}));
    }
    assert!(rejects(&v));
}

#[test]
fn rejects_duplicate_keys_unknown_fields_and_control_labels() {
    let raw = String::from_utf8(serde_json::to_vec(&base()).unwrap()).unwrap();
    let raw = raw.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(parse_contract(raw.as_bytes()).is_err());
    let mut v = base();
    v["safe"] = json!({"op":"true","ignored":false});
    assert!(rejects(&v));
    v = base();
    v["actions"][0]["outcomes"][0]["observe"] = json!(format!("ack{}[31m", char::from(27)));
    assert!(rejects(&v));
}

#[test]
fn json_size_and_predicate_depth_are_bounded() {
    assert!(parse_contract(&vec![b' '; 1024 * 1024 + 1]).is_err());
    let mut v = base();
    let mut p = json!({"op":"true"});
    for _ in 0..17 {
        p = json!({"op":"not","arg":p});
    }
    v["safe"] = p;
    assert!(rejects(&v));
}
