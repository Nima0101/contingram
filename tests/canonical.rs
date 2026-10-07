use contingram::{compile, parse_contract};
use serde_json::json;

#[test]
fn canonical_wire_bytes_are_frozen() {
    let input = json!({
        "schema_version":1,"name":"one-bit",
        "variables":[{"name":"x","values":["on","off"]}],
        "initial":{"op":"eq","var":"x","value":"off"},
        "safe":{"op":"true"},
        "goal":{"op":"eq","var":"x","value":"on"},
        "actions":[{"id":"set","enabled":{"op":"true"},"outcomes":[
            {"id":"done","when":{"op":"true"},"set":[{"var":"x","value":"on"}],"observe":"ok"}
        ]}]
    });
    let model = compile(&parse_contract(&serde_json::to_vec(&input).unwrap()).unwrap()).unwrap();
    let expected = r#"{"semantic_version":1,"variables":[{"name":"x","values":["off","on"]}],"states":[[0],[1]],"initial":[0],"safe":[0,1],"goal":[1],"actions":[{"id":"set","edges":[{"from":0,"to":1,"observation":"ok"},{"from":1,"to":1,"observation":"ok"}]}]}"#;
    assert_eq!(model.canonical_bytes(), expected.as_bytes());
    assert_eq!(model.digest().len(), 64);
}

#[test]
fn guards_read_pre_state_and_assignments_are_simultaneous() {
    let input = json!({
        "schema_version":1,"name":"paired",
        "variables":[{"name":"a","values":["no","yes"]},{"name":"b","values":["no","yes"]}],
        "initial":{"op":"all","args":[{"op":"eq","var":"a","value":"no"},{"op":"eq","var":"b","value":"no"}]},
        "safe":{"op":"true"},"goal":{"op":"true"},
        "actions":[{"id":"both","enabled":{"op":"eq","var":"a","value":"no"},"outcomes":[
            {"id":"first","when":{"op":"eq","var":"a","value":"no"},"set":[{"var":"a","value":"yes"},{"var":"b","value":"yes"}],"observe":"same"},
            {"id":"second","when":{"op":"eq","var":"a","value":"no"},"set":[{"var":"b","value":"no"}],"observe":"same"}
        ]}]
    });
    let model = compile(&parse_contract(&serde_json::to_vec(&input).unwrap()).unwrap()).unwrap();
    let edges = &model.ir().actions[0].edges;
    let from_zero: Vec<_> = edges.iter().filter(|e| e.from == 0).map(|e| e.to).collect();
    assert_eq!(from_zero, vec![0, 3]);
    assert!(edges.iter().all(|e| e.from < 2));
}
