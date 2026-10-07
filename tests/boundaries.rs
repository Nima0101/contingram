use contingram::*;
use serde_json::{json, Value};
fn contract(domains: &[usize], outcomes: usize) -> Value {
    json!({"schema_version":1,"name":"boundaries","variables":domains.iter().enumerate().map(|(i,n)|json!({"name":format!("v{i}"),"values":(0..*n).map(|j|format!("s{j}")).collect::<Vec<_>>()})).collect::<Vec<_>>(),"initial":{"op":"true"},"safe":{"op":"true"},"goal":{"op":"true"},"actions":[{"id":"a","enabled":{"op":"true"},"outcomes":(0..outcomes).map(|i|json!({"id":format!("o{i}"),"when":{"op":"true"},"set":[],"observe":"ok"})).collect::<Vec<_>>()}]})
}
fn parse(v: &Value) -> Result<Contract, Error> {
    parse_contract(&serde_json::to_vec(v).unwrap())
}
#[test]
fn product_outcome_and_expanded_edge_boundaries() {
    assert_eq!(
        compile(&parse(&contract(&[16, 16], 1)).unwrap())
            .unwrap()
            .ir()
            .states
            .len(),
        256
    );
    assert_eq!(
        parse(&contract(&[16, 16, 2], 1)).unwrap_err().code,
        "model_limit"
    );
    let exact = compile(&parse(&contract(&[16, 2], 512)).unwrap()).unwrap();
    assert_eq!(exact.origins().len(), 16384);
    assert_eq!(
        parse(&contract(&[16, 2], 513)).unwrap_err().code,
        "model_limit"
    );
    assert_eq!(
        compile(&parse(&contract(&[11, 3], 512)).unwrap())
            .unwrap_err()
            .code,
        "model_limit"
    );
}
#[test]
fn compile_work_cannot_exceed_bound() {
    let mut v = contract(&[16, 16], 1);
    v["safe"] = json!({"op":"all","args":vec![json!({"op":"true"});4090]});
    assert_eq!(
        compile(&parse(&v).unwrap()).unwrap_err().code,
        "compile_limit"
    );
}
#[test]
fn byte_caps_accept_exact_and_reject_plus_one() {
    let mut bytes = serde_json::to_vec(&contract(&[1], 1)).unwrap();
    bytes.resize(MAX_CONTRACT_BYTES, b' ');
    parse_contract(&bytes).unwrap();
    bytes.push(b' ');
    assert_eq!(parse_contract(&bytes).unwrap_err().code, "input_limit");
    let m = compile(&parse(&contract(&[1], 1)).unwrap()).unwrap();
    let mut r = analyze(&m, SearchLimits::default()).unwrap();
    let mut bytes = report_bytes(&r).unwrap();
    bytes.resize(MAX_REPORT_BYTES, b' ');
    parse_report(&bytes).unwrap();
    bytes.push(b' ');
    assert_eq!(parse_report(&bytes).unwrap_err().code, "input_limit");
    r.model_sha256 = "x".repeat(MAX_REPORT_BYTES);
    assert_eq!(report_bytes(&r).unwrap_err().code, "output_limit");
}
#[test]
fn predicate_depth_and_count_exact_boundaries() {
    let mut v = contract(&[1], 1);
    for depth in 1..=17 {
        let mut e = json!({"op":"true"});
        for _ in 1..depth {
            e = json!({"op":"not","arg":e});
        }
        v["safe"] = e;
        assert_eq!(parse(&v).is_ok(), depth <= 16);
    }
}
