#!/usr/bin/env python3
"""Fail closed on private files, obvious secrets, local paths, and broken links.

This complements gitleaks; it is not a claim that pattern matching proves secrecy.
Before git initialization only the explicit candidate allowlist is inspected.
After initialization every tracked file must belong to that allowlist.
"""
import argparse
import io
import json
import os
import pathlib
import re
import stat
import subprocess
import tarfile
import tempfile
import urllib.parse
import zipfile

ROOT_FILES = {
    "Cargo.toml", "Cargo.lock", "README.md", "LICENSE", "NOTICE", "SECURITY.md",
    "TRADEMARKS.md", "CONTRIBUTING.md", "CODE_OF_CONDUCT.md", "CHANGELOG.md",
    "ROADMAP.md", "GOVERNANCE.md", ".gitignore", ".gitattributes", ".editorconfig", "deny.toml",
}
PUBLIC_DIRS = {"src", "tests", "examples", "docs", "benchmarks", ".github"}
SCRIPTS = {"public_gate.py", "package_consumer.py", "sbom.py", "release_archive.py",
           "evaluate.py", "benchmark.py", "third_party_licenses.py"}
FUZZ_FILES = {"Cargo.toml", "Cargo.lock", "README.md", ".gitignore"}


def manifest_errors(text):
    # The public manifest intentionally uses a JSON-compatible TOML string array.
    # Fail closed if its syntax changes; Cargo independently validates full TOML.
    arrays = re.findall(r"(?m)^include\s*=\s*(\[[^\]]*\])", text)
    if len(arrays) != 1:
        return ["Cargo.toml: require one explicit anchored package include array"]
    try:
        patterns = json.loads(arrays[0])
    except ValueError:
        return ["Cargo.toml: package include must be a JSON-compatible string array"]
    if not patterns or any(not isinstance(p, str) or not p.startswith("/") or ".." in p
                           for p in patterns):
        return ["Cargo.toml: every package include pattern must start with / and stay rooted"]
    return []


def allowed(name):
    p = pathlib.PurePosixPath(name)
    if p.is_absolute() or ".." in p.parts or "__pycache__" in p.parts:
        return False
    if len(p.parts) == 1:
        return name in ROOT_FILES
    if p.parts[0] in PUBLIC_DIRS:
        return True
    if p.parts[0] == "scripts":
        return len(p.parts) == 2 and p.name in SCRIPTS
    if p.parts[0] == "fuzz":
        return (len(p.parts) == 2 and p.name in FUZZ_FILES) or (
            len(p.parts) == 3 and p.parts[1] == "fuzz_targets" and p.suffix == ".rs")
    return False


def scan_bytes(name, data):
    errors = []
    # Split signatures keep the scanner's own source free of matching examples.
    banned = [b"Bi" + b"stam", b"Antagnings" + b"data", b"Ceder" + b"dalen",
              b"/Users" + b"/", b"/home" + b"/", b":\\" + b"Users\\",
              b"OpenSource-AI-" + b"Flagship"]
    if any(term.lower() in data.lower() for term in banned):
        errors.append(f"{name}: private identifier or absolute user path")
    patterns = [rb"gh[pousr]_" + rb"[A-Za-z0-9]{30,}",
                rb"github_pat_" + rb"[A-Za-z0-9_]{40,}",
                rb"AKIA" + rb"[A-Z0-9]{16}",
                rb"-----BEGIN " + rb"(?:RSA |EC |OPENSSH )?PRIVATE KEY-----"]
    if any(re.search(pattern, data) for pattern in patterns):
        errors.append(f"{name}: credential signature")
    return errors


def inventory(root):
    result = subprocess.run(["git", "ls-files", "-z"], cwd=root,
                            capture_output=True, check=False)
    if result.returncode == 0 and result.stdout:
        return sorted(x.decode() for x in result.stdout.split(b"\0") if x)
    candidates = []
    for item in root.iterdir():
        if item.name in PUBLIC_DIRS | {"scripts", "fuzz"} and item.is_dir():
            for directory, directories, files in os.walk(item, followlinks=False):
                if item.name == "fuzz":
                    directories[:] = [d for d in directories if d == "fuzz_targets"]
                else:
                    directories[:] = [d for d in directories if d not in
                                      {"target", "__pycache__", ".git", "node_modules"}]
                for name in files:
                    path = pathlib.Path(directory) / name
                    relative = path.relative_to(root).as_posix()
                    if allowed(relative):
                        candidates.append(relative)
        elif item.is_file() and allowed(item.name):
            candidates.append(item.name)
    return sorted(candidates)


def links(root, name, text):
    errors = []
    for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", text):
        target = target.split(' "', 1)[0].strip("<>")
        parsed = urllib.parse.urlsplit(target)
        if parsed.scheme or parsed.netloc:
            continue
        path = ((root / pathlib.Path(name).parent / urllib.parse.unquote(parsed.path))
                if parsed.path else (root / name)).resolve()
        if not path.is_relative_to(root.resolve()) or not path.exists():
            errors.append(f"{name}: unresolved local link {target}")
        elif parsed.fragment and path.suffix == ".md":
            document = path.read_text(encoding="utf-8")
            anchors, counts = set(), {}
            for heading in re.findall(r"^#{1,6}\s+(.+?)\s*#*\s*$", document, re.M):
                slug = re.sub(r"[^\w -]", "", heading.lower()).replace(" ", "-")
                count = counts.get(slug, 0)
                anchors.add(slug + (f"-{count}" if count else ""))
                counts[slug] = count + 1
            anchors.update(re.findall(r'(?:id|name)=["\']([^"\']+)["\']', document))
            if urllib.parse.unquote(parsed.fragment) not in anchors:
                errors.append(f"{name}: unresolved local anchor {target}")
    return errors


