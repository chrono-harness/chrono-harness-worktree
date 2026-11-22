# chrono-harness-worktree

创建、管理与清理任务工作树。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/worktree)

## 复用本地完整检查的上下文

为单元或收集操作复用已有上下文时，worktree 调用已安装的 `chrono-judge-workflow check-observation-age`。请在工作树外安装本仓 mise 声明的固定发行，使可执行文件在 worktree 的 PATH 上可用。宿主有单独安装位置时，可在 worktree 配置的 `check_inputs.observation_age_program` 填写可执行文件路径。

workflow 判断原始 UTC 时间与宿主设定的 `max_age_hours`。上下文相同且仍有效时，worktree 保留原观察时间。过期时生成本轮上下文。无效时间、无效阈值或命令失败时，准备操作报告原始原因。
