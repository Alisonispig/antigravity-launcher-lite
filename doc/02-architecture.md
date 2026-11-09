# 技术架构与三平台适配

## 技术决策

- Tauri 2 管理桌面窗口与 IPC；Vue 3 + TypeScript + Vite 提供本地界面。
- Rust 承担发现、握手、检测、进程启动、配置与脱敏。Tokio 处理异步网络及取消；具体 crate 在实施时核实版本与许可证。
- UI 不加载远端页面，不暴露任意 shell、文件读写或任意命令执行接口。只启用需要的插件和 capabilities；设置 CSP。
- 不复用 Electron 主进程作为后台，不依赖外部 curl/nc，不复制有缺陷的 shell 变量拼接实现。

## 工程结构（已建立最小骨架）

```text
src/                       Vue 界面、组件、类型、IPC 封装
src-tauri/src/commands/     窄范围 IPC command
src-tauri/src/core/         proxy、probe、launch、config、diagnostics
src-tauri/src/platform/     windows、macos、linux 适配
src-tauri/capabilities/     最小权限配置
src-tauri/tests/            协议与进程集成测试
.github/workflows/         验证、三平台构建及发布
```

## 核心数据与接口草案

`AppCandidate { id, path, version?, source }`；`ProxyCandidate { id, host, port, protocols, source }`；`ProbeResult { stage, target, result, latencyMs?, errorCode? }`；`LaunchResult { pid, startedAt, earlyExit? }`。

commands：`discover_apps`、`discover_proxies`、`probe_proxy`、`cancel_probe`、`launch_antigravity`、`load_config`、`save_config`、`export_diagnostics`。检测进度通过事件推送；每次请求有 ID，旧请求结果不得覆盖新选择。具体字段随实现补充并保持前后端类型一致。

## 代理发现与验证

发现顺序：用户显式选择/保存配置 → 系统静态代理 → 环境变量 → 常见本地端口。去重并标注来源；无效的旧配置不能静默掩盖其他候选。自动探测仅扫描预设的本机端口，不扫描局域网或全端口。远端代理仅在用户明确输入或系统配置指定时检测。

分别验证 TCP、HTTP CONNECT/SOCKS5、隧道内 TLS、目标响应。支持 HTTP CONNECT 和 SOCKS5 无认证；TLS 校验保持启用。SOCKS5 使用域名地址请求，HTTP CONNECT 使用主机名，避免本地 DNS 成为必要前提。

初始检测目标候选为 accounts.google.com 与 daily-cloudcode-pa.googleapis.com，实施前结合客户端行为核实。401/403/404 等响应可说明网络层到达，不能视为授权成功。目标不可达、需要认证、协议错误和服务拒绝应分别显示。HTTP 代理首版指以明文 HTTP 连接代理并 CONNECT 到 HTTPS 目标，不混称为 TLS-to-proxy 支持。

设置单阶段超时、总超时、有限并发和取消。HTTP-only 端口不伪装成 SOCKS，SOCKS-only 不塞进只接受 HTTP 的配置。

## 启动与进程生命周期

用原生进程 API 直接启动目标可执行文件，参数按数组传递，不通过 shell。仅替换子进程的代理环境，保留其余正常环境，清除冲突的大小写代理键。HTTP_PROXY/HTTPS_PROXY、ALL_PROXY、NO_PROXY 按实际协议构造，明确 loopback 绕过规则。代理是否被不同 Antigravity 组件采用必须实测。

首版将“SOCKS-only + 客户端部分组件不支持”列为待验证兼容性，不实现隐式 HTTP-to-SOCKS 转换；不满足启动要求时建议使用 HTTP/混合端口。Chromium 代理启动参数是否需要由各平台验证结果决定，不宣称环境变量强制代理所有流量。

检测旧实例时结合路径及进程信息，避免只用名称误判；不自动 kill，不读取或记录可能含 token 的完整命令行。启动后观察早期退出并收集有限日志；启动器退出后应允许 Antigravity 独立运行，避免附带 kill-on-drop。通过真实三平台测试确认生命周期。

## 平台差异

