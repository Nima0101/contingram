#!/usr/bin/env python3
"""Build the distributable crate, then consume its library and CLI in isolation."""
import os
import pathlib
import subprocess
import tarfile
import tempfile
from public_gate import allowed, manifest_errors

ROOT = pathlib.Path(__file__).resolve().parents[1]


def run(*args, cwd=ROOT, env=None):
    subprocess.run(args, cwd=cwd, env=env, check=True)


def main():
    errors = manifest_errors((ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    if errors:
        raise SystemExit("\n".join(errors))
    listing = subprocess.check_output(
        ["cargo", "package", "--list", "--locked", "--allow-dirty"], cwd=ROOT, text=True, encoding="utf-8")
    for name in listing.splitlines():
        if name not in {"Cargo.toml.orig", ".cargo_vcs_info.json"} and not allowed(name):
            raise SystemExit("package inventory outside publication allowlist: " + name)
    run("cargo", "package", "--locked", "--allow-dirty")
    target = pathlib.Path(os.environ.get("CARGO_TARGET_DIR", "target"))
    if not target.is_absolute():
        target = ROOT / target
    crates = sorted((target / "package").glob("contingram-*.crate"))
    if len(crates) != 1:
        raise SystemExit("expected exactly one package archive; clean stale package output")
    run("python3", str(ROOT / "scripts/public_gate.py"), "--archive", str(crates[0]))
    with tempfile.TemporaryDirectory(prefix="contingram-consumer-") as temporary:
        base = pathlib.Path(temporary)
        with tarfile.open(crates[0]) as archive:
            for item in archive.getmembers():
                name = pathlib.PurePosixPath(item.name)
                if name.is_absolute() or ".." in name.parts or not (item.isfile() or item.isdir()):
                    raise SystemExit("unsafe crate archive")
                destination = base.joinpath(*name.parts)
                if not destination.resolve().is_relative_to(base.resolve()):
                    raise SystemExit("crate member escapes temporary directory")
                if item.isdir():
                    destination.mkdir(parents=True, exist_ok=True)
                else:
                    destination.parent.mkdir(parents=True, exist_ok=True)
                    destination.write_bytes(archive.extractfile(item).read())
        package = next(base.glob("contingram-*"))
        consumer = base / "consumer"
        (consumer / "src").mkdir(parents=True)
        # JSON quoted strings are valid TOML basic strings for these paths.
        import json
        (consumer / "Cargo.toml").write_text(
            '[package]\nname="package-consumer"\nversion="0.0.0"\nedition="2021"\n'
            '[dependencies]\ncontingram={path=' + json.dumps(str(package)) + '}\n', encoding="utf-8")
        (consumer / "src/main.rs").write_text('''
use contingram::{analyze, compile, parse_contract, verify, CheckLimits, SearchLimits, Status};
fn main() {
    let bytes = std::fs::read(std::env::args().nth(1).unwrap()).unwrap();
    let model = compile(&parse_contract(&bytes).unwrap()).unwrap();
    let report = analyze(&model, SearchLimits::default()).unwrap();
    assert_eq!(report.status, Status::PolicyFound);
    verify(&model, &report, CheckLimits::default()).unwrap();
}
''', encoding="utf-8")
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(base / "target")
        run("cargo", "run", "--manifest-path", str(consumer / "Cargo.toml"), "--",
            str(package / "examples/contracts/receipt.json"), cwd=base, env=env)
        run("cargo", "install", "--path", str(package), "--locked", "--root", str(base / "install"),
            cwd=base, env=env)
        binary = base / "install/bin" / ("contingram.exe" if os.name == "nt" else "contingram")
        run(str(binary), "demo", cwd=base, env=env)
    print("Packaged library and installed CLI consumer checks passed.")


if __name__ == "__main__":
    main()
