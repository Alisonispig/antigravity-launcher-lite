# 打包、签名与发布

## 1.0.2 发布矩阵

| 平台 | 发布架构 | 已发布产物 | 用户侧要求 |
| --- | --- | --- | --- |
| Windows | x64 | NSIS setup.exe，优先按用户安装 | Microsoft WebView2；安装器检测并按需引导安装 |
| macOS | Apple Silicon arm64、Intel x64 | 分架构 DMG，附 .app 压缩包 | 系统 WKWebView；根据 Tauri/依赖核实最低系统版本 |
| Linux | x86_64 | AppImage、deb | 发行版 WebKitGTK/图形栈依赖；AppImage 也须实测运行条件 |

Windows ARM64 与 Linux ARM64 为后续任务，不因提供源码就宣称二进制支持。macOS 可后续评估 universal 包与分包的体积权衡。最低 OS 版本在初始化与 CI runner 选择时固定，不写未经验证的兼容承诺。

## 用户安装体验

- macOS：打开 DMG，拖入 Applications，启动。免费未公证版本可能被 Gatekeeper 拦截，README 必须清楚标注，不宣称换 Tauri 即能解决“已损坏”。
- Windows：安装器明确 WebView2 是否需要联网下载；默认不捆绑整个浏览器运行时。缺网时给出可理解的说明及官方离线安装入口，不让用户安装 Node/Rust。
- Linux：优先给出发行版与格式选择说明；列出已验证环境。deb 声明系统依赖；AppImage 不承诺所有发行版免依赖，检查 FUSE、WebKitGTK 与图形兼容性。

## 签名策略

开发和自用阶段不要求购买 Apple Developer Program。macOS 本地/adhoc 签名仅满足相应构建与运行需求，不等于可信 Developer ID 或苹果公证；尤其注意 Apple Silicon 的签名要求。公开包先明确标注是否未公证，不自动移除隔离属性、不关闭系统安全机制。

正式面向普通用户分发时，评估 Developer ID 签名、hardened runtime、notarization 与 stapling。会员当前官方费用 99 美元/年（部分地区当地货币计价），执行前再次核实。即便公证成功也不承诺完全没有系统提示。

Windows 未签名包可能触发 SmartScreen；Windows Authenticode 签名另行评估，不与苹果会员混淆。Linux 提供 SHA-256 校验值，后续按分发渠道评估签名。

## GitHub Actions 验证与打包

验证任务运行前端类型检查/构建、cargo fmt、clippy 与必要测试。macOS、Windows、Linux 分别在对应 runner 原生构建。使用固定工具链策略，锁文件提交，第三方 Action 在实施时固定到可审计版本/commit。macOS 两架构分别构建；跨架构构建结果不等于在真实硬件验证运行。

通过 workflow_dispatch 指定版本触发打包，产物检查与 CI 安装/启动观察全部通过后，先创建 Draft Release，附版本说明、架构、产物校验值、签名/公证状态、构建环境及已测试系统。维护者下载核对后再公开；发布标签关联实际构建提交。初版不实现应用内自动安装更新；若以后加入 updater，其更新签名密钥与系统代码签名证书分别管理。

不得把证书、密码或私钥提交到仓库。没有证书时允许清晰标注的构建流程，不伪装成功公证。

## 开源合规

本项目采用 MIT。参考两个项目的设计，不将“公开源码”默认等同于可无条件复制；移植代码时核实实际文件的许可证与版权，并保留声明，新增 THIRD_PARTY_NOTICES。发布前审核依赖许可证与应用品牌，不使用 Google 官方应用图标或暗示官方出品，不打包 Antigravity 本体。

## 开发环境排障记录

2026-11-04 本机 macOS 默认 27 SDK 与链接器不兼容，开发验证通过为单次命令指定已有的 15.4 SDK 解决：

```sh
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk npm run tauri build
```

这是本机环境记录，不是通用安装步骤；其他机器应按实际 SDK 与工具链处理，不应照抄不存在的路径。若 Rust 未加入 PATH，可先执行 `. "$HOME/.cargo/env"`。

## 安装包流水线操作

维护者通过 `打包发布` 工作流手动选择 main 与版本号：

```sh
gh workflow run release.yml --ref main -f version=1.0.2
```

工作流在 macOS 15（arm64）、macOS 15 Intel、Windows Server 2022、Ubuntu 22.04 上分别构建原生安装包。各目标先运行核心测试，收集第三方依赖许可，再生成 DMG/应用 ZIP、当前用户 NSIS、deb/AppImage。生成的许可文件随包附带，Windows Python 显式使用 UTF-8 读取 Cargo 元数据。

每个构建检查二进制架构、版本与安装包结构，并在 CI 临时目录执行安装/启动观察。脚本只结束其自己创建的测试进程。四个目标全部成功后，汇总同一构建提交的产物、SHA-256 和 BUILD-METADATA，再创建预览版 Draft Release；工作流不会自动公开。公开发布前还需下载核对哈希、检查 Mac 实际窗口并更新发布说明。

CI 中的短时进程观察不等于用户真实设备上的完整安装验收；缺少设备验收的平台始终标注预览。macOS 为 adhoc 本地签名，无 Developer ID 与苹果公证；Windows 未做 Authenticode 签名。

1.0.0 已按上述流程公开，维护者确认多个平台测试通过后转为正式版并设为 Latest，构建与校验记录见 `04-roadmap.md`。GitHub Release 中的构建元数据是各文件对应提交和 SHA-256 的依据；主分支后续文档维护不改变已发布安装包对应的源码标签。

1.0.1 于 2026-11-06 完成三平台常规验证与四架构原生打包，下载核对 SHA-256 后正式公开并设为 Latest。macOS 14 runner 已不可用，常规验证迁移到 macOS 15；实际构建提交、流水线与附件记录见 `04-roadmap.md`。
