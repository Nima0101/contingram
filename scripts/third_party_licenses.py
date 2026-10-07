#!/usr/bin/env python3
"""Preserve locked crate and selected Rust standard-library license notices."""
import argparse
import hashlib
import html.parser
import json
import pathlib
import subprocess


class NoticeText(html.parser.HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.parts = []

    def handle_data(self, data):
        self.parts.append(data)

    def handle_starttag(self, tag, attributes):
        if tag in {"p", "h1", "h2", "h3", "li", "pre", "summary"}:
            self.parts.append("\n")
        if tag == "a":
            for name, value in attributes:
                if name == "href" and value and not value.startswith("#"):
                    self.parts.append(" [" + value + "] ")


def fenced(text):
    fence = "`" * max(4, max((len(p) for p in text.split() if set(p) == {"`"}), default=0) + 1)
    return fence + "text\n" + text.rstrip() + "\n" + fence + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--rust-sysroot", type=pathlib.Path)
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--locked", "--format-version", "1"]))
    sysroot = args.rust_sysroot or pathlib.Path(subprocess.check_output(
        ["rustc", "--print", "sysroot"], text=True, encoding="utf-8").strip())
    rustc = sysroot / "bin" / ("rustc.exe" if (sysroot / "bin/rustc.exe").exists() else "rustc")
    version = subprocess.check_output([str(rustc), "--version"], text=True, encoding="utf-8").strip()
    rust_docs = sysroot / "share/doc/rust"
    copyright_file = rust_docs / "COPYRIGHT-library.html"
    if not copyright_file.is_file():
        raise SystemExit("selected Rust toolchain must provide official COPYRIGHT-library.html")
    sections = ["# Third-party license notices\n",
        "Generated from `cargo metadata --locked` and the selected official Rust toolchain. "
        "This conservative inventory includes build dependencies and all resolved target variants; "
        "it does not assert that every listed component is linked into every binary. "
        "Contingram's own code remains Apache-2.0. Upstream notices below retain their original terms.\n",
        "Regenerate with `python3 scripts/third_party_licenses.py --output docs/third-party-licenses.md` "
        "using an official rustup toolchain. Release builds regenerate these notices for the "
        "compiler actually used and bundle them as `THIRD_PARTY_NOTICES.md`.\n",
        "Toolchain snapshot: `" + version + "`.\n",
        "Rust licensing sources: [Rust copyright policy](https://github.com/rust-lang/rust/blob/main/COPYRIGHT), "
        "[compiler builtins](https://github.com/rust-lang/compiler-builtins).\n",
        "## Locked Cargo dependencies\n"]
    texts = {}
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if not package["source"]:
            continue
        directory = pathlib.Path(package["manifest_path"]).parent
        files = {p for p in directory.iterdir() if p.is_file() and p.name.upper().startswith(
            ("LICENSE", "COPYING", "COPYRIGHT", "NOTICE", "UNLICENSE"))}
        if package.get("license_file"):
            files.add(directory / package["license_file"])
        if not files:
            raise SystemExit("no license notice found for " + package["name"])
        sections.append("### " + package["name"] + " " + package["version"] + "\n")
        sections.append("Declared license: `" + (package["license"] or "see notice") + "`. "
                        "[Upstream package](https://crates.io/crates/" + package["name"] + "/" + package["version"] + ").\n")
        for path in sorted(files):
            content = path.read_text(encoding="utf-8").rstrip()
            digest = hashlib.sha256(content.encode()).hexdigest()
            if digest not in texts:
                texts[digest] = (len(texts) + 1, content)
            index, _ = texts[digest]
            sections.append("- `" + path.name + "`: license text " + str(index) + ".\n")
    sections.append("## Preserved crate license texts\n")
    for index, content in texts.values():
        sections.append("### License text " + str(index) + "\n" + fenced(content))
    sections.append("## Rust standard library and compiler builtins\n")
    sections.append("The official standard-library copyright inventory is preserved below. "
                    "It includes component-specific copyright statements and exceptions. "
                    "Compiler builtins may include Apache-2.0 with the LLVM exception alongside MIT/Apache terms; "
                    "the inventory and full exception text govern those components.\n")
    for name in ["Apache-2.0.txt", "MIT.txt", "LLVM-exception.txt"]:
        sections.append("### Rust toolchain license: " + name + "\n" + fenced((rust_docs / "licenses" / name).read_text(encoding="utf-8")))
    text = NoticeText()
    text.feed(copyright_file.read_text(encoding="utf-8"))
    sections.append("### Official standard-library copyright inventory\n" + fenced("".join(text.parts)))
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text("\n".join(sections), encoding="utf-8")
    print("Preserved notices for", sum(bool(p["source"]) for p in metadata["packages"]),
          "locked crates and", version)


if __name__ == "__main__":
    main()
