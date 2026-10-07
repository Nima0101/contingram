//! Enumerates complete observation-history tables, then concrete executions.
//! No belief construction, recursive action selection, or production search helper.
use contingram::{analyze, compile, parse_contract, verify, CheckLimits, Ir, SearchLimits, Status};
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn table_oracle(ir: &Ir, horizon: usize) -> Option<bool> {
    let alphabet: Vec<_> = ir
        .actions
        .iter()
        .flat_map(|a| a.edges.iter().map(|e| e.observation.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut histories: Vec<Vec<String>> = Vec::new();
    let mut level = vec![vec![]];
    let radix = ir.actions.len() + 1; // zero means STOP
    let mut count = 1usize;
    for _ in 0..horizon {
        for history in &level {
            count = count.checked_mul(radix)?;
            if count > 20_000 {
                return None;
            }
            histories.push(history.clone());
        }
        level = level
            .iter()
            .flat_map(|h| {
                alphabet.iter().map(move |o| {
                    let mut next = h.clone();
                    next.push(o.clone());
                    next
                })
            })
            .collect();
    }
    for mut counter in 0..count {
        let decisions: Vec<_> = histories
            .iter()
            .map(|_| {
                let choice = counter % radix;
                counter /= radix;
                choice
            })
            .collect();
        let mut paths: Vec<_> = ir
            .initial
            .iter()
            .map(|&world| (world, Vec::<String>::new()))
            .collect();
        let mut accepted = true;
        while let Some((world, history)) = paths.pop() {
            if !ir.safe.contains(&world) {
                accepted = false;
                break;
            }
            let decision = if history.len() == horizon {
                0
            } else {
                decisions[histories.iter().position(|h| h == &history).unwrap()]
            };
            if decision == 0 {
                if !ir.goal.contains(&world) {
                    accepted = false;
                    break;
                }
                continue;
            }
            let mut enabled = false;
            for edge in &ir.actions[decision - 1].edges {
                if edge.from == world {
                    enabled = true;
                    let mut next = history.clone();
                    next.push(edge.observation.clone());
                    paths.push((edge.to, next));
                }
            }
            if !enabled {
                accepted = false;
                break;
            }
        }
        if accepted {
            return Some(true);
        }
    }
    Some(false)
}

fn predicate(mask: usize, worlds: usize) -> Value {
    json!({"op":"any","args":(0..worlds).filter(|s|mask & (1 << s) != 0).map(|s|json!({"op":"eq","var":"state","value":format!("s{s}")})).collect::<Vec<_>>()})
}
fn contract(initial: usize, safe: usize, goal: usize, table: &[Vec<Vec<(usize, usize)>>]) -> Value {
    let worlds = table[0].len();
    let actions: Vec<_> = table.iter().enumerate().map(|(a, rows)| {
        let enabled = rows.iter().enumerate().fold(0,|mask,(s,row)| mask | if row.is_empty() { 0 } else { 1 << s });
        let outcomes: Vec<_> = rows.iter().enumerate().flat_map(|(s,row)|row.iter().enumerate().map(move |(i,(to,obs))|json!({"id":format!("s{s}-e{i}"),"when":predicate(1 << s,worlds),"set":[{"var":"state","value":format!("s{to}")}],"observe":format!("o{obs}")}))).collect();
        json!({"id":format!("a{a}"),"enabled":predicate(enabled,worlds),"outcomes":outcomes})
    }).collect();
    json!({"schema_version":1,"name":"history-oracle","variables":[{"name":"state","values":(0..worlds).map(|s|format!("s{s}")).collect::<Vec<_>>()}],"initial":predicate(initial,worlds),"safe":predicate(safe,worlds),"goal":predicate(goal,worlds),"actions":actions})
}
fn compare(value: &Value, horizon: usize) -> bool {
    let model = compile(&parse_contract(&serde_json::to_vec(value).unwrap()).unwrap()).unwrap();
    let expected =
        table_oracle(model.ir(), horizon).expect("test model exceeds table enumeration envelope");
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
        "H={horizon}, model={value}"
    );
    verify(&model, &report, CheckLimits::default()).unwrap();
    expected
}

#[test]
fn exhaustive_two_world_disabled_or_deterministic_relations() {
    // 3 choices (disabled, ->s0, ->s1) for each of four action/world cells.
    // Cross every nonempty initial set and every safety and goal subset.
    let mut comparisons = 0;
    for encoded in 0..81 {
        let mut code = encoded;
        let mut table = vec![vec![vec![]; 2]; 2];
        for rows in &mut table {
            for row in rows {
                let choice = code % 3;
                code /= 3;
                if choice != 0 {
                    row.push((choice - 1, 0));
                }
            }
        }
        for initial in 1..4 {
            for safe in 0..4 {
                for goal in 0..4 {
                    let value = contract(initial, safe, goal, &table);
                    for horizon in 0..=2 {
                        compare(&value, horizon);
                        comparisons += 1;
                    }
                }
            }
        }
    }
    assert_eq!(comparisons, 11_664);
}

#[test]
fn seeded_nondeterministic_two_observation_history_tables() {
    let (mut wins, mut losses) = (0, 0);
    for seed in 0..128u64 {
        let mut state = seed ^ 0x562a_9310_117a_b7f1;
        let mut draw = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            (state >> 32) as usize
        };
        let initial = 1 + draw() % 7;
        let safe = draw() % 8;
        let goal = draw() % 8;
        let table: Vec<_> = (0..2)
            .map(|_| {
                (0..3)
                    .map(|_| (0..draw() % 3).map(|_| (draw() % 3, draw() % 2)).collect())
                    .collect()
            })
            .collect();
        let value = contract(initial, safe, goal, &table);
        for horizon in 0..=3 {
            if compare(&value, horizon) {
                wins += 1;
            } else {
                losses += 1;
            }
        }
    }
    assert!(wins > 20 && losses > 20);
}

#[test]
fn oracle_cap_is_explicit_and_not_a_losing_result() {
    let value = contract(
        1,
        7,
        4,
        &[
            vec![vec![(1, 0), (2, 1)], vec![(2, 0)], vec![(2, 1)]],
            vec![vec![(0, 0)], vec![(1, 1)], vec![(2, 0)]],
        ],
    );
    let model = compile(&parse_contract(&serde_json::to_vec(&value).unwrap()).unwrap()).unwrap();
    assert_eq!(table_oracle(model.ir(), 4), None); // 3^15 tables exceeds 20,000
}

#[test]
fn concrete_oracle_preserves_same_observation_nondeterminism() {
    // start, left, right, goal, bad. Both finish actions are enabled in both
    // ambiguous worlds, but each is unsafe in one. Hidden state cannot select it.
    let mut table = vec![vec![vec![]; 5]; 3];
    table[0][0] = vec![(1, 0), (2, 0)];
    table[1][1] = vec![(3, 0)];
    table[1][2] = vec![(4, 0)];
    table[2][1] = vec![(4, 0)];
    table[2][2] = vec![(3, 0)];
    assert!(!compare(&contract(1, 15, 8, &table), 2));
    table[0][0][1].1 = 1;
    assert!(compare(&contract(1, 15, 8, &table), 2));
}
