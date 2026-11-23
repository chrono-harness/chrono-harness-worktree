# chrono-harness-worktree

创建、管理与清理任务工作树。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/worktree)

## 复用本地完整检查的上下文

为单元或收集操作复用已有上下文时，worktree 调用已安装的 `chrono-judge-workflow check-observation-age`。请在工作树外安装本仓 mise 声明的固定发行，使可执行文件在 worktree 的 PATH 上可用。宿主有单独安装位置时，可在 worktree 配置的 `check_inputs.observation_age_program` 填写可执行文件路径。

workflow 判断原始 UTC 时间与宿主设定的 `max_age_hours`。上下文相同且仍有效时，worktree 保留原观察时间。过期时生成本轮上下文。无效时间、无效阈值或命令失败时，准备操作报告原始原因。

工作树读取宿主登记时，通过已安装的 `chrono-judge-registration worktree-inputs-v1` 校验固定 Git 输入。默认从 PATH 解析判官；宿主可以在工作树配置中用 `registration_program` 绑定自己的安装路径。调用方管理 Git 读取和工作树状态，判官从 stdin 接收登记数据并返回选定的配置、FILEMAP 或路径。使用 mise 的宿主分别固定两个命令的发行。

### 查询已采用的工作树程序

`chrono-worktree adopted-owner-v1 ROOT SELECTED [--if-installed [COMMIT]]` 供 bootstrap 核对已部署的生命周期程序。ROOT 是宿主绝对路径，SELECTED 是宿主登记的程序相对路径。查询核对采用收据、发布意图、发布结果以及原程序、暂存程序和选中程序的文件身份。成功时输出 JSON，含构建报告的来源提交、位置、摘要、读取该报告的槽位、暂存记录及选中程序身份。没有采用记录时输出 `null`。加 `--if-installed` 后，当前程序与记录不符也返回 `null`。给定 COMMIT 时只匹配该来源提交。已匹配的记录损坏时退出 2，错误写到 stderr。
