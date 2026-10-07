//! A raw-JSON truth table evaluator, independent of contract::Expr and compile.
use contingram::{compile, parse_contract};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
type Assignment = BTreeMap<String, String>;

fn truth(expression: &Value, values: &Assignment) -> bool {
    match expression["op"].as_str().unwrap() {
        "true" => true,
        "false" => false,
        "eq" => {
            values[expression["var"].as_str().unwrap()] == expression["value"].as_str().unwrap()
        }
        "not" => !truth(&expression["arg"], values),
        "all" => expression["args"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| truth(e, values)),
        "any" => expression["args"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| truth(e, values)),
        _ => panic!("test generated an unknown predicate"),
    }
}
fn assignments(contract: &Value) -> Vec<Assignment> {
    let mut rows = vec![Assignment::new()];
    // Use source declaration/domain order, unlike normalized implementation order.
    for variable in contract["variables"].as_array().unwrap() {
        rows = rows
            .into_iter()
            .flat_map(|row| {
                variable["values"].as_array().unwrap().iter().map(move |v| {
                    let mut next = row.clone();
                    next.insert(
                        variable["name"].as_str().unwrap().into(),
                        v.as_str().unwrap().into(),
                    );
                    next
                })
            })
            .collect();
    }
    rows
}
fn assert_lowering(contract: &Value) {
    let rows = assignments(contract);
    let model = compile(&parse_contract(&serde_json::to_vec(contract).unwrap()).unwrap()).unwrap();
    let ir = model.ir();
    let decoded: Vec<Assignment> = ir
        .states
        .iter()
        .map(|state| {
            ir.variables
                .iter()
                .enumerate()
                .map(|(i, v)| (v.name.clone(), v.values[usize::from(state[i])].clone()))
                .collect()
        })
        .collect();
    assert_eq!(
        rows.iter().cloned().collect::<BTreeSet<_>>(),
        decoded.iter().cloned().collect()
    );
    for (predicate, indices) in [
        ("initial", &ir.initial),
        ("safe", &ir.safe),
        ("goal", &ir.goal),
    ] {
        let expected: BTreeSet<_> = rows
            .iter()
            .filter(|r| truth(&contract[predicate], r))
            .cloned()
            .collect();
        let actual: BTreeSet<_> = indices.iter().map(|&i| decoded[i].clone()).collect();
        assert_eq!(actual, expected, "predicate={predicate}");
    }
    for source_action in contract["actions"].as_array().unwrap() {
        let mut expected = BTreeSet::new();
        for before in &rows {
            if !truth(&source_action["enabled"], before) {
                continue;
            }
            for outcome in source_action["outcomes"].as_array().unwrap() {
                if !truth(&outcome["when"], before) {
                    continue;
                }
                let mut after = before.clone();
                for assignment in outcome["set"].as_array().unwrap() {
                    after.insert(
                        assignment["var"].as_str().unwrap().into(),
                        assignment["value"].as_str().unwrap().into(),
                    );
                }
                expected.insert((
                    before.clone(),
                    after,
                    outcome["observe"].as_str().unwrap().to_owned(),
                ));
            }
        }
        let action = ir
            .actions
            .iter()
            .find(|a| a.id == source_action["id"].as_str().unwrap())
            .unwrap();
        let actual: BTreeSet<_> = action
            .edges
            .iter()
            .map(|e| {
                (
                    decoded[e.from].clone(),
                    decoded[e.to].clone(),
                    e.observation.clone(),
                )
            })
            .collect();
        assert_eq!(actual, expected, "action={}", action.id);
    }
}

#[test]
fn every_corpus_edge_and_predicate_matches_separate_truth_tables() {
    let manifest: Value =
        serde_json::from_slice(include_bytes!("../examples/contracts/manifest.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/contracts")
            .join(case["file"].as_str().unwrap());
        assert_lowering(&serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap());
    }
    assert_lowering(
        &serde_json::from_slice(include_bytes!("../examples/contracts/receipt.json")).unwrap(),
    );
}

#[test]
fn composed_predicates_and_simultaneous_assignments_match_raw_reference() {
    let mut expressions = vec![
        json!({"op":"true"}),
        json!({"op":"false"}),
        json!({"op":"all","args":[]}),
        json!({"op":"any","args":[]}),
    ];
    for variable in ["x", "y"] {
        for value in ["on", "off"] {
            expressions.push(json!({"op":"eq","var":variable,"value":value}));
        }
    }
    let atoms = expressions.clone();
    for left in &atoms {
        for right in &atoms {
            expressions.push(json!({"op":"all","args":[left,{"op":"not","arg":right}]}));
            expressions.push(json!({"op":"any","args":[left,{"op":"not","arg":right}]}));
        }
    }
    for expression in expressions {
        let contract = json!({"schema_version":1,"name":"truth-table","variables":[{"name":"y","values":["on","off"]},{"name":"x","values":["on","off"]}],"initial":{"op":"true"},"safe":expression,"goal":{"op":"not","arg":expression},"actions":[
            {"id":"edit","enabled":{"op":"true"},"outcomes":[
                {"id":"unchanged","when":{"op":"true"},"set":[],"observe":"same"},
                {"id":"both","when":expression,"set":[{"var":"x","value":"on"},{"var":"y","value":"off"}],"observe":"same"},
                {"id":"other","when":{"op":"not","arg":expression},"set":[{"var":"x","value":"off"}],"observe":"other"}
            ]},
            {"id":"guarded","enabled":expression,"outcomes":[{"id":"always","when":{"op":"true"},"set":[],"observe":"guarded"}]}
        ]});
        assert_lowering(&contract);
    }
}
