# Antigravity Launcher Lite

基于 Tauri 2 的开源 Antigravity 代理启动器，计划支持 Windows、macOS、Linux。

源码仓库：[Alisonispig/antigravity-launcher-lite](https://github.com/Alisonispig/antigravity-launcher-lite)。

通过图形界面识别客户端、检测已有代理，并以进程级代理配置启动 Antigravity。无需 TUN，不修改系统全局代理，不捆绑代理内核或 Antigravity 本体。

## 当前状态

2026-11-04：已建立 Tauri 2 + Vue 3 + TypeScript 工程，实现手动客户端路径校验、无认证 HTTP CONNECT/HTTPS 检测、子进程代理环境与启动、已有实例检测和早期退出观察。界面默认白色浅色主题，提供中英切换、明暗主题与分页操作记录。关闭窗口后驻留菜单栏/系统托盘；macOS 同时隐藏 Dock 图标，通过菜单栏“打开主窗口”恢复，选择“退出”完全退出启动器。

这是开发中的最小流程，不是首版交付。自动发现、SOCKS5、取消检测、诊断导出及正式安装包尚未完成。路径、代理、语言和主题已支持本地自动保存与恢复。已生成 macOS arm64 开发版 `.app` 并验证真实窗口中的路径校验与代理连接失败反馈；仅在 macOS arm64 本机验证构建和核心测试；Windows/Linux 工作流已编写但尚未执行，真实客户端联网和三平台生命周期仍待验收。

## 本地开发

需要 Node.js 24、固定的 Rust 1.99.1 工具链和对应平台 Tauri 开发依赖。普通用户无需开发工具，后续通过安装包使用。

```sh
npm ci
npm run tauri dev
```

若 Rust 尚未加入 PATH，先执行 `. "$HOME/.cargo/env"`。前端预览使用 `npm run dev`，浏览器中检测与启动按钮禁用，不提供模拟成功结果。

```sh
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

本机默认 macOS 27 SDK 与链接器存在不兼容，验证时在命令前指定 `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk`，仅影响该命令。其他机器按实际已安装 SDK 处理，不需要照抄此路径。项目未修改系统工具链选择。

开发使用：填写客户端绝对路径（macOS 选择 `.app`），填入 HTTP/混合代理主机和端口；可先验证路径、检测代理，再“检测并启动”。若客户端已有实例，请自行正常退出。HTTPS 检测目标暂定 `accounts.google.com`，只判断该目标网络可达，不代表登录或全部 Antigravity 服务可用。检测总预算约 15 秒，启动后观察 1.5 秒；当前不支持中途取消。客户端路径、代理主机和有效端口、语言和主题修改后自动保存，底部显示保存结果，完全退出后再次打开会恢复。配置位于平台应用配置目录（macOS 为 `~/Library/Application Support/io.github.antigravity-launcher-lite/config.json`），不保存代理认证信息。配置损坏时先备份再恢复默认；读取失败或配置来自较新版本时不覆盖原文件。操作记录仍只保留在本次会话。

## 文档入口

- [项目规划与功能范围](doc/01-product-plan.md)
- [技术架构与三平台适配](doc/02-architecture.md)
- [打包、签名与发布](doc/03-distribution.md)
- [开发任务与验收](doc/04-roadmap.md)
- [参考项目与技术资料](doc/05-references.md)

## 技术选型

Tauri 2 + Rust；界面使用 Vue 3、TypeScript、Vite 和轻量 CSS。依赖版本在工程初始化时核实并提交锁文件。使用系统 WebView，不携带 Chromium、Node.js 或 Node sidecar。

## 许可证

采用 MIT。实现以独立编写为主；引用第三方代码时逐项保留版权、许可证及来源。
