"""为当前构建目标收集依赖许可与源码定位，不读取用户配置。"""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    result = subprocess.run(["cargo", "metadata", "--locked", "--format-version", "1", "--manifest-path", str(ROOT / "src-tauri/Cargo.toml"), "--filter-platform", args.target], capture_output=True, text=True, check=True)
    metadata = json.loads(result.stdout)
    ids = {node["id"] for node in metadata["resolve"]["nodes"]}
    text = ["Antigravity Launcher Lite — 第三方依赖声明", "项目许可证见安装目录内的 LICENSE.txt。", "以下列出目标平台构建图中的依赖（含构建依赖）；源码可按包名和版本从对应上游获取。", ""]
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["id"] not in ids or package.get("source") is None:
            continue
        assert package.get("license") or package.get("license_file"), f"缺少许可声明：{package['name']}"
        text += ["=" * 72, f"Rust: {package['name']} {package['version']}", "License: " + (package.get("license") or "see license file"), f"Source: https://crates.io/crates/{package['name']}/{package['version']}", package.get("repository") or ""]
        root = Path(package["manifest_path"]).parent
        licenses = [p for p in root.iterdir() if p.is_file() and p.name.lower().startswith(("licen", "copying", "notice"))]
        if package.get("license_file"):
            licenses.append(root / package["license_file"])
        if not licenses:
            licenses = [p for p in root.iterdir() if p.is_file() and p.name.lower().startswith("readme")]
            # 部分 crate 在源码文件头保留版权与许可声明。
            for source in [root / "lib.rs", root / "src/lib.rs"]:
                if source.is_file():
                    text += ["Source header:", "\n".join(source.read_text(encoding="utf-8", errors="replace").splitlines()[:30])]
        for path in sorted(set(licenses)):
            text += [f"--- {path.name} ---", path.read_text(encoding="utf-8", errors="replace")]
    lock = json.loads((ROOT / "package-lock.json").read_text(encoding="utf-8"))
    for relative, package in sorted(lock["packages"].items()):
        if not relative or package.get("dev"):
            continue
        root = ROOT / relative
        info = json.loads((root / "package.json").read_text(encoding="utf-8"))
        text += ["=" * 72, f"npm: {info['name']} {info['version']}", "License: " + str(info.get("license", "see package source")), f"Source: https://www.npmjs.com/package/{info['name']}/v/{info['version']}"]
        for path in sorted(root.iterdir()):
            if path.is_file() and path.name.lower().startswith(("licen", "copying", "notice")):
                text += [f"--- {path.name} ---", path.read_text(encoding="utf-8", errors="replace")]
    output = ROOT / "src-tauri/notices/THIRD-PARTY-NOTICES.txt"
    output.write_text("\n".join(text) + "\n", encoding="utf-8")
    print(f"生成第三方声明：{output.name} ({output.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
