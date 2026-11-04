"""核对四个原生构建的清单并生成总校验文件。"""
import argparse
import hashlib
import json
from pathlib import Path


def digest(path):
    checksum = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
    return checksum.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--version", required=True)
    parser.add_argument("--commit", required=True)
    args = parser.parse_args()
    root = Path("release-assets")
    manifests = [json.loads(p.read_text(encoding="utf-8")) for p in sorted(root.glob("manifest-*.json"))]
    assert {m["platform"] for m in manifests} == {"macos-arm64", "macos-x64", "windows-x64", "linux-x64"}
    assert len(manifests) == 4
    files = []
    for manifest in manifests:
        assert manifest["version"] == args.version and manifest["commit"] == args.commit
        for entry in manifest["files"]:
            path = root / entry["name"]
            assert path.name == entry["name"] and path.stat().st_size == entry["bytes"] > 0
            assert digest(path) == entry["sha256"], f"校验失败：{path.name}"
            files.append(path)
    assert len(files) == 7 and len({p.name for p in files}) == 7
    metadata = root / "BUILD-METADATA.json"
    metadata.write_text(json.dumps({"version": args.version, "commit": args.commit, "builds": manifests}, indent=2) + "\n", encoding="utf-8")
    files.append(metadata)
    (root / "SHA256SUMS.txt").write_text("".join(f"{digest(p)}  {p.name}\n" for p in sorted(files)), encoding="utf-8")
    for path in root.glob("manifest-*.json"):
        path.unlink()
    print(f"已核对 {len(files) - 1} 个安装包/应用压缩包，并生成 SHA256SUMS.txt")


if __name__ == "__main__":
    main()