| 平台 | 发现与启动 | 代理来源 | 重点风险 |
| --- | --- | --- | --- |
| Windows | 常见用户/系统安装目录、手动 exe；使用原生进程接口 | 当前用户系统静态代理、环境变量、常见端口 | 路径含空格/中文，环境变量大小写不敏感；不更改注册表 |
| macOS | /Applications、~/Applications、手动 .app；读取 Info.plist 后解析 Contents/MacOS 的真实程序 | 系统静态代理、环境变量、常见端口 | 区分 .app 目录与可执行文件；直接启动内层程序以传入环境，验证 app bundle 行为 |
| Linux | PATH、常见 /opt 位置、手动文件/AppImage | 环境变量，GNOME 静态代理可选，常见端口 | Wayland/X11、AppImage 权限、启动器包装脚本与发行版差异 |

首版不解析执行 PAC，不保证 KDE 等所有桌面代理设置读取；显示限制并允许手动填写。Flatpak/Snap 安装的目标应用先标记未支持，避免宣称跨沙箱启动有效。

## 配置与诊断

配置存放 Tauri 平台应用配置目录，包含 schemaVersion，提供迁移和损坏备份。日志存放平台日志目录，限制单文件大小和保留数，避免写入应用安装目录。

诊断只记录必要的系统/应用版本、代理来源、协议、检测阶段、错误码与时间。代理 URL 认证部分、用户目录、token、Cookie、环境变量全集、订阅 URL、完整启动命令均不得进入导出。默认无遥测，不上传诊断。导出到用户选择的位置。

## M0/M1 当前实现说明（2026-11-04）

- 现有 IPC 为 `validate_app`、`probe_proxy`、`launch_antigravity`、`load_config`、`save_config`；构建脚本生成 command 权限，只向 main 窗口授权这些窄范围接口，不引入 shell、通用文件或远端页面能力。
- 当前为手动 HTTP/混合端口，不解析代理 URL，不接受认证信息。CONNECT 头限制 8 KiB；连接及握手预算 5 秒，HTTPS 请求预算 10 秒；不跟随重定向，TLS 校验开启。探测固定目标 accounts.google.com，其他目标与客户端实际使用情况待核实。
- 探测与启动共享互斥锁，后端拒绝并发操作。暂未实现取消和阶段事件。输入在操作期间禁用，错误只返回稳定错误码和阶段，不返回底层错误字符串。
- macOS 核对 com.google.antigravity bundle ID、解析 Info.plist、拒绝路径逃逸；Windows/Linux 当前仅做指定文件名、绝对路径及文件属性约束，未进行真实系统运行验证。手动路径不能证明二进制的发布者身份。
- 仅按进程可执行路径检测已有实例，不读取 argv/environ。权限不足或包装器移交情况下仍可能漏判，不能保证检测全部既有实例。
- 子进程继承普通环境，先清除冲突代理键，再设置 HTTP/HTTPS/ALL_PROXY 及 loopback NO_PROXY；不构造 shell 命令。标准输入输出关闭，避免采集目标敏感日志；正常启动后等待线程负责回收，不随启动器退出杀进程。
- 诊断目前只有前端内存中最多 30 条会话结果，日志不落盘。配置已支持持久化；文件轮转、脱敏导出、自动发现与 SOCKS5 留待 M2。

## 窗口与菜单栏生命周期

Rust 创建菜单栏/托盘图标与“打开主窗口”“退出”菜单，不向前端增加托盘或进程控制权限。只有托盘已成功创建时，主窗口关闭事件才被阻止并改为隐藏；初始化失败直接返回错误，避免无入口的隐藏程序。macOS 隐藏窗口后使用 Accessory activation policy 移除 Dock 图标，恢复窗口时切回 Regular，再显示、解除最小化并聚焦。选择托盘“退出”或应用退出菜单直接结束启动器，不结束用户的 Antigravity。后台不轮询代理，不新增常驻服务或开机启动。

前端用固定视口布局和最小窗口尺寸（800×640）保证内容容纳，记录每页三条；帮助拆为“怎么用”和“常见问题”。不通过裁切主要内容来掩盖溢出。图标使用内置 imagegen 生成白底彩色源图，再由 Tauri CLI 生成各平台尺寸，详情见 `06-icon-design.md`。

## 本地配置实现（2026-11-04）

