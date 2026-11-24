# chrono-harness-worktree

创建、管理与清理任务工作树。

运行 `./check.sh` 验证独立组件。证明使用 Verus 0.2026.04.24.f8e1704，通过环境变量 `VERUS` 指定可执行文件。

[固定来源](https://github.com/ChronoAIProject/chrono-harness/tree/5f6892dfa2f73675a84e4ebdc3b06eb83b824d6e/crates/worktree)

## 到期工作树在哪一轮进入清理

`maintain` 枚举协调者登记的工作树，按路径排序并逐项处理。对普通关联工作树，年龄取自物理目录创建时间。宿主配置复用窗口 R 和删除窗口 D，要求 0 < R < D。协调者和仅管理缓存的目录由各自的策略清理。[枚举](crates/worktree/src/automatic/service.rs) · [年龄读取](crates/worktree/src/automatic/age.rs)

**单次访问的保证。** 对已登记、状态为 active、没有已启动退役意图的普通工作树，设本轮读到目录年龄 a。程序在 a > R 时开始复用，在 a > D 时允许按年龄删除。恰好 a = D 时仅通过复用年龄判断。由于 R < D，任何读到 a > D 的访问都同时通过这两项年龄判断，并将工作树送入本轮租约与保全检查，即使没有待清缓存。Verus 直接验证生产代码使用的这两项判定；专属证明覆盖任意有序窗口、阈值相等和越过一纳秒的观察。[年龄判定](crates/worktree/src/automatic/age.rs) · [本轮选择](crates/worktree/src/automatic.rs) · [Verus 合同](crates/worktree/src/verified_lifecycle.rs) · [专属证明](crates/worktree-tests/tests/verified_lifecycle.rs)

**实际删除的条件。** 维护程序必须能读取登记与创建时间，取得排他租约，确认没有活跃或状态未知的使用方。它将需保全的提交、暂存区和源码改动保存到协调者 Git 引用，复核检出身份和清理资格，再移除符合条件的工作树。受保护的文件、不可用的退役原件或持续持有的租约都可以推迟删除。即使维护每小时运行，只要租约一直由使用方持有，工作树就可以一直存在。[源码保全与退役](crates/worktree/src/automatic/age.rs)

**从单轮到时间上界。** 取删除窗口已严格过去、且处置条件开始持续满足的时刻 t。设一次完整维护在 S 时间内取得包含该目录的有限登记快照，目录在快照中的位置为 k（从 0 起）。如果每项访问耗时至多 B，目标目录处置和复核至多 C，则至迟在 t + S + (k + 1) × B + C 可观察到该工作树和其中缓存消失。生产游标每次恰好前进一项，最后一个地址须经 k + 1 步；专属证明将该步数与严格年龄阈值、上述外部时间界连接。完成以实际路径不存在且 Git 无冲突登记为准。[遍历与处置](crates/worktree/src/automatic/service.rs) · [删除确认](crates/worktree/src/maintenance.rs) · [步数与时间界](crates/worktree-tests/tests/verified_lifecycle.rs)

先前受租约保护的轮数可以是任意有限值。这里的 S 包括下一次成功调度和取得其完整快照前的等待；B 包括前序地址的处理开销。使用方持续释放、转存或无需保留、源码已保全、时钟与目录身份稳定、Git 和文件系统操作成功，并且此后没有重新创建同一目录，是该条件界的前提。服务停机、始终持锁、权限失败或登记持续漏项时，可保留目录且没有固定墙钟上界。

示例仓在 macOS 用户会话采用 launchd，将复用窗口缩为 23 秒，删除和原件保留窗口设为 24 秒。六轮定时维护成功，工作树、缓存和到期报告均已回收。另一次把原件保留窗口缩为 3 秒的实验中，退役记录在消费过程中到期，三轮维护报告失败。在 Ubuntu 用户会话中，systemd 定时器实际触发了到期工作树、Python 缓存和 PR 提醒的回收，并保留了仍有效的提醒。[macOS 验收](https://github.com/chrono-harness/chrono-harness-example/pull/29) · [Ubuntu 定时运行](https://github.com/chrono-harness/chrono-harness-example/actions/runs/45231805116)

## 复用本地完整检查的上下文

为单元或收集操作复用已有上下文时，worktree 调用已安装的 `chrono-judge-workflow check-observation-age`。请在工作树外安装本仓 mise 声明的固定发行，使可执行文件在 worktree 的 PATH 上可用。宿主有单独安装位置时，可在 worktree 配置的 `check_inputs.observation_age_program` 填写可执行文件路径。

workflow 判断原始 UTC 时间与宿主设定的 `max_age_hours`。上下文相同且仍有效时，worktree 保留原观察时间。过期时生成本轮上下文。无效时间、无效阈值或命令失败时，准备操作报告原始原因。

工作树读取宿主登记时，通过已安装的 `chrono-judge-registration worktree-inputs-v1` 校验固定 Git 输入。默认从 PATH 解析判官；宿主可以在工作树配置中用 `registration_program` 绑定自己的安装路径。调用方管理 Git 读取和工作树状态，判官从 stdin 接收登记数据并返回选定的配置、FILEMAP 或路径。使用 mise 的宿主分别固定两个命令的发行。

单独选装工作树可以使用 `chrono-worktree-config/v3`，省略 `registration_program`，由工作树读取固定 Git 提交中登记的业务目录、文件路径和分支。配置了判官路径的宿主使用同一工作树操作并交给已安装的判官校验完整登记。v1/v2 的安装选择仍按已有配置解析。原件保管和文件日志分别由宿主的 retention.json 和 logging.json 启用。

### 查询已采用的工作树程序

`chrono-worktree adopted-owner-v1 ROOT SELECTED [--if-installed [COMMIT]]` 供 bootstrap 核对已部署的生命周期程序。ROOT 是宿主绝对路径，SELECTED 是宿主登记的程序相对路径。查询核对采用收据、发布意图、发布结果以及原程序、暂存程序和选中程序的文件身份。成功时输出 JSON，含构建报告的来源提交、位置、摘要、读取该报告的槽位、暂存记录及选中程序身份。没有采用记录时输出 `null`。加 `--if-installed` 后，当前程序与记录不符也返回 `null`。给定 COMMIT 时只匹配该来源提交。已匹配的记录损坏时退出 2，错误写到 stderr。

## 清理宿主的滚动日志

采用 `.chrono-harness/logging.json` 的宿主须在工作树外安装独立发行的 `chrono-diagnostics`。维护时按主检出的 `diagnostics_program` 或 PATH 选择该命令，同步调用 `expire-logs-v1 ROOT` 核对每次结果。日志策略归该命令解释，部分删除失败进入工作树维护报告；没有日志配置时无需安装该程序。

宿主采用 `.chrono-harness/retention.json` 时，worktree 在删除业务目录前向已安装的 `chrono-harness retained-artifacts-v1` 查询这些路径。保管进程检查仍被消费者占用的原件，并把目录删除租约的文件描述符交给 worktree。回收前会再次核对租约。请在工作树外安装本仓 mise 配置选定的 runner 发行。worktree 从 PATH 查找 runner。使用系统 PATH 的定时服务可从宿主的工作树程序链接旁加载同一套安装中的 runner。

单独选装 worktree 时，v3 主机在已登记的报告业务目录中保存工作树报告和进程原件。运行 `maintain` 时，`terminal_history.keep_seconds` 同时决定单工具报告的保留时间。省略该字段时报告默认保留 24 小时。需要长期使用的报告须在窗口内复制到持久存储。活跃进程持有内核租约，仍由现行登记或保留窗口内报告引用的原件继续保留。到期且最后一个引用结束后，由下一次取得目录租约的成功维护轮次删除。宿主配置定时维护后，这一过程自动运行。已配置通用原件保管的宿主使用其登记的保留策略。

## 多个命令复用工作树

宿主可在协调者的 cleanup.json 中，为已登记的 operation 声明此次命令读写的资源。use、check 和 bootstrap 从同一份协调者配置取范围。下面的两个命令可以并行读取源码，写入文档的操作和读取同一文档的操作会互斥。

```json
{
  "operation_scopes": {
    "example.check": {"reads": ["path:src"], "writes": ["path:build"]},
    "docs.preview": {"reads": ["path:src", "path:docs"], "writes": ["path:preview"]},
    "docs.edit": {"reads": [], "writes": ["path:docs"]}
  }
}
```

`path:` 后是相对当前工作树的文件或目录。目录范围包括后代。用 `checkout` 声明整棵树。Git 的 index、HEAD、当前分支与指定的公共引用分别用 `git:index`、`git:head`、`git:branch` 和 `git:refs/heads/名字` 声明。同一仓库中的两个检出有各自的 index，可以竞争同一个实际分支引用。维护方应为实际会写 index 和 ref 的复合操作同时登记这两个资源。只有通过本入口进入并获得保护的命令参加调度；其他 Git 客户端仍使用 Git 自己的锁。

启用此配置后，缺少范围的命令保守地占用当前工作树、其 Git 元数据和公共引用。具体命令只在与当前消费者的读写范围冲突时被拒绝，最后一个仍持有能力的子孙进程结束后可以重新进入。临时声明位于协调者的 operation-scopes 业务目录。正常返回、后续操作和定时维护都可回收到期声明。机器或服务中断期间没有墙钟清理上界。
