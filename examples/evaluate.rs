//! Reproduce the declared in-process performance experiment. No network or tools run.
use contingram::{
    analyze, compile, parse_contract, report_bytes, verify, CheckLimits, Model, SearchLimits,
    Status,
};
use serde_json::{json, Value};
use std::hint::black_box;
use std::time::Instant;

const WARMUPS: usize = 3;
const SAMPLES: usize = 21;
fn eq(var: &str, value: &str) -> Value {
    json!({"op":"eq","var":var,"value":value})
}
fn family(bits: usize, variant: &str) -> Model {
    let mut variables = vec![
        json!({"name":"done","values":["no","yes"]}),
        json!({"name":"mode","values":["left","right"]}),
    ];
    variables
        .extend((0..bits - 2).map(|b| json!({"name":format!("n{b}"),"values":["zero","one"]})));
    let mut observations = Vec::new();
    for mode in ["left", "right"] {
        for noise in 0..if variant == "branching" {
            1 << (bits - 2)
        } else {
            1
        } {
            let mut guards = vec![eq("mode", mode)];
            if variant == "branching" {
                guards.extend((0..bits - 2).map(|b| {
                    eq(
                        &format!("n{b}"),
                        if noise & (1 << b) == 0 { "zero" } else { "one" },
                    )
                }));
            }
            observations.push(json!({"id":format!("read-{mode}-{noise}"),"when":{"op":"all","args":guards},"set":[],"observe":if variant == "unresolved" { "same".into() } else { format!("{mode}-{noise}") }}));
        }
    }
    let mut actions = vec![json!({"id":"inspect","enabled":{"op":"true"},"outcomes":observations})];
    for mode in ["left", "right"] {
        actions.push(json!({"id":format!("finish-{mode}"),"enabled":eq("mode",mode),"outcomes":[{"id":"complete","when":{"op":"true"},"set":[{"var":"done","value":"yes"}],"observe":"ack"}]}));
    }
    let value = json!({"schema_version":1,"name":format!("evaluation-{variant}"),"variables":variables,"initial":eq("done","no"),"safe":{"op":"true"},"goal":eq("done","yes"),"actions":actions});
    compile(&parse_contract(&serde_json::to_vec(&value).unwrap()).unwrap()).unwrap()
}
fn summary(mut samples: Vec<u128>) -> Value {
    samples.sort_unstable();
    json!({"unit":"nanoseconds","min":samples[0],"median":samples[SAMPLES/2],"p95":samples[19],"max":samples[SAMPLES-1]})
}
fn measure(name: &str, model: &Model, limits: SearchLimits) -> Value {
    let mut synth = Vec::new();
    let mut check = Vec::new();
    let mut total = Vec::new();
    let mut reference = None;
    let mut last = None;
    let mut check_stats = None;
    let mut unknown_samples = 0;
    for sample in 0..WARMUPS + SAMPLES {
        let start = Instant::now();
        let report = black_box(analyze(black_box(model), limits).unwrap());
        let synthesis_elapsed = start.elapsed().as_nanos();
        let checking_start = Instant::now();
        let checked = if report.status == Status::Unknown {
            None
        } else {
            Some(verify(black_box(model), black_box(&report), CheckLimits::default()).unwrap())
        };
        let checking_elapsed = checking_start.elapsed().as_nanos();
        let total_elapsed = start.elapsed().as_nanos();
        let bytes = report_bytes(&report).unwrap();
        if let Some(previous) = &reference {
            assert_eq!(previous, &bytes, "nondeterministic report");
        } else {
            reference = Some(bytes);
        }
        if sample >= WARMUPS {
            synth.push(synthesis_elapsed);
            total.push(total_elapsed);
            if let Some(checked) = checked {
                check.push(checking_elapsed);
                check_stats = Some(checked.stats);
            } else {
                unknown_samples += 1;
            }
        }
        last = Some(report);
    }
    let report = last.unwrap();
    json!({"family":name,"worlds":model.ir().states.len(),"initial_worlds":model.ir().initial.len(),"actions":model.ir().actions.len(),"edges":model.ir().actions.iter().map(|a|a.edges.len()).sum::<usize>(),"horizon":limits.horizon,"max_nodes":limits.max_nodes,"max_work":limits.max_work,"status":report.status,"solver_stats":report.stats,"checker_stats":check_stats,"artifact_bytes":reference.unwrap().len(),"unknown_samples":unknown_samples,"limit":report.limit,"synthesis":summary(synth),"verification":if check.is_empty(){Value::Null}else{summary(check)},"combined":summary(total)})
}
fn main() {
    let mut results = Vec::new();
    for bits in 2..=8 {
        for variant in ["ambiguity", "branching", "unresolved", "cutoff"] {
            let model = family(bits, variant);
            let limits = SearchLimits {
                horizon: if variant == "unresolved" { 4 } else { 2 },
                max_work: if variant == "cutoff" { 1 } else { 1_000_000 },
                ..Default::default()
            };
            let result = measure(variant, &model, limits);
            let expected = match variant {
                "unresolved" => "no_policy_within_horizon",
                "cutoff" => "unknown",
                _ => "policy_found",
            };
            assert_eq!(result["status"], expected);
            results.push(result);
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/heldout");
    for name in [
        "stale-receipt-refresh",
        "identity-expired-ledger",
        "partial-stage-status",
    ] {
        let model = compile(
            &parse_contract(&std::fs::read(root.join(format!("{name}.json"))).unwrap()).unwrap(),
        )
        .unwrap();
        results.push(measure(
            name,
            &model,
            SearchLimits {
                horizon: 3,
                ..Default::default()
            },
        ));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"schema_version":1,"warmups":WARMUPS,"samples":SAMPLES,"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"debug_assertions":cfg!(debug_assertions),"timing_scope":"in-process analysis and independent verification; excludes compilation, file IO and report serialization","results":results})).unwrap());
}
