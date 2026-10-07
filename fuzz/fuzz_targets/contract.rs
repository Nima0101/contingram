#![no_main]
use contingram::{
    analyze, compile, parse_contract, parse_report, report_bytes, verify, CheckLimits,
    SearchLimits, Status,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() > 65536 {
        return;
    }
    if let Ok(contract) = parse_contract(bytes) {
        if let Ok(model) = compile(&contract) {
            let report = analyze(
                &model,
                SearchLimits {
                    horizon: 4,
                    max_nodes: 128,
                    max_work: 10000,
                },
            )
            .unwrap();
            let encoded = report_bytes(&report).unwrap();
            let reparsed = parse_report(&encoded).unwrap();
            if report.status != Status::Unknown {
                verify(&model, &reparsed, CheckLimits::default()).unwrap();
            }
            assert_eq!(encoded, report_bytes(&reparsed).unwrap());
        }
    }
});
