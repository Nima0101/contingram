#!/usr/bin/env python3
"""Emit SPDX 2.3 JSON from Cargo's resolved locked dependency graph."""
import argparse
import datetime
import hashlib
import json
import pathlib
import re
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--locked", "--format-version", "1"]))
    lock = pathlib.Path("Cargo.lock").read_text(encoding="utf-8")
    checksums = {}
    for block in lock.split("[[package]]")[1:]:
        values = dict(re.findall(r'^([a-z]+) = "([^"]*)"$', block, re.M))
        if "checksum" in values:
            checksums[(values["name"], values["version"])] = values["checksum"]
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True, encoding="utf-8").strip()
    digest = hashlib.sha256((lock + commit).encode()).hexdigest()
    timestamp = subprocess.check_output(["git", "show", "-s", "--format=%ct", "HEAD"], text=True, encoding="utf-8").strip()
    created = datetime.datetime.fromtimestamp(int(timestamp), datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
    identifiers = {p["id"]: "SPDXRef-" + re.sub(r"[^A-Za-z0-9.-]", "-", p["name"] + "-" + p["version"])
                   for p in metadata["packages"]}
    packages = []
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        name, version = package["name"], package["version"]
        item = {"SPDXID": identifiers[package["id"]], "name": name, "versionInfo": version,
                "downloadLocation": f"https://crates.io/api/v1/crates/{name}/{version}/download"
                if package["source"] else "NOASSERTION", "filesAnalyzed": False,
                "licenseConcluded": "NOASSERTION", "licenseDeclared":
                (package["license"].replace("/", " OR ") if package["license"] else "NOASSERTION"),
                "copyrightText": "NOASSERTION", "externalRefs": [{"referenceCategory": "PACKAGE-MANAGER",
                "referenceType": "purl", "referenceLocator": f"pkg:cargo/{name}@{version}"}]}
        if (name, version) in checksums:
            item["checksums"] = [{"algorithm": "SHA256", "checksumValue": checksums[(name, version)]}]
        packages.append(item)
    relationships = [{"spdxElementId": "SPDXRef-DOCUMENT", "relationshipType": "DESCRIBES",
                      "relatedSpdxElement": identifiers[metadata["resolve"]["root"]]}]
    for node in metadata["resolve"]["nodes"]:
        for dependency in node["dependencies"]:
            relationships.append({"spdxElementId": identifiers[node["id"]], "relationshipType": "DEPENDS_ON",
                                  "relatedSpdxElement": identifiers[dependency]})
    document = {"spdxVersion": "SPDX-2.3", "dataLicense": "CC0-1.0", "SPDXID": "SPDXRef-DOCUMENT",
                "name": "contingram-dependencies", "documentNamespace":
                "https://github.com/Nima0101/contingram/sbom/" + digest,
                "creationInfo": {"created": created, "creators": ["Tool: contingram-sbom-0.1.0"]},
                "packages": packages, "relationships": relationships}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
