#![no_main]
use contingram::{compile, parse_contract, parse_report, verify, CheckLimits};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|bytes: &[u8]| {
    if bytes.len() > 65536 {
        return;
    }
    if let Ok(report) = parse_report(bytes) {
        let model = compile(
            &parse_contract(include_bytes!("../../examples/contracts/receipt.json")).unwrap(),
        )
        .unwrap();
        let _ = verify(
            &model,
            &report,
            CheckLimits {
                max_nodes: 128,
                max_work: 10000,
            },
        );
    }
});