def check_archive(path):
    errors = []
    entries = []
    if zipfile.is_zipfile(path):
        with zipfile.ZipFile(path) as archive:
            for item in archive.infolist():
                if item.is_dir():
                    continue
                mode = item.external_attr >> 16
                if stat.S_ISLNK(mode) or (stat.S_IFMT(mode) and not stat.S_ISREG(mode)):
                    errors.append(f"{item.filename}: archive links/special files forbidden")
                    continue
                entries.append((item.filename, archive.read(item)))
    else:
        with tarfile.open(path) as archive:
            for member in archive.getmembers():
                if member.isdir():
                    continue
                if not member.isfile():
                    errors.append(f"{member.name}: archive links/special files forbidden")
                    continue
                entries.append((member.name, archive.extractfile(member).read()))
    native = any(pathlib.PurePosixPath(name).name in {"contingram", "contingram.exe"}
                 for name, _ in entries)
    if len({name for name, _ in entries}) != len(entries):
        errors.append("duplicate archive members forbidden")
    native_files = {"LICENSE", "NOTICE", "README.md", "TRADEMARKS.md", "THIRD_PARTY_NOTICES.md"}
    native_files.add("contingram.exe" if str(path).endswith(".zip") else "contingram")
    if native and {pathlib.PurePosixPath(name).name for name, _ in entries} != native_files:
        errors.append("native archive inventory must contain exactly binary and required notices")
    prefixes = set()
    for name, data in entries:
        p = pathlib.PurePosixPath(name)
        if p.is_absolute() or ".." in p.parts or len(p.parts) < 2:
            errors.append(f"{name}: invalid archive layout")
            continue
        relative = pathlib.PurePosixPath(*p.parts[1:]).as_posix()
        prefixes.add(p.parts[0])
        if native and (len(p.parts) != 2 or relative not in native_files):
            errors.append(f"{name}: unexpected native archive member")
        if not native and relative not in {"Cargo.toml.orig", ".cargo_vcs_info.json"} and not allowed(relative):
            errors.append(f"{name}: archive file outside allowlist")
        errors.extend(scan_bytes(name, data))
    if len(prefixes) != 1:
        errors.append("archive requires one package root")
    if not entries:
        errors.append("empty archive")
    return errors


def self_test():
    assert manifest_errors('include = ["README.md", "/src/**"]')
    assert not manifest_errors('include = ["/README.md", "/src/**"]')
    assert allowed("src/lib.rs")
    assert not allowed(".mission/evidence.md")
    assert not allowed("scripts/start_astra_high_flagship.sh")
    assert not allowed("src/../.mission/evidence.md")
    assert not scan_bytes("src/a.rs", b"safe synthetic content")
    assert scan_bytes("src/a.rs", b"/Users" + b"/someone/private")
    assert scan_bytes("src/a.rs", b"ghp_" + b"A" * 36)
    with tempfile.TemporaryDirectory(prefix="contingram-gate-test-") as directory:
        path = pathlib.Path(directory) / "test.tar.gz"
        with tarfile.open(path, "w:gz") as archive:
            member = tarfile.TarInfo("package/src/lib.rs")
            member.size = 4
            archive.addfile(member, io.BytesIO(b"safe"))
        assert not check_archive(path)
        with tarfile.open(path, "w:gz") as archive:
            member = tarfile.TarInfo("package/../../escape")
            member.size = 3
            archive.addfile(member, io.BytesIO(b"bad"))
        assert check_archive(path)
        with tarfile.open(path, "w:gz") as archive:
            member = tarfile.TarInfo("package/src/lib.rs")
            member.type = tarfile.SYMTYPE
            member.linkname = "/etc/passwd"
            archive.addfile(member)
        assert check_archive(path)
        path = pathlib.Path(directory) / "test.zip"
        with zipfile.ZipFile(path, "w") as archive:
            member = zipfile.ZipInfo("package/src/lib.rs")
            member.external_attr = 0o120777 << 16
            archive.writestr(member, "/etc/passwd")
        assert check_archive(path)
    print("Public gate regression self-tests passed.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--archive", type=pathlib.Path)
    parser.add_argument("--inventory", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        self_test()
        return 0
    if args.archive:
        errors = check_archive(args.archive)
    else:
        names = inventory(args.root)
        if args.inventory:
            print("\n".join(names))
            return 0
        errors = []
        for name in sorted(ROOT_FILES - {".gitattributes"}):
            if not (args.root / name).is_file():
                errors.append(f"missing required public file: {name}")
        for name in names:
            path = args.root / name
            if not allowed(name):
                errors.append(f"{name}: tracked file outside publication allowlist")
            if path.is_symlink() or not path.is_file():
                errors.append(f"{name}: non-regular public file")
                continue
            data = path.read_bytes()
            errors.extend(scan_bytes(name, data))
            if name == "Cargo.toml":
                errors.extend(manifest_errors(data.decode("utf-8")))
            if path.suffix == ".md":
                errors.extend(links(args.root, name, data.decode("utf-8")))
    for error in errors:
        print(error)
    if errors:
        return 1
    print("Public file, path, credential-signature, and local-link checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
