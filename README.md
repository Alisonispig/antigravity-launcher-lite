<div align="center">
  <img src="src/assets/app-icon.png" width="96" height="96" alt="Antigravity Launcher Lite 图标" />
  <h1>Antigravity Launcher Lite</h1>
  <p>为 Antigravity 桌面客户端配置代理、检测连接并启动。</p>
  <p>
    <img src="https://img.shields.io/badge/version-1.0.0-527bec" alt="版本 1.0.0" />
    <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="MIT License" /></a>
    <a href="https://github.com/Alisonispig/antigravity-launcher-lite/actions/workflows/verify.yml"><img src="https://github.com/Alisonispig/antigravity-launcher-lite/actions/workflows/verify.yml/badge.svg" alt="构建验证" /></a>
  </p>
  <p>
    <a href="#使用方法">使用方法</a> ·
    <a href="https://github.com/Alisonispig/antigravity-launcher-lite/releases">版本下载</a> ·
    <a href="#常见问题">常见问题</a> ·
    <a href="#鸣谢与参考项目">鸣谢</a> ·
    <a href="https://github.com/Alisonispig/antigravity-launcher-lite/issues">反馈问题</a>
  </p>
</div>

## 这是什么？

Antigravity Launcher Lite 是一个独立的 Antigravity 代理启动工具。你只需要填写客户端路径和已有代理的地址、端口，启动器就会检查连接，并通过子进程代理环境启动 Antigravity，省去反复在终端设置环境变量的步骤。

浏览器能够正常访问网络，不一定意味着桌面客户端使用了相同的代理。本项目把**代理配置、连接检测和客户端启动**放在一个窗口里，方便排查与代理有关的连接或登录问题。

它需要配合你已经安装的 Antigravity 和代理软件使用，不提供代理节点、订阅或代理内核。

> 本应用为独立开发的非官方工具，非 Google 出品，与 Google 无隶属或合作关系。

## 功能

- **手动配置客户端**：填写安装路径并验证；macOS 支持解析 `.app` 中的实际可执行文件。
- **检测代理连接**：支持无需账号密码的 HTTP 代理及提供 HTTP CONNECT 能力的混合端口，检查代理握手和 HTTPS 连接。
- **检测并启动**：启动前重新验证路径与连接，识别已有实例，并观察启动后的早期退出。
- **自动保存配置**：记住客户端路径、代理地址、端口、语言和主题，下次打开自动恢复。
- **菜单栏与托盘**：关闭窗口后保留后台入口；macOS 同时隐藏 Dock 图标，可从菜单栏恢复窗口或退出。
- **中文、英文与明暗主题**：支持浅色、深色及跟随系统，macOS 原生标题栏与页面配色同步。
- **操作记录**：在窗口内查看本次会话最近的检测与启动结果。

应用基于 Tauri 2、Vue 3、TypeScript 和 Rust，使用系统 WebView，不捆绑 Chromium 或 Node.js 运行时。

## 下载与平台支持

