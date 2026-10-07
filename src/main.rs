#![forbid(unsafe_code)]
use contingram::{
    analyze, compile, follow, parse_contract, parse_report, report_bytes, verify, CheckLimits,
    Error, Model, Report, SearchLimits, Status, MAX_CONTRACT_BYTES, MAX_REPORT_BYTES,
};
use serde::Serialize;
use std::io::{Read, Write};

const HELP: &str = "Contingram — bounded recovery-policy compiler and independent verifier\n\nUsage:\n  contingram analyze FILE [--horizon N] [--max-nodes N] [--max-work N]\n  contingram verify MODEL REPORT\n  contingram lower FILE\n  contingram explain MODEL REPORT\n  contingram follow MODEL REPORT [OBSERVATION ...]\n  contingram demo\n  contingram telemetry status|explain\n  contingram --help|--version\n\nExit: 0 success/policy, 1 no policy within horizon, 2 invalid/I/O/check error, 3 UNKNOWN.\nOffline: never executes tools. Defaults: horizon 8, nodes 10000, work 1000000.\nTelemetry/tracking: OFF by default; collection unavailable in v0.1 (no endpoint).\nInspect with telemetry status; disable with CONTINGRAM_TELEMETRY=0.\n";
const TELEMETRY_EXPLANATION: &str = "Contingram supports opt-in telemetry/tracking. OFF by default. This release has no collection endpoint or operator and sends/stores no telemetry, even with CONTINGRAM_TELEMETRY=1. CONTINGRAM_TELEMETRY=0 disables; DO_NOT_TRACK/DNT veto opt-in. The closed event interface permits only version, OS family, architecture, command category, coarse result/error and duration bucket. No identifiers, prompts, source, payloads, secrets, paths or file contents. No sharing/sale. Retention: none. See docs/telemetry.md.";

