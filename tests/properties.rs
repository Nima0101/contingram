use contingram::{analyze, compile, parse_contract, report_bytes, SearchLimits, Status};
use serde_json::{json, Value};

fn corpus() -> Vec<Value> {
    let manifest: Value =
        serde_json::from_slice(include_bytes!("../examples/contracts/manifest.json")).unwrap();
    manifest["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("examples/contracts")
                .join(case["file"].as_str().unwrap());
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        })
        .collect()
}
fn run(contract: &Value, horizon: u32) -> contingram::Report {
    let model = compile(&parse_contract(&serde_json::to_vec(contract).unwrap()).unwrap()).unwrap();
    analyze(
        &model,
        SearchLimits {
            horizon,
            ..Default::default()
        },
    )
    .unwrap()
}
fn win(contract: &Value, horizon: u32) -> bool {
    let status = run(contract, horizon).status;
    assert_ne!(
        status,
        Status::Unknown,
        "small metamorphic test must complete"
    );
    status == Status::PolicyFound
}

#[test]
fn increasing_horizon_preserves_every_existing_policy() {
    for contract in corpus() {
        let mut won = false;
        for horizon in 0..=5 {
            let now = win(&contract, horizon);
            assert!(
                !won || now,
                "lost policy at H={horizon}: {}",
                contract["name"]
            );
            won = now;
        }
    }
}

#[test]
fn bijective_observation_renaming_preserves_verdicts() {
    for contract in corpus() {
        let mut renamed = contract.clone();
        for action in renamed["actions"].as_array_mut().unwrap() {
            for outcome in action["outcomes"].as_array_mut().unwrap() {
                outcome["observe"] =
                    json!(format!("renamed-{}", outcome["observe"].as_str().unwrap()));
            }
        }
        for horizon in 0..=4 {
            assert_eq!(win(&contract, horizon), win(&renamed, horizon));
        }
    }
}

#[test]
fn observation_refinement_cannot_destroy_a_policy() {
    for contract in corpus() {
        let mut refined = contract.clone();
        for action in refined["actions"].as_array_mut().unwrap() {
            for (index, outcome) in action["outcomes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                // Retain the old label as a prefix, so this strictly splits partitions.
                outcome["observe"] =
                    json!(format!("{}-r{index}", outcome["observe"].as_str().unwrap()));
            }
        }
        for horizon in 0..=4 {
            assert!(
                !win(&contract, horizon) || win(&refined, horizon),
                "{}",
                contract["name"]
            );
        }
    }
}

#[test]
fn weakened_safety_cannot_destroy_a_policy() {
    for contract in corpus() {
        let mut weakened = contract.clone();
        weakened["safe"] = json!({"op":"true"});
        for horizon in 0..=4 {
            assert!(!win(&contract, horizon) || win(&weakened, horizon));
        }
    }
}

#[test]
fn adding_adversarial_self_loops_cannot_create_a_policy() {
    for contract in corpus() {
        let mut adversarial = contract.clone();
        for action in adversarial["actions"].as_array_mut().unwrap() {
            // The added outcome applies precisely where the original action was enabled.
            // Existing edges and enabledness remain unchanged; this is not edge replacement.
            let guard = action["enabled"].clone();
            action["outcomes"].as_array_mut().unwrap().push(
                json!({"id":"adversarial-loop","when":guard,"set":[],"observe":"adversarial-wait"}),
            );
        }
        for horizon in 0..=4 {
            assert!(!win(&adversarial, horizon) || win(&contract, horizon));
        }
    }
}

#[test]
fn declaration_permutations_preserve_report_bytes() {
    for contract in corpus() {
        let mut reordered = contract.clone();
        reordered["variables"].as_array_mut().unwrap().reverse();
        for var in reordered["variables"].as_array_mut().unwrap() {
            var["values"].as_array_mut().unwrap().reverse();
        }
        reordered["actions"].as_array_mut().unwrap().reverse();
        for action in reordered["actions"].as_array_mut().unwrap() {
            action["outcomes"].as_array_mut().unwrap().reverse();
        }
        assert_eq!(
            report_bytes(&run(&contract, 4)).unwrap(),
            report_bytes(&run(&reordered, 4)).unwrap(),
            "{}",
            contract["name"]
        );
    }
}