下载入口：[1.0.0 正式版](https://github.com/Alisonispig/antigravity-launcher-lite/releases/tag/v1.0.0)。

**1.0.0 已正式发布，多个平台的使用测试已由维护者确认通过。** 按下面的系统和架构选择安装包；如需自行修改或构建，也可以使用下方的源码运行方式。

| 平台 | 下载文件 | 验证范围 |
| --- | --- | --- |
| macOS · Apple Silicon | `macos-arm64.dmg` / `macos-arm64.app.zip` | CI 原生打包与启动检查，并在本机打开安装包验证窗口与配置恢复 |
| macOS · Intel | `macos-x64.dmg` / `macos-x64.app.zip` | CI 原生打包与启动检查；维护者确认设备测试通过 |
| Windows · x64 | `windows-x64-setup.exe` | CI 安装与启动检查；维护者确认设备测试通过 |
| Linux · x86_64 | `linux-x64.deb` / `linux-x64.AppImage` | CI 包结构与启动检查；维护者确认设备测试通过 |

文件名均带有 `Antigravity-Launcher-Lite_1.0.0_` 前缀。Mac 打开 DMG 后将应用拖入 Applications；Windows 运行安装器，默认安装到当前用户；Linux 按发行版选择 deb 或 AppImage。

macOS 使用 adhoc 本地签名，未做 Developer ID 签名与苹果公证；Windows 未做代码签名。系统可能出现安全提示，具体依赖、签名状态、校验值与使用限制见发布说明。CI 启动检查不等于真实设备上的完整使用验收。

## 使用方法

### 1. 准备客户端与代理

安装 Antigravity，打开你平时使用的代理软件，并确认代理连接正常。在代理软件中找到 **HTTP 端口**或**混合端口**。

例如，代理地址为 `127.0.0.1`、端口为 `7890`，就在启动器中分别填写这两项。主机栏只填写 IP 或主机名，不要填写 `http://`、完整代理 URL 或账号密码。

不需要为了使用本工具开启 TUN，也不需要由本工具切换系统全局代理。

### 2. 填写客户端路径

| 系统 | 应填写的内容 |
| --- | --- |
| macOS | Antigravity 的 `.app` 路径，通常为 `/Applications/Antigravity.app` |
| Windows | `Antigravity.exe` 的完整路径 |
| Linux | `antigravity` 可执行文件或 `Antigravity.AppImage` 的完整路径 |

点击路径旁的“验证”，确认选择正确。路径必须是绝对路径；Linux 下目标文件还需要具备可执行权限。

### 3. 检测并启动

1. 如果 Antigravity 已经运行，先通过它自己的退出菜单正常退出。
2. 填写代理主机和端口，可以先点击“检测”检查连接。
3. 点击“检测并启动”，等待检查完成并打开客户端。
4. 在 Antigravity 中确认实际联网和登录情况。

输入修改后会自动保存，底部显示“已保存”。不完整或无效的代理输入不会覆盖上次保存的有效配置。

**检测通过与登录成功是两回事。** 当前检测目标为 `accounts.google.com`，成功仅表示经所选代理可以建立到该目标的 HTTPS 连接；它不能证明所有 Antigravity 服务都可访问，也不能排除账号、地区或服务端限制。启动器传入的是代理环境变量，并不保证客户端所有组件、所有流量都会采用该代理。

### 4. 关闭与退出

关闭窗口后，可通过菜单栏或系统托盘图标中的“打开主窗口”回到启动器。选择“退出”才会完全结束启动器。

退出启动器不会主动关闭已经启动的 Antigravity。

## 常见问题

### 无法连接代理，应该检查什么？

确认代理软件正在运行，填写的主机、端口与代理软件中的设置一致。部分软件为 HTTP 和 SOCKS5 分别设置端口，请选用 HTTP 端口，或支持 HTTP CONNECT 的混合端口。

当前不支持 SOCKS-only 端口、需要账号密码的代理以及 PAC 配置。自动发现客户端与代理、多配置档案和取消检测也尚未提供。

### 提示“客户端已运行”，但我已经关掉窗口了？

关闭编辑窗口不一定会退出 Antigravity。请在 Antigravity 自身的菜单中选择退出，再回到启动器重试。本工具不会替你强制结束进程。

### 检测通过，为什么还是无法登录？

先检查代理规则是否覆盖客户端需要访问的服务，并查看 Antigravity 的具体报错。当前检测只检查一个目标，不能保证登录链路中的其他服务同样可用。更换代理节点也不能解决所有账号或服务端问题。

### 设置保存在哪里？

配置保存在系统应用配置目录的 `io.github.antigravity-launcher-lite/config.json` 中。macOS 的位置为：

```text
~/Library/Application Support/io.github.antigravity-launcher-lite/config.json
```

配置损坏时会先保留备份，再恢复默认值。若读取失败或配置来自较新版本，启动器会提示问题并避免覆盖原文件。操作记录仅保留在当前会话，退出后不会保存。

## 隐私与工作方式

- 代理设置只传给本次启动的 Antigravity 子进程，不修改系统全局代理。
- 不读取或导出 Antigravity 的 token、Cookie 或账号数据库。
- 不修补 Antigravity 文件，不提供登录代办、账号切换或地区解锁服务。
- 不自动提权，不自动结束用户应用，不关闭系统安全机制。
- 配置保存在本机；当前无遥测，不上传配置或操作记录。

连接检测会经你填写的代理访问检测目标，这是检查网络连通性所必需的请求。

## 从源码运行

开发环境需要 **Node.js 24**、**Rust 1.99.1** 和对应系统的 [Tauri 开发依赖](https://v2.tauri.app/start/prerequisites/)。Rust 版本由仓库内的 `rust-toolchain.toml` 固定。

```sh
git clone https://github.com/Alisonispig/antigravity-launcher-lite.git
cd antigravity-launcher-lite
npm ci
npm run tauri dev
```

构建当前平台的应用：

```sh
npm run tauri build
```

构建产物位于 `src-tauri/target/release/bundle/`。构建 Windows、macOS 和 Linux 产物需要各自平台的开发环境及依赖。

代码检查与测试：

```sh
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

`npm run dev` 仅启动浏览器中的前端预览，不能执行代理检测或启动客户端；完整功能需要 Tauri 桌面窗口。

## 鸣谢与参考项目

本项目的使用场景与设计思路参考了以下开源项目，感谢作者公开分享实现与经验：

| 项目 | 作者 | 参考内容 |
| --- | --- | --- |
| [antigravity-proxy-launcher](https://github.com/peroperoyui-lab/antigravity-proxy-launcher) | [peroperoyui-lab](https://github.com/peroperoyui-lab) | 为 Antigravity 设置进程级代理的启动方式，以及与代理相关的 OAuth 连接问题排查思路 |
| [antigravity-launcher](https://github.com/Ikaros-521/antigravity-launcher) | [Ikaros-521](https://github.com/Ikaros-521) | 图形化代理启动器的交互、代理检测与状态反馈设计 |

Antigravity Launcher Lite 使用 Tauri、Vue 和 Rust 独立实现。参考项目保留各自的版权与许可，本项目的 MIT 许可证不改变其他项目或 Antigravity 本体的许可。

也感谢 [Tauri](https://v2.tauri.app/)、[Vue](https://cn.vuejs.org/)、[Rust](https://www.rust-lang.org/)、[Vite](https://vite.dev/)、[Tokio](https://tokio.rs/) 及其依赖生态的维护者。本项目图标通过图像生成工具制作，源文件与生成说明见 [图标资源](doc/06-icon-design.md)。

## 参考资料与项目文档

- [Tauri 开发前置依赖](https://v2.tauri.app/start/prerequisites/) · [权限与 Capabilities](https://v2.tauri.app/security/capabilities/) · [应用分发](https://v2.tauri.app/distribute/)
- [Vue 3 文档](https://cn.vuejs.org/guide/introduction.html) · [Rust 程序设计语言](https://doc.rust-lang.org/book/)
- [Apple：安全地打开 Mac 应用](https://support.apple.com/102445)
- [产品规划](doc/01-product-plan.md) · [技术架构](doc/02-architecture.md) · [打包与发布](doc/03-distribution.md) · [开发路线图](doc/04-roadmap.md)
- [参考项目核对记录](doc/05-references.md)

## 反馈与参与

欢迎通过 [Issues](https://github.com/Alisonispig/antigravity-launcher-lite/issues) 报告问题或提出建议，也欢迎提交 Pull Request。

报告问题时，请说明系统与芯片架构、启动器和 Antigravity 版本、代理协议类型、复现步骤，以及窗口中的错误提示。截图或诊断内容请先去除账号信息、代理口令、订阅链接和其他隐私数据。

平台兼容性、代理协议、配置可靠性与使用体验是目前优先完善的方向。具体任务见 [开发路线图](doc/04-roadmap.md)。

## 许可证与声明

本项目采用 [MIT License](LICENSE)，可以在遵守许可证的前提下使用、修改和分发。

Antigravity 及相关名称、商标属于其各自权利人。本项目不分发 Antigravity 本体，不代表 Google，也不提供官方支持。软件按“原样”提供，不承诺解决所有网络或登录问题。
