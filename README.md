# chrono-harness-worktree

创建、管理与清理任务工作树。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/worktree)

## 到期工作树在哪一轮进入清理

`maintain` 枚举协调者登记的工作树，按路径排序并逐项处理。对普通关联工作树，年龄取自物理目录创建时间。宿主配置复用窗口 R 和删除窗口 D，要求 0 < R < D。协调者和仅管理缓存的目录由各自的策略清理。[枚举](crates/worktree/src/automatic/service.rs) · [年龄读取](crates/worktree/src/automatic/age.rs)

**单次访问的保证。** 对已登记、状态为 active、没有已启动退役意图的普通工作树，设本轮读到目录年龄 a。程序在 a > R 时开始复用，在 a > D 时允许按年龄删除。恰好 a = D 时仅通过复用年龄判断。由于 R < D，任何读到 a > D 的访问都同时通过这两项年龄判断，并将工作树送入本轮租约与保全检查，即使没有待清缓存。Verus 直接验证生产代码使用的这两项判定；专属证明覆盖任意有序窗口、阈值相等和越过一纳秒的观察。[年龄判定](crates/worktree/src/automatic/age.rs) · [本轮选择](crates/worktree/src/automatic.rs) · [Verus 合同](crates/worktree/src/verified_lifecycle.rs) · [专属证明](crates/worktree-tests/tests/verified_lifecycle.rs)

**实际删除的条件。** 维护程序必须能读取登记与创建时间，取得排他租约，确认没有活跃或状态未知的使用方。它将需保全的提交、暂存区和源码改动保存到协调者 Git 引用，复核检出身份和清理资格，再移除符合条件的工作树。受保护的文件、不可用的退役原件或持续持有的租约都可以推迟删除。即使维护每小时运行，只要租约一直由使用方持有，工作树就可以一直存在。[源码保全与退役](crates/worktree/src/automatic/age.rs)

**从单轮到时间上界。** 取删除窗口已严格过去、且全部删除条件开始持续满足的时刻 t。若宿主保证在随后的 P 时间内成功访问这条登记，并能在访问后的 C 时间内完成该次处置，则至迟在 t + P + C 删除。P 包含调度、枚举和同轮较早地址的耗时。维护遍历本轮取得的地址；若调度停止、登记始终不可读、磁盘操作一直失败或租约持续被占用，就无法从间隔配置推出 P 或 C。宿主需要实测定时触发、Git 保全及最终文件消失。

示例仓在 macOS 用户会话采用 launchd，将复用窗口缩为 23 秒，删除和原件保留窗口设为 24 秒。六轮定时维护成功，工作树、缓存和到期报告均已回收。另一次把原件保留窗口缩为 3 秒的实验中，退役记录在消费过程中到期，三轮维护报告失败。Linux 的 systemd 用户定时器仍待原生验收。[示例验收](https://github.com/chrono-harness/chrono-harness-example/pull/29)

## 复用本地完整检查的上下文

为单元或收集操作复用已有上下文时，worktree 调用已安装的 `chrono-judge-workflow check-observation-age`。请在工作树外安装本仓 mise 声明的固定发行，使可执行文件在 worktree 的 PATH 上可用。宿主有单独安装位置时，可在 worktree 配置的 `check_inputs.observation_age_program` 填写可执行文件路径。

workflow 判断原始 UTC 时间与宿主设定的 `max_age_hours`。上下文相同且仍有效时，worktree 保留原观察时间。过期时生成本轮上下文。无效时间、无效阈值或命令失败时，准备操作报告原始原因。

工作树读取宿主登记时，通过已安装的 `chrono-judge-registration worktree-inputs-v1` 校验固定 Git 输入。默认从 PATH 解析判官；宿主可以在工作树配置中用 `registration_program` 绑定自己的安装路径。调用方管理 Git 读取和工作树状态，判官从 stdin 接收登记数据并返回选定的配置、FILEMAP 或路径。使用 mise 的宿主分别固定两个命令的发行。

### 查询已采用的工作树程序

`chrono-worktree adopted-owner-v1 ROOT SELECTED [--if-installed [COMMIT]]` 供 bootstrap 核对已部署的生命周期程序。ROOT 是宿主绝对路径，SELECTED 是宿主登记的程序相对路径。查询核对采用收据、发布意图、发布结果以及原程序、暂存程序和选中程序的文件身份。成功时输出 JSON，含构建报告的来源提交、位置、摘要、读取该报告的槽位、暂存记录及选中程序身份。没有采用记录时输出 `null`。加 `--if-installed` 后，当前程序与记录不符也返回 `null`。给定 COMMIT 时只匹配该来源提交。已匹配的记录损坏时退出 2，错误写到 stderr。

## 清理宿主的滚动日志

采用 `.chrono-harness/logging.json` 的宿主须在工作树外安装独立发行的 `chrono-diagnostics`。维护时按主检出的 `diagnostics_program` 或 PATH 选择该命令，同步调用 `expire-logs-v1 ROOT` 核对每次结果。日志策略归该命令解释，部分删除失败进入工作树维护报告；没有日志配置时无需安装该程序。
