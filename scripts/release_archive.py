#!/usr/bin/env python3
"""Build an explicit, deterministic archive inventory for a tagged binary."""
import argparse
import gzip
import io
import pathlib
import re
import subprocess
import tarfile
import zipfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=["x86_64-unknown-linux-gnu",
                        "aarch64-apple-darwin", "x86_64-pc-windows-msvc"])
    parser.add_argument("--version", required=True)
    parser.add_argument("--notices", type=pathlib.Path, default=pathlib.Path("docs/third-party-licenses.md"))
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+", args.version):
        raise SystemExit("invalid release version")
    binary = "contingram.exe" if "windows" in args.target else "contingram"
    sources = {binary: pathlib.Path("target") / args.target / "release" / binary,
               **{name: pathlib.Path(name) for name in ["LICENSE", "NOTICE", "README.md", "TRADEMARKS.md"]}}
    sources["THIRD_PARTY_NOTICES.md"] = args.notices
    readme = f"""# Contingram {args.version}

Bounded recovery-policy compiler and independent artifact verifier.
Created and maintained by [Nima Khaki](https://github.com/Nima0101).

Run `{binary} demo`, `{binary} --help`, and `{binary} telemetry status`.
On Unix, prefix the executable with `./` when running from this directory.

[Source, examples, and full quickstart](https://github.com/Nima0101/contingram/tree/v{args.version})

Telemetry and privacy: Contingram supports opt-in telemetry interfaces, but collection
is disabled in this release because no first-party endpoint is configured. No network
telemetry is sent. `CONTINGRAM_TELEMETRY=0` is the explicit kill switch.

See LICENSE, NOTICE, TRADEMARKS.md, and THIRD_PARTY_NOTICES.md for code licensing,
attribution, branding, and dependency/runtime notices.
Verify the downloaded archive against SHA256SUMS from the same release, then run
`gh attestation verify ARCHIVE --repo Nima0101/contingram` to verify build provenance.
These checks establish origin and integrity, not the truth of a recovery model.
""".encode()
    timestamp = int(subprocess.check_output(["git", "show", "-s", "--format=%ct", "HEAD"]))
    prefix = f"contingram-{args.version}-{args.target}"
    output = pathlib.Path("dist")
    output.mkdir(exist_ok=True)
    if "windows" in args.target:
        with zipfile.ZipFile(output / (prefix + ".zip"), "w", zipfile.ZIP_DEFLATED) as archive:
            for name, source in sorted(sources.items()):
                info = zipfile.ZipInfo(prefix + "/" + name, (1980, 1, 1, 0, 0, 0))
                info.external_attr = 0o100644 << 16
                archive.writestr(info, readme if name == "README.md" else source.read_bytes(),
                                 compress_type=zipfile.ZIP_DEFLATED)
    else:
        with (output / (prefix + ".tar.gz")).open("wb") as raw:
            with gzip.GzipFile(fileobj=raw, mode="wb", filename="", mtime=0) as compressed:
                with tarfile.open(fileobj=compressed, mode="w") as archive:
                    for name, source in sorted(sources.items()):
                        data = readme if name == "README.md" else source.read_bytes()
                        info = tarfile.TarInfo(prefix + "/" + name)
                        info.size, info.mtime = len(data), timestamp
                        info.mode = 0o755 if name == binary else 0o644
                        archive.addfile(info, io.BytesIO(data))


if __name__ == "__main__":
    main()
