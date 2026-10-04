const messages = {
  "zh-CN": {
    hosts: { title: "工具资源", subtitle: "查看各工具当前的用户级能力，区分已纳入 AgentHub 的资源与工具自有内容，并安全清理可写资源。", rescan: "重新扫描", chooseTarget: "选择工具", scanning: "正在读取工具资源…", resources: "项资源", canonicalMatch: "已纳入 AgentHub", hostOnly: "工具自有", constraint: "受保护资源", legendHint: "已纳入表示与库中内容一致；受保护资源由工具官方管理。", canonicalId: "AgentHub ID", notCanonical: "AgentHub 中没有对应内容", empty: "未发现资源", emptyHint: "当前工具未配置此类资源。", selected: "项已选择", cleanupHint: "删除前会备份；AgentHub 库内容不会改变。", deleteSelected: "删除所选工具资源", deleteTitle: "删除这些工具资源？", deleteBody: "AgentHub 会重新扫描并备份所有受影响路径，然后只删除列出的资源。失败时会自动尝试恢复。", deleteAction: "备份并删除", driftWarning: "其中包含已纳入 AgentHub 的资源。删除会产生差异；已开启的自动同步可能在下次 AgentHub 修改时重新创建它。", deleted: "项工具资源已删除并备份", selectUnimported: "一键选中未纳入 AgentHub 的资源", noUnimported: "没有未纳入 AgentHub 的可删除资源", clearSelection: "取消选择", cleanupAck: "我已核对以上资源，确认从该工具中删除（可从备份恢复）", relations: { canonical_match: "已纳入 AgentHub", host_only: "工具自有", constraint: "受保护" }, constraints: { claude_plugins_cli_managed: "Claude Code 插件由官方 CLI 管理，禁止直接删除目录。", codex_plugins_cli_managed: "Codex 插件市场与缓存由官方 CLI 管理，禁止直接删除目录。" } },
    appName: "AgentHub", skip: "跳到主要内容", nav: { overview: "总览", inventory: "我的能力库", hosts: "工具资源", sync: "同步到工具", git: "版本记录", transactions: "同步记录", settings: "设置与诊断" },
    overview: { eyebrow: "AGENTHUB", title: "能力管理中心", subtitle: "一处维护，清晰预览，安全同步到每个开发环境。", pending: "有未保存的版本", clean: "已保存为版本", recent: "最近同步记录", emptyTx: "还没有同步记录。生成预览后，每次应用与回滚都会出现在这里。", flowTitle: "从 AgentHub 库流向每个工具", channels: "个工具已同步", autoChannels: "个自动同步工具", resources: "项能力", capabilities: "资源分类", ready: "已同步", autoReady: "自动同步", notEnabled: "未启用", historyLayer: "版本记录", stateLayer: "本机状态", allCapabilities: "全部能力", totalResources: "AgentHub 库资源总数" },
    kinds: { skill: "Skills", mcp: "MCP", plugin: "Plugins", rule: "Rules" }, targets: { agents: "共享 Agents", cursor: "Cursor", codex: "Codex", claude: "Claude Code" },
    common: { loading: "正在读取本机状态…", retry: "重试", close: "关闭", cancel: "取消", save: "保存更改", search: "搜索能力", clearSearch: "清除搜索", view: "查看", edit: "编辑", logHint: "诊断日志位置：" },
    inventory: { title: "我的能力库", subtitle: "集中管理 Skills、MCP、插件和规则，按需同步到开发工具。", empty: "这里还没有内容", emptyHint: "从工具扫描导入，或新建规则开始使用。", digest: "内容摘要", importRule: "导入规则", newRule: "新建规则", editRule: "修改规则", delete: "删除能力", deleteTitle: "从 AgentHub 删除这项能力？", deleteBody: "该能力会从 AgentHub 库删除；已启用自动同步且包含它的目标会通过带备份的同步删除对应投影。其他未选择的工具能力不会被直接修改。", deleteAction: "删除并同步已启用目标", importMarkdown: "选择 Markdown", ruleFormHint: "文件只用于初始化草稿，确认保存后才会写入 AgentHub 库。", id: "稳定唯一标识（ID）", idPlaceholder: "例如：team-safety", idHint: "它是 AgentHub 库目录名，也是同步引用和历史追踪的固定身份。仅使用小写字母、数字、短横线或下划线；创建后不能修改。", displayName: "显示名称", activation: "激活方式", activationAlways: "始终启用", activationManual: "手动引用", activationPaths: "指定路径", paths: "适用路径", pathsHint: "每行一个相对路径或 glob。", targets: "兼容目标", body: "规则正文", bodyHint: "使用 Markdown 编写给 Agent 的明确规则。", saveRule: "保存规则", saving: "正在保存…", required: "请填写这个字段。", invalidId: "ID 必须是小写 slug，最长 80 个字符。", targetRequired: "至少选择一个兼容目标。", pathRequired: "按路径激活时至少填写一个路径。", fileTooLarge: "Markdown 文件不能超过 1 MB。", discardTitle: "放弃未保存的规则？", discardBody: "当前草稿中的修改不会写入 AgentHub 库。", keepEditing: "继续编辑", discard: "放弃修改", loadFailed: "无法读取规则，请重试。", importedFile: "已从文件初始化草稿", viewDetails: "查看详情", detailLoadFailed: "无法读取能力详情，请重试。", files: "个文件", fileInventory: "文件清单", pluginPreviewHint: "插件仅预览 AgentHub Manifest 和文件清单；payload 不会被执行或直接渲染。", previewTruncated: "预览已截断" },
    sync: { title: "同步到工具", subtitle: "先选择目标和范围；开启自动同步后，AgentHub 的后续修改会立即投影到该目标。", plan: "预览变更", planning: "正在计算…", apply: "备份并同步", warning: "同步会删除所选资源类别中工具多出的资源，但不会改动模型、主题、账户或权限等设置。", noPlan: "预览一次，或开启自动同步", noPlanHint: "预览用于审阅当前差异；自动同步会保存这次范围，并在每次 AgentHub 写入后自动备份、同步和验证。", steps: "项文件变更", confirmTitle: "确认同步", confirmBody: "AgentHub 会先完整备份可写资源类别，再写入并重新验证；任何验证失败都会自动回滚。", autoConfirmTitle: "开启自动同步？", autoConfirmBody: "请再次核对下方能力清单。AgentHub 会先同步当前范围，成功后保存；以后仅自动同步这些能力。不会扫描或合并工具侧手工修改。", autoConfirmAction: "同步当前范围并开启", disableAutoTitle: "关闭自动同步？", disableAutoBody: "以后修改 AgentHub 时将不再自动更新此目标。工具当前内容和已有恢复记录不会改变。", disableAutoAction: "关闭自动同步", chooseTarget: "选择同步目标", review: "审阅变更", backup: "创建备份", verify: "写入并验证", ready: "变更预览已就绪", canonicalSnapshot: "库版本摘要", capabilityChanges: "项能力变更", affected: "项受影响", create: "新增", update: "替换", delete: "删除", skip: "跳过", files: "个文件", exactChanges: "具体能力变更", domainProjection: "整体替换的能力类别", noneSelected: "未选择", fileDetails: "文件变更明细", noChanges: "目标已与 AgentHub 库一致", scopeTitle: "选择同步范围", scopeHint: "逐项选择要同步的能力；该清单同时用于预览和自动同步，已启用目标修改后需要保存。", scopeDeleteHint: "取消选择后预览变更，可先看到目标中的对应删除项；只有确认执行或保存自动同步范围后才会修改工具。", selected: "项已选择", selectAll: "全部选择", clearAll: "清空选择", rulesToggle: "同步 AgentHub 库 Rules 到目标", included: "已包含", excluded: "不触碰", enableAuto: "开启自动同步", updateAuto: "保存自动同步范围", disableAuto: "关闭自动同步", autoEnabled: "自动同步已开启", autoOn: "自动同步中" },
    git: { title: "版本记录", subtitle: "由你决定何时为 AgentHub 库保存一个版本（底层使用 Git）；同步不要求先保存版本。", status: "未保存的改动", diff: "改动内容", branch: "AgentHub 库", clean: "没有未保存的改动", noDiff: "当前没有未保存的改动", commitTitle: "保存一个版本", commitHint: "保存能力库当前内容；连接远程仓库后会自动合并并上传版本。", message: "版本说明", messagePlaceholder: "例如：添加团队安全规则", identity: "作者信息", name: "姓名", email: "邮箱", identityHint: "用于标记版本作者，仅保存在本机；与仓库登录账号无关。", commit: "保存版本", committing: "正在保存…", history: "历史版本", noHistory: "还没有保存过版本", nothingToCommit: "当前没有需要保存的改动。" },
    transactions: { subtitle: "每次实际写入的备份、验证和回滚结果都保留在本机；连续自动同步会折叠显示。", emptyTitle: "还没有同步记录", emptyBody: "当你第一次把 AgentHub 库同步到目标后，同步记录会按时间出现在这里。", rollback: "恢复到此次同步前", rollbackApply: "恢复到此次同步前", rollbackRollback: "恢复到此次回滚前", rollbackTitle: "恢复工具历史状态？", rollbackBody: "AgentHub 将先备份目标当前状态，再恢复这次同步执行前的可写资源类别并验证。AgentHub 库不会改变，恢复后会产生可检测的差异。", rollbackAction: "备份当前状态并恢复", rollingBack: "正在恢复…", rollbackDone: "工具状态已恢复并验证", historyLoadFailed: "无法读取完整同步记录。", verifiedApply: "写入结果已重新读取并验证", verifiedRollback: "历史状态已恢复并验证", backupAvailable: "本机备份可用", modeReviewed: "预览确认", modeAuto: "自动同步", modeRollback: "手动回滚", autoBatch: "自动同步", runs: "次", statusApplied: "已应用", statusRollbackApplied: "已恢复", statusRolledBack: "已自动回滚", statusRollbackFailed: "回滚失败", statusApplying: "正在应用", statusRollingBack: "正在回滚" },
    settings: { subtitle: "管理能力库、本机设置与工具同步，查看诊断信息。", canonical: "AgentHub 库", canonicalHint: "版本记录由 Git 保存", database: "本机状态", databaseHint: "变更预览、同步记录、目标与加密秘密索引", targets: "同步通道", targetsHint: "已启用的桌面工具目标", logs: "诊断日志", logsHint: "发生错误时可复制此目录中的最新日志", loadingPath: "正在读取日志位置…", gitReady: "Git 可用", gitMissing: "未检测到 Git", enabled: "个已启用", localOnly: "仅保存在本机", healthy: "状态正常" },
    init: { title: "建立 AgentHub 库", subtitle: "扫描各工具的用户级资源，选择要导入的内容。后续同步需确认范围，才会修改工具文件。", scan: "开始全局扫描", import: "导入所选并完成", importing: "正在验证并导入…", found: "发现的用户级资源", groups: "能力分类", discovered: "项发现", selectGroup: "选择本类", clearGroup: "清空本类", emptyGroup: "未发现此类能力", emptyGroupHint: "当前工具的用户目录中未发现此类资源。", duplicate: "与另一来源内容相同；同时选择也只会导入一份", none: "未发现可导入资源，也可以创建空的 AgentHub 库。", selectAll: "全选", clear: "清空选择", selected: "已选择", importable: "可导入", rejected: "需处理", notImportable: "不可导入", rule_empty: "规则文件为空，请先补充内容", rule_too_large: "规则文件超过 1 MB", rule_invalid_encoding: "规则文件不是 UTF-8 编码", recover: "清理未完成导入", recoverTitle: "清理未完成的初始化？", recoverBody: "只会删除 AgentHub 库中上次失败产生的副本，不会修改 Cursor、Codex 或 Claude Code。", recoverAction: "清理并重新扫描", legacyReset: "重置失败的初始化", legacyResetConfirm: "将删除上次失败导入到 AgentHub 的 AgentHub 库副本并重新开始。不会修改任何工具。确定继续吗？" },
    toast: { planReady: "变更预览已生成", applied: "已同步并验证", autoEnabledSynced: "当前状态已同步，后续修改将自动同步", autoEnabled: "目标已是最新状态，自动同步已开启", autoDisabled: "自动同步已关闭", ruleSaved: "规则已保存到 AgentHub 库", ruleSavedAutoSynced: "规则已保存并自动同步到", ruleSavedAutoFailed: "规则已保存，但自动同步失败：", capabilityDeleted: "能力已从 AgentHub 库删除", capabilityDeletedAutoFailed: "能力已删除，但自动同步失败：", committed: "已保存新版本", failed: "操作失败" }
  },
  en: {
    appName: "AgentHub", skip: "Skip to main content", nav: { overview: "Overview", inventory: "My library", sync: "Sync to tools", git: "Versions", transactions: "Sync history", settings: "Settings & diagnostics" },
    overview: { eyebrow: "AGENTHUB", title: "Capability switchboard", subtitle: "Maintain once, review clearly, and sync safely to every development environment.", pending: "Unsaved version", clean: "Saved as a version", recent: "Recent transactions", emptyTx: "No sync transactions yet. Plans, applies, and rollbacks will appear here.", flowTitle: "From the AgentHub library to every tool", channels: "channels synced", autoChannels: "automatic targets", resources: "capabilities", capabilities: "Capability flow", ready: "Synced", autoReady: "Automatic sync", notEnabled: "Not enabled", historyLayer: "Saved versions", stateLayer: "business state", allCapabilities: "All capabilities", totalResources: "Library resources" },
    kinds: { skill: "Skills", mcp: "MCP", plugin: "Plugins", rule: "Rules" }, targets: { agents: "Shared Agents", cursor: "Cursor", codex: "Codex", claude: "Claude Code" },
    common: { loading: "Reading local state…", retry: "Retry", close: "Close", cancel: "Cancel", save: "Save changes", search: "Search capabilities", clearSearch: "Clear search", view: "View", edit: "Edit", logHint: "Diagnostic logs: " },
    inventory: { title: "My library", subtitle: "Manage Skills, MCP, plugins and rules in one library, then sync to your tools.", empty: "Nothing here yet", emptyHint: "Add it to library and it will appear here.", digest: "Content digest", importRule: "Import rule", newRule: "New rule", editRule: "Edit rule", delete: "Delete capability", deleteTitle: "Delete this capability from AgentHub?", deleteBody: "The capability is removed from library. Enabled automatic-sync targets whose scope includes it remove the projection through a backed-up transaction. Other unselected host capabilities are not edited directly.", deleteAction: "Delete and sync enabled targets", importMarkdown: "Choose Markdown", ruleFormHint: "The file only initializes a draft. library is written after you explicitly save.", id: "Stable unique ID", idPlaceholder: "For example: team-safety", idHint: "This is the library directory name and the stable identity used by sync references and history. Use lowercase letters, numbers, hyphens, or underscores only; it cannot change after creation.", displayName: "Display name", activation: "Activation", activationAlways: "Always on", activationManual: "Manual reference", activationPaths: "Specific paths", paths: "Applicable paths", pathsHint: "One relative path or glob per line.", targets: "Compatible targets", body: "Rule body", bodyHint: "Write clear instructions for the agent in Markdown.", saveRule: "Save rule", saving: "Saving…", required: "This field is required.", invalidId: "ID must be a lowercase slug with no more than 80 characters.", targetRequired: "Choose at least one compatible target.", pathRequired: "Path activation requires at least one path.", fileTooLarge: "Markdown files must be no larger than 1 MB.", discardTitle: "Discard unsaved rule?", discardBody: "Changes in this draft will not be written to library.", keepEditing: "Keep editing", discard: "Discard changes", loadFailed: "The rule could not be loaded. Try again.", importedFile: "Draft initialized from file", viewDetails: "View details", detailLoadFailed: "Capability details could not be loaded. Try again.", files: "files", fileInventory: "File inventory", pluginPreviewHint: "Plugins preview only the AgentHub manifest and file inventory. Payloads are neither executed nor rendered.", previewTruncated: "Preview truncated" },
    sync: { title: "Sync to tools", subtitle: "Choose a target and scope. Once enabled, later AgentHub changes are projected automatically.", plan: "Preview changes", planning: "Calculating…", apply: "Back up and apply", warning: "Sync removes extra host resources inside the selected domains while preserving models, themes, accounts, permissions, and other settings.", noPlan: "Preview once, or enable automatic sync", noPlanHint: "Plan reviews the current difference. Automatic sync saves this scope and backs up, syncs, and verifies after every AgentHub write.", steps: "file changes", confirmTitle: "Confirm full replacement", confirmBody: "AgentHub fully backs up writable capability domains, writes, then validates. Any verification failure rolls back automatically.", autoConfirmTitle: "Enable automatic sync?", autoConfirmBody: "Review the named scope below. AgentHub syncs it once, then saves it so only those capabilities are automatically synchronized. Host-side edits are not scanned or merged.", autoConfirmAction: "Sync this scope and enable", disableAutoTitle: "Disable automatic sync?", disableAutoBody: "Future AgentHub changes will no longer update this target automatically. Current host content and existing rollback transactions remain unchanged.", disableAutoAction: "Disable automatic sync", chooseTarget: "Choose a target", review: "Review changes", backup: "Create backup", verify: "Write and verify", ready: "Preview ready", canonicalSnapshot: "library snapshot", capabilityChanges: "capability changes", affected: "Affected", create: "Create", update: "Replace", delete: "Delete", skip: "Skip", files: "files", exactChanges: "Exact capability changes", domainProjection: "Combined capability domain", noneSelected: "None selected", fileDetails: "File change details", noChanges: "Target matches library", scopeTitle: "Choose sync scope", scopeHint: "Select individual capabilities for both Plans and automatic sync. Save the scope after changing an enabled target.", scopeDeleteHint: "Deselect a capability and preview the Plan to see its target deletion. The host changes only after Apply or saving the automatic scope.", selected: "selected", selectAll: "Select all", clearAll: "Clear all", rulesToggle: "Sync library Rules to the target", included: "Included", excluded: "Do not touch", enableAuto: "Enable automatic sync", updateAuto: "Save automatic sync scope", disableAuto: "Disable automatic sync", autoEnabled: "Automatic sync enabled", autoOn: "Automatic sync on" },
    git: { title: "Versions", subtitle: "You decide when to save a version of the AgentHub library (backed by Git); syncing does not require a saved version.", status: "Unsaved changes", diff: "Changed content", branch: "AgentHub library", clean: "No unsaved changes", noDiff: "No unsaved changes", commitTitle: "Save a version", commitHint: "Only AgentHub library files are saved. No host is synced or modified.", message: "Version note", messagePlaceholder: "For example: add team safety rules", identity: "Author", name: "Name", email: "Email", identityHint: "Stored only in the local Git config for ~/.agenthub.", commit: "Save version", committing: "Saving…", history: "Saved versions", noHistory: "No versions saved yet", nothingToCommit: "There are no changes to save." },
    transactions: { subtitle: "Every real write keeps its local backup, verification, and rollback result; consecutive automatic sync runs are collapsed.", emptyTitle: "No transactions yet", emptyBody: "Your first library sync will create a timestamped transaction here.", rollback: "Restore state before transaction", rollbackApply: "Restore state before this sync", rollbackRollback: "Restore state before this rollback", rollbackTitle: "Restore historical host state?", rollbackBody: "AgentHub first backs up the target's current state, then restores and verifies the writable capability domains captured before this transaction. library is unchanged, so the restored host may intentionally drift.", rollbackAction: "Back up current state and restore", rollingBack: "Restoring…", rollbackDone: "Host state restored and verified", historyLoadFailed: "The full transaction history could not be loaded.", verifiedApply: "Written state re-read and verified", verifiedRollback: "Historical state restored and verified", backupAvailable: "Local backup available", modeReviewed: "Previewed", modeAuto: "Automatic sync", modeRollback: "Manual rollback", autoBatch: "Automatic sync", runs: "runs", statusApplied: "Applied", statusRollbackApplied: "Restored", statusRolledBack: "Auto-rolled back", statusRollbackFailed: "Rollback failed", statusApplying: "Applying", statusRollingBack: "Rolling back" },
    settings: { subtitle: "Manage your library, local settings, tool synchronization and diagnostics.", canonical: "library source", canonicalHint: "Reviewable file history tracked by Git", database: "Business state", databaseHint: "Change previews, sync history, targets and encrypted credentials", targets: "Sync channels", targetsHint: "Enabled desktop tool targets", logs: "Diagnostic logs", logsHint: "Copy the newest file from this folder when reporting an error", loadingPath: "Reading log location…", gitReady: "Git available", gitMissing: "Git not found", enabled: "enabled", localOnly: "Stored only on this device", healthy: "Healthy" },
    init: { title: "Create the library source", subtitle: "Scan user-global resources and select what to import. Tool files change only after you explicitly confirm synchronization.", scan: "Scan global resources", import: "Import selected and finish", importing: "Validating and importing…", found: "Discovered user resources", groups: "Capability groups", discovered: "discovered", selectGroup: "Select group", clearGroup: "Clear group", emptyGroup: "No capabilities in this group", emptyGroupHint: "No matching user-global resources were found for this capability type.", duplicate: "Same content as another source; selecting both still imports one copy", none: "No importable resources found. You can create an empty library source.", selectAll: "Select all", clear: "Clear selection", selected: "selected", importable: "importable", rejected: "need attention", notImportable: "Unavailable", rule_empty: "The rule file is empty; add content first", rule_too_large: "The rule file exceeds 1 MB", rule_invalid_encoding: "The rule file is not UTF-8", recover: "Clean incomplete import", recoverTitle: "Clean the incomplete initialization?", recoverBody: "This only removes copies left in AgentHub library by the failed import. Cursor, Codex and Claude Code are not changed.", recoverAction: "Clean and scan again", legacyReset: "Reset failed initialization", legacyResetConfirm: "This removes library copies from the failed AgentHub import and starts again. No host is modified. Continue?" },
    toast: { planReady: "Change preview ready", applied: "Synced and verified", autoEnabledSynced: "Current state synced; future changes will sync automatically", autoEnabled: "Target is current and automatic sync is enabled", autoDisabled: "Automatic sync disabled", ruleSaved: "Rule saved to library", ruleSavedAutoSynced: "Rule saved and automatically synced to", ruleSavedAutoFailed: "Rule saved, but automatic sync failed for:", capabilityDeleted: "Capability deleted from library", capabilityDeletedAutoFailed: "Capability deleted, but automatic sync failed for:", committed: "New version saved", failed: "Operation failed" }
  }
} as const;
const extraMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": {
    inventory: { skillChanges: "项工具侧 Skill 内容有变化", checkingSkills: "正在检查工具侧 Skill…", skillCheckFailed: "部分 Skill 未能完成检查，请重试。", skillCheckUnavailable: "项资源无法读取", reviewSkillChanges: "查看 Skill 变化", skillChangeImportHint: "这些工具中的 Skill 与 AgentHub 库不同。选择要导入的副本；原项会保留，修改后的内容作为新条目加入库。导入后是否同步到工具遵循设置中的导入同步选项。", scanImport: "扫描并导入", scanImportTitle: "从工具导入", scanImportBody: "重新扫描用户级全局位置。只把你选择的新内容复制进 AgentHub 库；相同内容会自动去重，工具不会被修改。", importScanned: "导入所选", importingScan: "正在安全导入…", alreadyCanonical: "库中已有相同内容", scanImported: "项已导入", scanSkipped: "项重复已跳过", scanAutoFailed: "个自动同步目标失败" },
    sync: { agentsHint: "共享 Skills 目录会被支持 Agent Skills 的多个工具读取；选中管理且清空列表即可显式清空该目录。", strictActive: "强制覆盖已开启", clearManaged: "清空受管域", notManaged: "不管理", cliManaged: "CLI 管理", manageDomain: "管理这个资源类别", emptyMeansClear: "选中管理但不选资源时，将清空目标对应资源类别", domainUntouched: "不读取、不写入该资源类别", rulesIncludedHint: "用全部 AgentHub 库 Rules 重建目标规则域", claudePluginConstraint: "Claude Code 的插件库由官方 plugin CLI 管理。v0.1 会保留 ~/.claude/plugins 并跳过该域，避免破坏登录、市场和已安装插件状态。", codexPluginConstraint: "Codex 插件由个人 marketplace、安装缓存与配置共同管理。v0.1 会保留这些状态并跳过插件域，避免用目录覆盖破坏市场与安装记录。" },
    settings: { coverageTitle: "覆盖策略", coverageHint: "选择让 AgentHub 库覆盖全部可写能力，还是只管理你明确选择的资源类别。", strict: "强制覆盖", scoped: "选择性管理", strictOverwrite: "AgentHub 强制覆盖所有可写资源类别", strictOverwriteHint: "下次预览或自动同步会使用全部 AgentHub 库能力；工具额外内容会删除。", syncAfterImport: "导入后自动同步到工具", syncAfterImportHint: "导入先写入 AgentHub 库，再按已启用目标的保存范围同步；关闭时只导入，不修改任何工具。", planGuard: "手动同步仍需预览确认", backupGuard: "每次写入仍先备份并验证", vendorGuard: "厂商、组织与只读能力始终保留" },
  },
  en: {
    nav: { hosts: "Tool resources" },
    hosts: { title: "Tool resources", subtitle: "Inspect user-level capabilities in each tool, distinguish library matches from host content, and safely clean writable resources.", rescan: "Rescan", chooseTarget: "Choose a host", scanning: "Reading host resources…", resources: "resources", canonicalMatch: "library match", hostOnly: "Host only", constraint: "Protected constraint", legendHint: "Relationship labels describe the current match; they do not claim file ownership.", canonicalId: "library ID", notCanonical: "No corresponding AgentHub content", empty: "No resources found", emptyHint: "This user-level host capability domain is empty.", selected: "selected", cleanupHint: "Affected paths are backed up first; library is unchanged.", deleteSelected: "Delete selected host resources", deleteTitle: "Delete these host resources?", deleteBody: "AgentHub rescans and backs up every affected path, then deletes only the listed resources. Failure triggers an automatic restore attempt.", deleteAction: "Back up and delete", driftWarning: "This includes library matches. Deletion creates drift, and automatic sync may recreate them after the next AgentHub mutation.", deleted: "host resources deleted and backed up", selectUnimported: "Select everything not in AgentHub", noUnimported: "No deletable resources outside AgentHub", clearSelection: "Clear selection", cleanupAck: "I have reviewed these resources and confirm deleting them from this tool (restorable from backup)", relations: { canonical_match: "library match", host_only: "Host only", constraint: "Protected" }, constraints: { claude_plugins_cli_managed: "Claude Code plugins are managed by the official CLI; raw directory deletion is disabled.", codex_plugins_cli_managed: "Codex marketplace and cache state are CLI-managed; raw directory deletion is disabled." } },
    inventory: { skillChanges: "tool Skill copies differ from the library", checkingSkills: "Checking tool Skills…", skillCheckFailed: "Some Skills could not be checked. Retry to refresh.", skillCheckUnavailable: "resources could not be read", reviewSkillChanges: "Review Skill changes", skillChangeImportHint: "These tool Skills differ from the library. Select copies to import; originals are retained and changed content is added as separate entries. The sync-after-import setting controls subsequent tool writes.", scanImport: "Scan & import", scanImportTitle: "Import back from hosts", scanImportBody: "Rescan user-global locations. Only selected new content is copied into library; identical content is deduplicated and hosts are never modified.", importScanned: "Import selected", importingScan: "Importing safely…", alreadyCanonical: "Identical content is already library", scanImported: "imported", scanSkipped: "duplicates skipped", scanAutoFailed: "automatic targets failed" },
    sync: { agentsHint: "This shared Skills directory is loaded by multiple Agent Skills clients. Manage it with an empty selection to explicitly clear it.", strictActive: "Strict overwrite on", clearManaged: "Clear managed domains", notManaged: "Unmanaged", cliManaged: "CLI managed", manageDomain: "Manage this domain", emptyMeansClear: "A managed domain with no selected resources is cleared on the target", domainUntouched: "This domain is neither read nor written", rulesIncludedHint: "Rebuild the target Rules domain from every library Rule", claudePluginConstraint: "Claude Code's plugin store is managed by the official plugin CLI. v0.1 preserves ~/.claude/plugins and skips this domain to protect login, marketplace, and installed-plugin state.", codexPluginConstraint: "Codex plugins span a personal marketplace, install cache, and configuration. v0.1 preserves that state and skips the plugin domain instead of corrupting marketplace records with a directory overwrite." },
    settings: { coverageTitle: "Coverage policy", coverageHint: "Choose whether AgentHub is a strict source of truth or manages only explicitly selected domains.", strict: "Strict overwrite", scoped: "Scoped management", strictOverwrite: "Force AgentHub over all writable capability domains", strictOverwriteHint: "The next Plan or automatic sync uses every library capability and deletes extra host content.", syncAfterImport: "Run automatic sync after reverse import", syncAfterImportHint: "Import writes library first, then syncs saved scopes for enabled targets. When off, import never modifies a host.", planGuard: "Manual sync still requires Plan confirmation", backupGuard: "Every write is still backed up and verified", vendorGuard: "Vendor, organization, and read-only capabilities remain protected" },
  },
};
const workflowMessages = {
  "zh-CN": {
    inventory: { batchManage: "批量管理能力库", selectedCount: "已选择", selectVisible: "选择当前结果", clearVisible: "取消当前结果", selectCategory: "选择本类", clearCategory: "取消本类", selectItem: "选择能力", deleteSelected: "删除所选能力", batchTitle: "批量删除能力", batchBody: "将从 AgentHub 库中删除下列全部能力，并在本机保留恢复副本。已启用自动同步的工具可能同时移除对应内容。删除后仍需保存版本，才会记录到 Git；不会立即上传远端。", batchDeleted: "项能力已删除", deleteBackup: "删除前的本机恢复副本：" },
    init: { scannedCount: "扫描发现", newCount: "可选新内容", emptySource: "此来源未发现资源。只扫描用户级全局位置；系统 Skills 和受保护的插件缓存暂不支持导入。", sources: "按来源工具选择", allSources: "全部来源", totalSelected: "共选择", uniqueSelected: "去重后预计导入", selectionHint: "切换来源或类别会保留选择；相同内容只导入一份。", selectSource: "选择当前来源全部", clearSource: "清空当前来源选择", skill_runtime_excluded: "已跳过本机环境和缓存，保留技能文件与依赖清单", skill_symlink: "技能文件含符号链接，暂不支持导入", skill_special_file: "技能含特殊文件，无法导入", skill_empty: "SKILL.md 为空，请先补充内容", skill_invalid_encoding: "SKILL.md 不是 UTF-8 编码", skill_unreadable: "无法读取技能文件，请检查文件和权限" },

    "hosts": {
      "quickClean": "快速清理全部可删除资源",
      "protectedHint": "受保护的官方插件、市场与缓存不会被直接删除。此操作仅清理当前工具，已启用自动同步的资源可能被重新创建。"
    },
    "git": {
      "login": "仓库登录",
      "loginTitle": "登录仓库",
      "platform": "仓库平台",
      "selfHosted": "自建 Git / Gitea",
      "username": "登录用户名",
      "token": "访问令牌",
      "showToken": "显示访问令牌",
      "hideToken": "隐藏访问令牌",
      "tokenSettings": "打开令牌管理页面",
      "tokenHintGithub": "GitHub：选择对应仓库，并授予 Contents（内容）读写权限。使用访问令牌，不使用账号密码。",
      "tokenHintGit": "自建 Git／Gitea：在账号设置中创建具有仓库读写权限的访问令牌。",
      "credentialHint": "确认后在本机加密保存，界面和 CLI 共用；不会上传到仓库。密钥保存在本机私有目录，同一系统账户可解密。",
      "verifyLogin": "验证并保存登录",
      "loginRetry": "登录并重试同步",
      "signingIn": "正在验证…",
      "loginRequired": "请输入登录用户名和访问令牌。",
      "loginDone": "读取访问已验证；上传权限将在同步时验证。",
      "savedCredential": "已在本机加密保存登录凭据",
      "forgetCredential": "删除保存的凭据",
      "forgotCredential": "已删除本机保存的凭据。系统 Git 凭据未修改。",
      "unverified": "仓库已配置，尚未验证",
      "authFailed": "认证失败或仓库无访问权限",
      "networkError": "上次同步未完成，请检查错误",
      "syncedState": "上次同步成功",
      "retryLoginHint": "本地版本已保留。可更新登录信息后重试同步。",
      "remoteTitle": "多设备同步",
      "remoteHint": "连接专用仓库后，每次保存版本都会先合并远端更新，再自动上传。其他设备可点击“同步远端”接收内容。",
      "authHint": "GitHub 和自建 Git／Gitea 可在这里使用访问令牌登录；也支持系统 Git 凭据或 SSH。姓名和邮箱仅用于标记版本作者。",
      "remoteUrl": "远程仓库地址",
      "remoteBranch": "同步分支",
      "connected": "上次验证可读取",
      "syncNow": "同步远端",
      "disconnect": "断开连接",
      "connect": "验证并连接仓库",
      "synced": "远端版本已合并并上传",
      "localSaved": "版本已保存在本机。",
      "retryHint": "认证失败可重新登录；SSH 请检查系统密钥。冲突需先处理同一资源的改动。本地版本会保留，处理后可重试同步。",
      "remoteScope": "仅同步能力库和版本记录。工具配置、登录凭据、密钥目录、备份和同步记录留在本机；可识别的 MCP 明文凭据会阻止上传，请先改为环境变量引用并清理敏感历史；接收远端后，请预览并同步到工具。首次使用另一台设备，请先保存本机版本再同步。"
    },
    "settings": {
      "recoveryLocation": "重置前的完整恢复副本：",
      "mcpCredentials": "MCP 连接凭据",
      "resetTitle": "重置 AgentHub",
      "resetHint": "清空当前能力库和本机设置，重新选择从 Cursor、Codex 等工具导入。",
      "resetAction": "重置并重新导入",
      "resetBody": "当前 AgentHub 能力库、版本记录、自动同步设置、远程连接、密钥和本机同步记录将从活动目录移除，随后返回导入向导。Cursor、Codex、Claude Code 等工具中的资源不会被修改。",
      "resetBackup": "重置前会将完整数据移至同级 .agenthub-reset-* 私有恢复目录（包含密钥）。确认不再需要后可手动删除恢复副本。",
      "resetType": "请输入 AGENTHUB 确认重置"
    }
  },
  "en": {
    inventory: { batchManage: "Manage library selection", selectedCount: "Selected", selectVisible: "Select visible results", clearVisible: "Clear visible results", selectCategory: "Select category", clearCategory: "Clear category", selectItem: "Select capability", deleteSelected: "Delete selected capabilities", batchTitle: "Delete library selection", batchBody: "Remove every listed capability from the AgentHub library and keep a local recovery copy. Enabled automatic-sync targets may remove their copies. Save a version to record the deletions in Git; this action does not upload immediately.", batchDeleted: "capabilities deleted", deleteBackup: "Local recovery copy before deletion:" },
    "init": { scannedCount: "Discovered", newCount: "Selectable new resources", emptySource: "No resources found here. Only user-global locations are scanned; system Skills and protected plugin caches cannot currently be imported.", "sources": "Choose by source tool", "allSources": "All sources", "totalSelected": "Total selected", "uniqueSelected": "Unique resources to import", "selectionHint": "Selections persist across source and category filters; identical content is imported once.", "selectSource": "Select all from this source", "clearSource": "Clear this source", skill_runtime_excluded: "Local environments and caches excluded; skill files and dependency manifests retained", skill_symlink: "Skill files contain a symlink; import is unavailable", skill_special_file: "The skill contains a special file and cannot be imported", skill_empty: "SKILL.md is empty; add content first", skill_invalid_encoding: "SKILL.md must use UTF-8 encoding", skill_unreadable: "Cannot read skill files; check files and permissions" },
    "hosts": {
      "quickClean": "Quick clean all deletable resources",
      "protectedHint": "Official plugin stores, marketplaces and caches remain protected. Only this tool is cleaned; automatic sync may recreate managed resources."
    },
    "git": {
      "login": "Repository sign-in",
      "loginTitle": "Sign in to repository",
      "platform": "Repository platform",
      "selfHosted": "Self-hosted Git / Gitea",
      "username": "Login username",
      "token": "Access token",
      "showToken": "Show access token",
      "hideToken": "Hide access token",
      "tokenSettings": "Open token settings",
      "tokenHintGithub": "GitHub: select this repository and grant Contents read and write access. Use an access token instead of your account password.",
      "tokenHintGit": "Self-hosted Git / Gitea: create an access token with repository read and write permissions in account settings.",
      "credentialHint": "Credentials are encrypted on this device and shared by the app and CLI. They are never uploaded. The key is in your local private directory; the same system account can decrypt it.",
      "verifyLogin": "Verify and save sign-in",
      "loginRetry": "Sign in and retry sync",
      "signingIn": "Verifying…",
      "loginRequired": "Enter a login username and access token.",
      "loginDone": "Read access verified. Upload permission is checked when syncing.",
      "savedCredential": "Sign-in credentials encrypted on this device",
      "forgetCredential": "Delete saved credentials",
      "forgotCredential": "Saved local credentials deleted. System Git credentials were not changed.",
      "unverified": "Repository configured, not yet verified",
      "authFailed": "Authentication failed or repository access denied",
      "networkError": "Last sync did not complete; check the error",
      "syncedState": "Last sync succeeded",
      "retryLoginHint": "Your local version is preserved. Update sign-in details and retry sync.",
      "remoteTitle": "Sync across devices",
      "remoteHint": "Connect a dedicated repository. Each saved version merges remote updates before uploading; other devices can receive updates with Sync remote.",
      "authHint": "Sign in to GitHub or self-hosted Git / Gitea here with an access token. Saved system Git credentials and SSH also work. Author name and email are separate from authentication.",
      "remoteUrl": "Remote repository URL",
      "remoteBranch": "Sync branch",
      "connected": "Read access last verified",
      "syncNow": "Sync remote",
      "disconnect": "Disconnect",
      "connect": "Verify and connect",
      "synced": "Remote versions merged and uploaded",
      "localSaved": "Version saved on this device.",
      "retryHint": "Sign in again after authentication failures, or check SSH keys. Resolve conflicting changes before retrying. Your local version is preserved.",
      "remoteScope": "Only the library and versions are shared. Tool settings, login credentials, key directories, backups and sync history stay local. Recognizable plaintext MCP credentials block uploading; use environment references and remove sensitive history first. Preview and sync received resources to tools explicitly. Save a local version before syncing a new device."
    },
    "settings": {
      "recoveryLocation": "Complete recovery copy from before reset:",
      "mcpCredentials": "MCP credentials",
      "resetTitle": "Reset AgentHub",
      "resetHint": "Clear the active library and local settings, then import again from Cursor, Codex and other tools.",
      "resetAction": "Reset and import again",
      "resetBody": "The active library, versions, automatic sync settings, remote connection, keys and local sync history are removed from the active directory. The import wizard then opens. Resources in Cursor, Codex and Claude Code remain untouched.",
      "resetBackup": "The complete private recovery copy, including keys, is moved to a sibling .agenthub-reset-* directory. Delete it manually when no longer needed.",
      "resetType": "Type AGENTHUB to confirm"
    }
  }
};
const bootstrapMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": {
    "init": {
      "welcome": "欢迎使用 AgentHub",
      "chooseHint": "建立一个新库，或从已有仓库恢复能力和版本记录。",
      "newLibrary": "新建 AgentHub 库",
      "newHint": "从本机工具选择要导入的能力，也可以从空库开始。",
      "existingLibrary": "我已经有 AgentHub 库",
      "existingHint": "连接 GitHub 或 Gitea，在新设备上恢复已有的能力和历史。",
      "restoreTitle": "恢复已有 AgentHub 库",
      "restoreHint": "填写仓库地址并授权访问，直接下载已有内容和版本记录。",
      "restoreRequired": "请填写仓库地址；使用访问令牌时，还需要用户名和令牌。",
      "branchAuto": "留空自动选择",
      "branchHint": "留空时优先使用 agenthub 分支，否则使用仓库默认分支；也可以填写指定分支。",
      "authentication": "认证方式",
      "systemAuth": "系统凭据 / SSH",
      "restoreScope": "仅下载 AgentHub 库和版本，不上传、不修改本机工具。需要已有 AgentHub 格式的仓库；普通 Skills 仓库请先在原设备迁移。",
      "restoring": "正在认证、下载并校验能力和版本记录，请稍候…",
      "restoreAction": "认证并恢复能力库",
      "back": "返回选择",
      "emptyLibrary": "从空库开始",
      "restored": "项能力已从仓库恢复"
    }
  },
  "en": {
    "init": {
      "welcome": "Welcome to AgentHub",
      "chooseHint": "Create a new library or restore capabilities and versions from an existing repository.",
      "newLibrary": "Create an AgentHub library",
      "newHint": "Choose capabilities from local tools or start with an empty library.",
      "existingLibrary": "I already have an AgentHub library",
      "existingHint": "Connect GitHub or Gitea to restore your capabilities and history on this device.",
      "restoreTitle": "Restore an existing AgentHub library",
      "restoreHint": "Enter the repository address and authorize access to download its content and versions.",
      "restoreRequired": "Enter a repository address. Token authentication also requires a username and access token.",
      "branchAuto": "Leave empty to select automatically",
      "branchHint": "Prefer the agenthub branch when present, otherwise use the repository default branch. Enter a branch to override.",
      "authentication": "Authentication method",
      "systemAuth": "System credentials / SSH",
      "restoreScope": "Downloads only the library and versions. No upload or tool changes. An AgentHub-format repository is required; migrate a plain Skills repository on the original device first.",
      "restoring": "Authorizing, downloading and validating capabilities and versions…",
      "restoreAction": "Authorize and restore library",
      "back": "Back to choices",
      "emptyLibrary": "Start with an empty library",
      "restored": "capabilities restored from repository"
    }
  }
};
type Locale = keyof typeof messages;
let locale: Locale = "zh-CN";
export function setLocale(next: Locale) { locale = next; }
export function t(path: string): string {
  let value: unknown = bootstrapMessages[locale];
  for (const part of path.split(".")) value = (value as Record<string, unknown> | undefined)?.[part];
  if (value !== undefined) return String(value);
  value = workflowMessages[locale];
  for (const part of path.split(".")) value = (value as Record<string, unknown> | undefined)?.[part];
  if (value !== undefined) return String(value);
  value = extraMessages[locale];
  for (const part of path.split(".")) value = (value as Record<string, unknown> | undefined)?.[part];
  if (value !== undefined) return String(value);
  value = messages[locale];
  for (const part of path.split(".")) value = (value as Record<string, unknown>)[part];
  return String(value ?? path);
}
