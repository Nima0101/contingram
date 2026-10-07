//! An exhaustive small-model reference: tables are generated before JSON lowering.
//! It enumerates all nonempty belief subsets bottom-up, with no production search
//! state, memo keys, certificate traversal, or IR transition builder in the oracle.
use contingram::{analyze, compile, parse_contract, SearchLimits, Status};
use serde_json::{json, Value};

#[derive(Clone)]
struct Tiny {
    initial: u8,
    safe: u8,
    goal: u8,
    // action -> world -> nondeterministic (destination, observation) outcomes
    table: Vec<Vec<Vec<(usize, usize)>>>,
}

impl Tiny {
    fn winning_sets(&self, horizon: usize) -> Vec<bool> {
        let mut previous = vec![false; 8];
        for (belief, winner) in previous.iter_mut().enumerate().skip(1) {
            *winner =
                belief & !usize::from(self.safe) == 0 && belief & !usize::from(self.goal) == 0;
        }
        for _ in 0..horizon {
            let mut next = previous.clone();
            for (belief, winner) in next.iter_mut().enumerate().skip(1) {
                if belief & !usize::from(self.safe) != 0 || *winner {
                    continue;
                }
                for action in &self.table {
                    let mut post = [0usize; 2];
                    let mut enabled_everywhere = true;
                    for (world, outcomes) in action.iter().enumerate() {
                        if belief & (1 << world) != 0 {
                            enabled_everywhere &= !outcomes.is_empty();
                            for &(to, observation) in outcomes {
                                post[observation] |= 1 << to;
                            }
                        }
                    }
                    if enabled_everywhere && post.iter().all(|&b| b == 0 || previous[b]) {
                        *winner = true;
                    }
                }
            }
            previous = next;
        }
        previous
    }

    fn contract(&self) -> Value {
        fn predicate(mask: u8) -> Value {
            json!({"op":"any","args":(0..3).filter(|s| mask & (1 << s) != 0).map(|s|json!({"op":"eq","var":"state","value":format!("s{s}")})).collect::<Vec<_>>()})
        }
        let actions: Vec<_> = self
            .table
            .iter()
            .enumerate()
            .map(|(a, table)| {
                let enabled = table.iter().enumerate().fold(0u8, |bits, (s, outcomes)| {
                    bits | if outcomes.is_empty() { 0 } else { 1 << s }
                });
                let outcomes: Vec<_> = table
                    .iter()
                    .enumerate()
                    .flat_map(|(from, row)| {
                        row.iter().enumerate().map(move |(i, (to, obs))| json!({
                "id":format!("r{from}-{i}"),"when":predicate(1 << from),
                "set":[{"var":"state","value":format!("s{to}")}],"observe":format!("o{obs}")
            }))
                    })
                    .collect();
                json!({"id":format!("a{a}"),"enabled":predicate(enabled),"outcomes":outcomes})
            })
            .collect();
        json!({"schema_version":1,"name":"generated-reference","variables":[{"name":"state","values":["s0","s1","s2"]}],"initial":predicate(self.initial),"safe":predicate(self.safe),"goal":predicate(self.goal),"actions":actions})
    }
}

fn generated(seed: u64) -> Tiny {
    let mut state = seed ^ 0xc071_19a4_023b_851d;
    let mut draw = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        state >> 32
    };
    let initial = (1 + draw() % 7) as u8;
    let safe = (draw() % 8) as u8;
    let goal = (draw() % 8) as u8;
    let table = (0..2)
        .map(|_| {
            (0..3)
                .map(|_| {
                    let count = draw() % 3;
                    (0..count)
                        .map(|_| ((draw() % 3) as usize, (draw() % 2) as usize))
                        .collect()
                })
                .collect()
        })
        .collect();
    Tiny {
        initial,
        safe,
        goal,
        table,
    }
}

#[test]
fn exhaustive_belief_oracle_matches_128_original_tables_at_five_horizons() {
    let mut positive = 0;
    let mut negative = 0;
    for seed in 0..128 {
        let tiny = generated(seed);
        let model =
            compile(&parse_contract(&serde_json::to_vec(&tiny.contract()).unwrap()).unwrap())
                .unwrap();
        for horizon in 0..=4 {
            let expected = tiny.winning_sets(horizon)[usize::from(tiny.initial)];
            positive += usize::from(expected);
            negative += usize::from(!expected);
            let report = analyze(
                &model,
                SearchLimits {
                    horizon: horizon as u32,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(
                report.status,
                if expected {
                    Status::PolicyFound
                } else {
                    Status::NoPolicyWithinHorizon
                },
                "seed={seed}, H={horizon}"
            );
        }
    }
    assert!(
        positive > 20 && negative > 20,
        "oracle must exercise both polarities"
    );
}

#[test]
fn frozen_original_corpus_expectations() {
    let manifest: Value =
        serde_json::from_slice(include_bytes!("../examples/contracts/manifest.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/contracts")
            .join(case["file"].as_str().unwrap());
        let model = compile(&parse_contract(&std::fs::read(path).unwrap()).unwrap()).unwrap();
        let report = analyze(
            &model,
            SearchLimits {
                horizon: case["horizon"].as_u64().unwrap() as u32,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(report.status).unwrap(),
            case["expected_status"],
            "{}: {}",
            case["file"],
            case["reason"]
        );
    }
}

#[test]
fn three_world_obstruction_cannot_be_replaced_with_pairwise_checks() {
    let mut contract: Value = serde_json::from_slice(include_bytes!(
        "../examples/contracts/three-world-obstruction.json"
    ))
    .unwrap();
    for omitted in 0..3 {
        let mut pair = contract["initial"]["args"].as_array().unwrap().clone();
        pair.remove(omitted);
        contract["initial"] = json!({"op":"any","args":pair});
        let model =
            compile(&parse_contract(&serde_json::to_vec(&contract).unwrap()).unwrap()).unwrap();
        assert_eq!(
            analyze(
                &model,
                SearchLimits {
                    horizon: 1,
                    ..Default::default()
                }
            )
            .unwrap()
            .status,
            Status::PolicyFound
        );
        contract = serde_json::from_slice(include_bytes!(
            "../examples/contracts/three-world-obstruction.json"
        ))
        .unwrap();
    }
}
