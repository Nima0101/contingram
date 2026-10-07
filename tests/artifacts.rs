use contingram::*;
#[test]
fn portable_frozen_artifacts_check_in_a_fresh_process_or_library() {
    for (model, artifact, status) in [
        (
            &include_bytes!("../examples/contracts/receipt.json")[..],
            &include_bytes!("../examples/artifacts/receipt-policy.json")[..],
            Status::PolicyFound,
        ),
        (
            &include_bytes!("../examples/contracts/stale-read.json")[..],
            &include_bytes!("../examples/artifacts/stale-refutation.json")[..],
            Status::NoPolicyWithinHorizon,
        ),
    ] {
        let m = compile(&parse_contract(model).unwrap()).unwrap();
        let r = parse_report(artifact).unwrap();
        assert_eq!(
            verify(&m, &r, CheckLimits::default()).unwrap().status,
            status
        );
        let regenerated = analyze(
            &m,
            SearchLimits {
                horizon: r.horizon,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(
            report_bytes(&r).unwrap(),
            report_bytes(&regenerated).unwrap()
        );
    }
    let unknown = parse_report(include_bytes!(
        "../examples/artifacts/resource-unknown.json"
    ))
    .unwrap();
    assert_eq!(unknown.status, Status::Unknown);
    assert!(unknown.certificate.is_none());
    let m = compile(&parse_contract(include_bytes!("../examples/contracts/receipt.json")).unwrap())
        .unwrap();
    assert_eq!(
        verify(&m, &unknown, CheckLimits::default())
            .unwrap_err()
            .code,
        "no_certificate"
    );
}
