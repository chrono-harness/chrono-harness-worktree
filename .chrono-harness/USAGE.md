# 本仓的 harness

开发分支为 `dev`，正式版从 `main` 发布。

在仓库根目录运行 `mise install`。版本和两平台 SHA-256 位于 `.config/mise/conf.d/chrono-harness.toml`。

- 用 `mise exec -- chrono-pr pr-reminders` 查看本地待跟进 PR，再用 `pr-status --pr NUMBER --head-sha SHA` 读取 GitHub 的当前状态。

- 采用指令登记的项目可用 `mise exec -- chrono-instructions init --host-root .` 初始化，编辑登记源后运行 `mise exec -- chrono-instructions generate --host-root .`。

- 采用登记式检查的宿主按其配置运行 `mise exec -- chrono-harness check`。

- 用 `mise exec -- chrono-worktree --help` 查看工作树管理命令。

- 用 `mise exec -- chrono-ci --help` 查看工作流生成与校验命令。

- 原生验证入口可从 PATH 使用本仓锁定的 `verus`。
