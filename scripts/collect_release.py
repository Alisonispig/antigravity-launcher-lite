"""检查原生构建产物并收集带平台名称的发布文件。"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
PLATFORMS = {"macos-arm64", "macos-x64", "windows-x64", "linux-x64"}


def digest(path):
    with path.open("rb") as stream:
        checksum = hashlib.sha256()
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            checksum.update(chunk)
        return checksum.hexdigest()


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", choices=sorted(PLATFORMS), required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--version", required=True)
    args = parser.parse_args()
    package = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))
    config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    assert package["version"] == config["version"] == args.version, "发布版本与工程版本不一致"
    build = ROOT / "src-tauri/target" / args.target / "release"
    bundle = build / "bundle"
    output = ROOT / "release-assets"
    output.mkdir(exist_ok=True)
    prefix = f"Antigravity-Launcher-Lite_{args.version}_{args.platform}"
    files = []

    def single(pattern):
        paths = list(bundle.glob(pattern))
        assert len(paths) == 1, f"期望一个 {pattern}，实际为 {len(paths)}"
        return paths[0]

    def copy(source, suffix):
        target = output / (prefix + suffix)
        shutil.copy2(source, target)
        files.append(target)

    if args.platform.startswith("macos"):
        app = single("macos/*.app")
        with (app / "Contents/Info.plist").open("rb") as stream:
            info = plistlib.load(stream)
        assert info["CFBundleShortVersionString"] == args.version
        exe = app / "Contents/MacOS" / info["CFBundleExecutable"]
        arch = "arm64" if args.platform == "macos-arm64" else "x86_64"
        assert subprocess.check_output(["lipo", "-archs", str(exe)], text=True).strip() == arch
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app)], check=True)
        copy(single("dmg/*.dmg"), ".dmg")
        zipped = output / (prefix + ".app.zip")
        subprocess.run(["ditto", "-c", "-k", "--sequesterRsrc", "--keepParent", str(app), str(zipped)], check=True)
        files.append(zipped)
    elif args.platform == "windows-x64":
        data = (build / "antigravity-launcher-lite.exe").read_bytes()
        offset = struct.unpack_from("<I", data, 0x3C)[0]
        assert data[offset:offset + 4] == b"PE\0\0" and struct.unpack_from("<H", data, offset + 4)[0] == 0x8664
        copy(single("nsis/*-setup.exe"), "-setup.exe")
    else:
        data = (build / "antigravity-launcher-lite").read_bytes()[:64]
        assert data[:5] == b"\x7fELF\x02" and struct.unpack_from("<H", data, 18)[0] == 62
        deb = single("deb/*.deb")
        assert subprocess.check_output(["dpkg-deb", "-f", str(deb), "Architecture"], text=True).strip() == "amd64"
        assert subprocess.check_output(["dpkg-deb", "-f", str(deb), "Version"], text=True).strip() == args.version
        copy(deb, ".deb")
        copy(single("appimage/*.AppImage"), ".AppImage")
    manifest = {
        "version": args.version, "platform": args.platform, "target": args.target,
        "commit": os.environ.get("GITHUB_SHA", "local"),
        "files": [{"name": f.name, "bytes": f.stat().st_size, "sha256": digest(f)} for f in files],
    }
    (output / f"manifest-{args.platform}.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    for entry in manifest["files"]:
        print(f"{entry['name']}: {entry['bytes']} bytes / SHA-256 {entry['sha256']}")


if __name__ == "__main__":
    main()
