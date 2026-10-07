//! Expectations were frozen by an independent fixture author before solver execution.
use contingram::{analyze, compile, parse_contract, verify, CheckLimits, SearchLimits};
use serde_json::Value;

#[test]
fn independently_authored_heldout_expectations_and_relation_counts() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/heldout");
    let expectations: Value =
        serde_json::from_slice(&std::fs::read(root.join("expectations.json")).unwrap()).unwrap();
    let metrics: Value =
        serde_json::from_slice(&std::fs::read(root.join("metrics.json")).unwrap()).unwrap();
    let mut checked = 0;
    for case in expectations["cases"].as_array().unwrap() {
        let file = case["file"].as_str().unwrap();
        let model =
            compile(&parse_contract(&std::fs::read(root.join(file)).unwrap()).unwrap()).unwrap();
        let expected = metrics["contracts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["file"] == file)
            .unwrap();
        assert_eq!(
            model.ir().states.len() as u64,
            expected["worlds"].as_u64().unwrap(),
            "{file}"
        );
        assert_eq!(
            model.ir().initial.len() as u64,
            expected["initial_worlds"].as_u64().unwrap(),
            "{file}"
        );
        assert_eq!(
            model
                .ir()
                .actions
                .iter()
                .map(|a| a.edges.len())
                .sum::<usize>() as u64,
            expected["edges"].as_u64().unwrap(),
            "{file}"
        );
        for expected in case["expected"].as_array().unwrap() {
            let horizon = expected["horizon"].as_u64().unwrap() as u32;
            let report = analyze(
                &model,
                SearchLimits {
                    horizon,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(report.status).unwrap(),
                expected["status"],
                "{file}, H={horizon}: {}",
                case["rationale"]
            );
            verify(&model, &report, CheckLimits::default()).unwrap();
            checked += 1;
        }
    }
    assert_eq!(checked, 54);
}