- Rust 通过 Tauri 平台应用配置目录定位固定 `config.json`，前端不能指定配置文件位置。只授权 load/save 两个受限 command，不开放通用文件访问。
- schemaVersion=1，保存客户端路径、裸代理主机、有效端口、语言和主题。保存路径时不访问目标文件，客户端位置暂时不可用不影响保存；启动前仍执行完整客户端校验。代理字段拒绝 URL、认证信息及控制字符。
- 界面启动时先读取配置再允许编辑；编辑时立即发出保存请求，无延时防抖。Rust 同步命令在互斥锁保护下按序处理，前端用修订编号防止旧保存响应覆盖最新提示；无效或不完整代理输入不会覆盖上次有效配置。底部区分保存中、已保存、无效输入和失败重试。
- 写入使用同目录临时文件，写完同步文件后原子替换；Unix 文件权限为 0600，再同步目录。读取大小限制 16 KiB；损坏文件原样备份后回退默认值，备份失败及其他读取错误会提示并禁用保存。
- 未知 schema 版本保留原文件并提示使用较新版本；后续真实 schema 迁移尚待出现新版本时实施，不把拒绝覆盖视为已完成迁移。

## 原生标题栏外观同步

窗口继续使用系统装饰、窗口按钮、拖动与缩放行为。macOS 使用原生 Transparent 标题栏样式（不启用 Overlay、不绘制自定义顶栏），窗口底色与页面浅色/深色底色一致。受限 `set_window_theme` command 只接受主题枚举并作用于主窗口，启动加载已存主题后和用户切换主题时同步原生外观；跟随系统时清除强制主题，并在 ThemeChanged 事件中更新底色。原生窗口外观同步独立于配置保存，因此代理输入尚未完整时也能切换主题。

## 客户端更新后恢复代理（2026-11-06）

Rust `core/recovery.rs` 持有内存会话：经校验的安装路径、本次成功启动时的代理端点、主进程 PID/启动时间及安装文件 SHA-256。默认不开启自动恢复，新增配置字段 autoRestoreProxy 使用 serde 默认值兼容已有配置。开关有独立的窄范围持久化命令，普通输入的迟到保存不能反向覆盖它。启动器完全退出后不会接管已有客户端。

后台每两秒只刷新进程可执行路径及安装文件元数据；不读取 argv、environ 或客户端用户数据。元数据变化才重新散列客户端可执行文件、macOS Info.plist、固定位置的 Electron 版本信息/asar。原主进程消失、安装内容不同、同一路径的唯一新主进程稳定六秒且安装可重新校验时才恢复；原进程退出后最多等待两分钟。仅文件时间变化、普通退出、未更新的重启、多主进程或 PID 复用均不能直接触发退出请求。

恢复与手动检测/启动共用后端互斥锁。先检测原代理并重验进程身份，再发送一次正常关闭请求；macOS 使用主线程 NSRunningApplication.terminate，Windows 向匹配进程可见顶层窗口发送 WM_CLOSE，Linux X11 向匹配 _NET_WM_PID 的窗口发送 EWMH _NET_CLOSE_WINDOW。不使用强制终止或信号模拟正常退出。保存提示或拒绝退出最多等待两分钟；两次连续无目标进程快照后才重新校验安装并启动。观察到第三个进程则停止，避免重复重启；没有退出不会强行拉起。

每个监测任务持有 epoch，关闭开关、新启动会话后旧任务失效；退出请求和启动前重复检查，迟到结果不改变新会话。关闭开关不能撤销已经发出的系统退出请求。自动恢复成功后更新基线，继续跟踪下一次更新。前端只读取脱敏状态并显示使用提示，后台逻辑不依赖前端定时器或窗口可见性。

边界：只观察原安装路径，版本化目录迁移需手动修改路径。Linux 原生 Wayland 和 AppImage 挂载路径不具备当前实现所需的稳定主进程定位/正常关闭能力；未观察到目标或无法正常关闭时提示手动处理。窗口关闭请求是否等于完整退出依赖客户端实现，超时停止，不把发出请求当成退出成功。三平台真实客户端更新验收另记。

## 启动成功后收起窗口（2026-11-09）

新增 closeAfterLaunch 配置字段，缺失时默认 true。独立的 set_close_after_launch 命令在配置锁内读取最后有效配置后只修改该开关，并同步 Rust 内存状态；普通表单保存保留当前开关，防止迟到请求覆盖选择。用户手动启动通过 1.5 秒早期退出观察后，后端读取当前开关并调用共用 hide_main；不向前端开放任意窗口控制接口。隐藏时检查托盘入口，macOS 切换 Accessory 隐藏 Dock 图标，失败时恢复窗口并单独提示收起失败，不能将已成功的客户端启动误报为失败。检测、已有实例错误、早期退出和后台更新恢复不会额外收起用户打开的窗口。
