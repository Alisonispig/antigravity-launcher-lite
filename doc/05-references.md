# 参考资料

核对日期：2026-11-04。外部项目与平台文档会变化，正式实现/发布时再次核实版本和要求。

## 参考项目

- [peroperoyui-lab/antigravity-proxy-launcher](https://github.com/peroperoyui-lab/antigravity-proxy-launcher)：进程级代理启动脚本；已查看 commit f5adfce27e34dab0829bac7718e9358b06c41fb3。
- 该版本 macOS 第 67 行/Linux 第 31 行 HTTP_URL 表达式多出一个右花括号，独立片段已复现：空值变成 `}`，正常 URL 尾部也多 `}`；不要复制该实现。macOS AG_APP 的 `-x` 检查还需区分目录和真实程序。
- [Ikaros-521/antigravity-launcher](https://github.com/Ikaros-521/antigravity-launcher)：Electron 图形界面参考，关注真实握手检测及分阶段诊断思路。已查看 v1.1.0 相关配置，工作流明确不做 macOS 签名和公证；用户报告安装提示“已损坏”，未对其本机二进制做完整性诊断，不能认定唯一根因。

## 官方技术资料

- [Tauri 2 概览](https://v2.tauri.app/start/)
- [开发前置依赖](https://v2.tauri.app/start/prerequisites/)
- [分发与打包](https://v2.tauri.app/distribute/)
- [Capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri 进程模型](https://v2.tauri.app/concept/process-model/)
- [Windows 安装器与 WebView2](https://v2.tauri.app/distribute/windows-installer/)
- [Apple Developer Program](https://developer.apple.com/programs/)
- [macOS 应用分发](https://developer.apple.com/macos/distribution/)
- [Apple：安全地打开 Mac 应用](https://support.apple.com/102445)

## 已明确的产品决定

使用 Tauri 2；支持 Windows/macOS/Linux；开源发布；优先轻量和安装体验。暂不购买开发者会员也可建项目和本地开发，但换框架不会消除系统签名、公证或运行时依赖问题。公开源码仓库已创建并推送；付费签名和正式安装包发布尚未执行。
