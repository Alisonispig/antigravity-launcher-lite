# 打包、签名与发布

## 发布矩阵

| 平台 | 首版架构 | 计划产物 | 用户侧要求 |
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

## GitHub Actions 计划

验证任务运行前端类型检查/构建、cargo fmt、clippy 与必要测试。macOS、Windows、Linux 分别在对应 runner 原生构建。使用固定工具链策略，锁文件提交，第三方 Action 在实施时固定到可审计版本/commit。macOS 两架构分别构建；跨架构构建结果不等于在真实硬件验证运行。

tag 触发构建后先创建 Draft Release，附版本说明、架构、产物校验值、签名/公证状态、最低 OS 及已测试系统。完成安装烟测后再公开。初版不实现应用内自动安装更新；若以后加入 updater，其更新签名密钥与系统代码签名证书分别管理。

不得把证书、密码或私钥提交到仓库。没有证书时允许清晰标注的构建流程，不伪装成功公证。

## 开源合规

本项目采用 MIT。参考两个项目的设计，不将“公开源码”默认等同于可无条件复制；移植代码时核实实际文件的许可证与版权，并保留声明，新增 THIRD_PARTY_NOTICES。发布前审核依赖许可证与应用品牌，不使用 Google 官方应用图标或暗示官方出品，不打包 Antigravity 本体。

## 开发环境排障记录

2026-11-04 本机 macOS 默认 27 SDK 与链接器不兼容，开发验证通过为单次命令指定已有的 15.4 SDK 解决：

```sh
SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk npm run tauri build
```

这是本机环境记录，不是通用安装步骤；其他机器应按实际 SDK 与工具链处理，不应照抄不存在的路径。若 Rust 未加入 PATH，可先执行 `. "$HOME/.cargo/env"`。
