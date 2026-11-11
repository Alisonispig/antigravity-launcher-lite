# 项目协作约定

- 开始开发前阅读 README.md 和 doc/01-product-plan.md 至 doc/04-roadmap.md。
- 本项目是独立的 Tauri 2 跨平台桌面应用；不要放进其他项目，不引入 Electron 或 Node sidecar。
- 前端 Vue 3 + TypeScript，系统操作和代理逻辑使用 Rust；先完成最小纵向流程，再扩展功能。
- 文档用中文。实现状态以代码和验证结果为准，完成任务后同步更新 doc/04-roadmap.md。
- 代理仅传入目标子进程；禁止更改系统全局代理、自动提权、自动结束用户应用或关闭系统安全机制。
- 不读取或导出 Antigravity token、Cookie、账号数据库。日志不得包含代理认证信息、订阅链接或完整子进程命令行。
- 不把前端传入的字符串拼成 shell 命令；限定 Rust command 输入与 Tauri capabilities。
- macOS/Linux/Windows 使用各自平台适配；不要把仅在本机验证的功能标记为三平台已验证。
- 测试集中在协议握手、配置解析、进程环境和生命周期等真实风险；纯文档及样式修改不另写测试。
- GitHub 创建、推送和公开发布需有用户对应授权；本地初始化不代表已发布。
- 用户于 2026-11-11 要求与 `../thinkrail-launcher-lite` 配套发布：以后要求发布任一项目时，默认同时准备两个项目，统一版本号、分别构建和校验，两个项目均通过后再同批次公开；不因普通代码修改自行发布。配套流程见 ThinkRail 仓库的 `doc/release-policy.md`。
