# 公共约定和工具入口

开发分支为 `dev`。日常 PR 合入开发分支，正式版从 `main` 发布。

首次检出运行 `mise trust` 和 `mise install`。工具版本见 [.config/mise/conf.d/chrono-harness.toml](../.config/mise/conf.d/chrono-harness.toml)。


用 `mise exec -- chrono-pr pr-reminders` 查看本地提醒，再运行 `mise exec -- chrono-pr pr-status --pr NUMBER --head-sha SHA` 查询 GitHub。


跨项目通过固定发行的命令或公开接口协作。等待任务和 CI 使用完成事件或通知。安装包与缓存放在工作树外，有长期用途的产物在清理窗口内转存。
