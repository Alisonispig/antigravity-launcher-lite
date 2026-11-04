"""在 CI 的临时目录中检查安装包，并仅启动/结束本脚本创建的测试进程。"""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import time


def launch(command):
    with tempfile.TemporaryFile() as output:
        process = subprocess.Popen(command, stdout=output, stderr=output, start_new_session=os.name != "nt")
        try:
            time.sleep(6)
            if process.poll() is not None:
                output.seek(0)
                print(output.read().decode("utf-8", errors="replace")[-8000:])
                raise RuntimeError("安装包中的应用在启动观察期内退出")
        finally:
            if process.poll() is None:
                if os.name == "nt":
                    process.terminate()
                else:
                    import signal
                    os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    if os.name == "nt":
                        process.kill()
                    else:
                        os.killpg(process.pid, signal.SIGKILL)
                    process.wait()
    print("应用启动观察通过；仅结束本脚本创建的测试进程")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", required=True)
    args = parser.parse_args()
    root = Path("release-assets").resolve()
    with tempfile.TemporaryDirectory(prefix="antigravity-install-") as temporary:
        target = Path(temporary)
        if args.platform.startswith("macos"):
            mount = target / "mounted"
            mount.mkdir()
            subprocess.run(["hdiutil", "attach", "-nobrowse", "-readonly", "-mountpoint", str(mount), str(next(root.glob("*.dmg")))], check=True)
            try:
                app = next(mount.glob("*.app"))
                installed = target / app.name
                subprocess.run(["ditto", str(app), str(installed)], check=True)
                subprocess.run(["codesign", "--verify", "--deep", "--strict", str(installed)], check=True)
                with (installed / "Contents/Info.plist").open("rb") as stream:
                    info = plistlib.load(stream)
                launch([str(installed / "Contents/MacOS" / info["CFBundleExecutable"])])
            finally:
                subprocess.run(["hdiutil", "detach", str(mount)], check=True)
        elif args.platform == "windows-x64":
            installer = next(root.glob("*-setup.exe"))
            installed = target / "app"
            result = subprocess.run([str(installer), "/S", f"/D={installed}"], timeout=180)
            assert result.returncode == 0, f"NSIS 安装失败：{result.returncode}"
            exe = installed / "antigravity-launcher-lite.exe"
            assert exe.is_file(), "安装后未找到客户端程序"
            launch([str(exe)])
        else:
            installed = target / "deb"
            subprocess.run(["dpkg-deb", "--extract", str(next(root.glob("*.deb"))), str(installed)], check=True)
            exe = installed / "usr/bin/antigravity-launcher-lite"
            assert exe.is_file()
            launch(["dbus-run-session", "--", "xvfb-run", "-a", str(exe)])
            appimage = next(root.glob("*.AppImage"))
            appimage.chmod(0o755)
            launch(["dbus-run-session", "--", "xvfb-run", "-a", str(appimage)])


if __name__ == "__main__":
    main()