fn err(code: &'static str, message: &str) -> Error {
    Error {
        code,
        message: message.into(),
    }
}
fn read(path: &str, cap: usize) -> Result<Vec<u8>, Error> {
    // Reject non-files before opening; O_NONBLOCK also prevents a raced FIFO
    // from blocking on Unix. Always recheck the opened handle.
    if !std::fs::metadata(path)
        .map_err(|_| err("io_error", "cannot inspect input file"))?
        .is_file()
    {
        return Err(err("io_error", "input must be a regular file"));
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| err("io_error", "cannot open input file"))?;
    if !file
        .metadata()
        .map_err(|_| err("io_error", "cannot inspect opened file"))?
        .is_file()
    {
        return Err(err("io_error", "input must be a regular file"));
    }
    let mut bytes = Vec::new();
    file.take(cap as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| err("io_error", "cannot read input file"))?;
    if bytes.len() > cap {
        return Err(err("input_limit", "input exceeds byte limit"));
    }
    Ok(bytes)
}
fn model(path: &str) -> Result<Model, Error> {
    compile(&parse_contract(&read(path, MAX_CONTRACT_BYTES)?)?)
}
fn report(path: &str) -> Result<Report, Error> {
    parse_report(&read(path, MAX_REPORT_BYTES)?)
}
fn output(v: &impl Serialize) -> Result<(), Error> {
    let bytes = serde_json::to_vec(v).map_err(|_| err("internal_error", "serialization failed"))?;
    print_bytes(&bytes)
}
fn print_bytes(bytes: &[u8]) -> Result<(), Error> {
    let mut stdout = std::io::stdout().lock();
    stdout
        .write_all(bytes)
        .and_then(|_| stdout.write_all(b"\n"))
        .map_err(|_| err("io_error", "cannot write stdout"))
}
fn run(args: &[String]) -> Result<u8, Error> {
    let usage = || err("usage", "invalid arguments; use --help");
    let Some(command) = args.first().map(String::as_str) else {
        print_bytes(HELP.as_bytes())?;
        return Ok(0);
    };
    match command {
        "--help" | "help" if args.len() == 1 => print_bytes(HELP.as_bytes())?,
        "--version" if args.len() == 1 => {
            print_bytes(concat!("contingram ", env!("CARGO_PKG_VERSION")).as_bytes())?
        }
        "telemetry" if args.len() == 2 => match args[1].as_str() {
            "status" => output(&contingram::telemetry::status())?,
            "explain" => print_bytes(TELEMETRY_EXPLANATION.as_bytes())?,
            _ => return Err(usage()),
        },
        "analyze" if args.len() >= 2 => {
            let mut limits = SearchLimits::default();
            let mut seen = std::collections::BTreeSet::new();
            let rest = &args[2..];
            if rest.len() % 2 != 0 {
                return Err(usage());
            }
            for option in rest.chunks_exact(2) {
                if !seen.insert(&option[0]) || !option[1].bytes().all(|c| c.is_ascii_digit()) {
                    return Err(usage());
                }
                match option[0].as_str() {
                    "--horizon" => limits.horizon = option[1].parse().map_err(|_| usage())?,
                    "--max-nodes" => limits.max_nodes = option[1].parse().map_err(|_| usage())?,
                    "--max-work" => limits.max_work = option[1].parse().map_err(|_| usage())?,
                    _ => return Err(usage()),
                }
            }
            let result = analyze(&model(&args[1])?, limits)?;
            // Artifact framing must fit the same cap accepted by parse_report.
            std::io::stdout()
                .lock()
                .write_all(&report_bytes(&result)?)
                .map_err(|_| err("io_error", "cannot write stdout"))?;
            return Ok(match result.status {
                Status::PolicyFound => 0,
                Status::NoPolicyWithinHorizon => 1,
                Status::Unknown => 3,
            });
        }
        "lower" if args.len() == 2 => {
            let m = model(&args[1])?;
            output(
                &serde_json::json!({"schema_version":1,"model_sha256":m.digest(),"ir":m.ir(),"origins":m.origins()}),
            )?;
        }
        "verify" | "explain" if args.len() == 3 => {
            let m = model(&args[1])?;
            let r = report(&args[2])?;
            let checked = verify(&m, &r, CheckLimits::default())?;
            if command == "verify" {
                output(&checked)?;
            } else {
                let mut text = format!(
                    "Checked {:?} within {} decisions; model {}\n",
                    r.status,
                    r.horizon,
                    m.digest()
                );
                for (id, node) in r
                    .certificate
                    .as_ref()
                    .expect("verified")
                    .nodes
                    .iter()
                    .enumerate()
                {
                    let line = format!(
                        "node {id}: {}\n",
                        serde_json::to_string(node)
                            .map_err(|_| err("internal_error", "serialization failed"))?
                    );
                    if line.len() > MAX_REPORT_BYTES - text.len() {
                        return Err(err("output_limit", "explanation exceeds 8 MiB"));
                    }
                    text.push_str(&line);
                }
                std::io::stdout()
                    .lock()
                    .write_all(text.as_bytes())
                    .map_err(|_| err("io_error", "cannot write stdout"))?;
            }
        }
        "follow" if args.len() >= 3 => output(&follow(
            &model(&args[1])?,
            &report(&args[2])?,
            &args[3..],
            CheckLimits::default(),
        )?)?,
        "demo" if args.len() == 1 => {
            let mut cases = Vec::new();
            for (name, bytes, expected) in [
                (
                    "final-receipt",
                    &include_bytes!("../examples/contracts/receipt.json")[..],
                    Status::PolicyFound,
                ),
                (
                    "stale-read",
                    &include_bytes!("../examples/contracts/stale-read.json")[..],
                    Status::NoPolicyWithinHorizon,
                ),
            ] {
                let m = compile(&parse_contract(bytes)?)?;
                let r = analyze(
                    &m,
                    SearchLimits {
                        horizon: 2,
                        ..Default::default()
                    },
                )?;
                if r.status != expected {
                    return Err(err("internal_error", "demo expectation failed"));
                }
                verify(&m, &r, CheckLimits::default())?;
                cases.push(serde_json::json!({"case":name,"status":r.status,"horizon":r.horizon,"model_sha256":m.digest()}));
            }
            output(&cases)?;
        }
        _ => return Err(usage()),
    }
    Ok(0)
}
fn main() -> std::process::ExitCode {
    let args: Result<Vec<String>, _> = std::env::args_os()
        .skip(1)
        .map(|a| a.into_string())
        .collect();
    let args = match args {
        Ok(args) => args,
        Err(_) => {
            let _ = serde_json::to_writer(
                std::io::stderr().lock(),
                &err("usage", "arguments must be valid UTF-8"),
            );
            let _ = writeln!(std::io::stderr());
            return 2.into();
        }
    };
    let start = std::time::Instant::now();
    let result = run(&args);
    // Telemetry is isolated from model semantics and command success. No transport
    // exists in this release; the event has no access to paths or diagnostics.
    use contingram::telemetry::{Event, Feature, Outcome};
    let feature = args.first().and_then(|a| match a.as_str() {
        "analyze" => Some(Feature::Analyze),
        "verify" => Some(Feature::Verify),
        "lower" => Some(Feature::Lower),
        "explain" => Some(Feature::Explain),
        "follow" => Some(Feature::Follow),
        "demo" => Some(Feature::Demo),
        _ => None,
    });
    if contingram::telemetry::status().collection_enabled {
        if let Some(feature) = feature {
            let outcome = match &result {
                Ok(1) => Outcome::NoPolicyWithinHorizon,
                Ok(3) => Outcome::Unknown,
                Ok(_) => Outcome::Success,
                Err(e) if e.code == "io_error" => Outcome::IoError,
                Err(_) => Outcome::InvalidInput,
            };
            let _ = contingram::telemetry::try_emit(&Event::new(feature, outcome, start.elapsed()));
        }
    }
    match result {
        Ok(code) => code.into(),
        Err(e) => {
            let _ = serde_json::to_writer(std::io::stderr().lock(), &e);
            let _ = writeln!(std::io::stderr());
            2.into()
        }
    }
}
