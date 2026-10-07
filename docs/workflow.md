# 验证、构建与发布

默认版本号读取 `package.json`，与 Cargo 和 Tauri 保持一致。**用户未指定新版本号时，不改版本号、不自动递增。** 当前程序版本为 `0.1.5`。

## 发布顺序

1. 在本地完成相应测试与 Ubuntu 编译，由用户测试确认。
2. 推送代码；普通推送和 PR 只检查文档、版本元数据和工作流语法，不编译程序或安装包。
3. 需要云端产物时，为已测试的提交创建版本标签。标签推送自动构建 Windows 与 Ubuntu 安装包，并执行对应测试。
4. 双平台构建成功后，只有用户明确同意本次发布，才创建发行版，直接使用已有产物，不重复构建。

新版本标签使用 `v<版本号>`。程序版本保持不变但需要新补丁标签时，使用 `v<版本号>-patch.N`，例如 `v0.1.4-patch.1`；`N` 从 1 起递增。不覆盖或移动既有标签。替换历史发行版仍须单独获得用户明确授权。

## 三个工作流

- **检查**（`checks.yml`）：普通推送与 PR 执行轻量检查，不生成可执行文件或安装包。
- **构建安装包**（`build.yml`）：由版本标签推送触发。支持在已有标签上手动重建全部或单个平台；普通分支默认拒绝构建；用户明确要求先构建、成功后再打标签时，可手动开启 `approved_pre_tag=true`。产物保留 14 天，临时下载需登录 GitHub。标签构建本身不发布发行版。
- **发布发行版**（`release.yml`）：手动指定成功构建的运行 ID、标签，并显式确认本次发布授权。检查提交、双平台产物、版本与校验值后发布；默认拒绝覆盖既有发行版。

```bash
# 本地测试通过并得到用户确认后，为当前版本创建新补丁标签
# 数字按尚未使用的下一个补丁编号选择，不改程序版本
git tag -a v0.1.4-patch.1 -m "AgentHub 0.1.4 patch 1"
git push origin v0.1.4-patch.1

# 仅在已测试标签上重建 Windows 包，不创建发行版
gh workflow run build.yml --repo wds666666/agenthub \
  --ref v0.1.4-patch.1 -f platform=windows

# 双平台构建成功且用户明确授权本次发布后，复用该构建的产物
gh workflow run release.yml --repo wds666666/agenthub \
  --ref v0.1.4-patch.1 -f build_run_id=<成功运行ID> \
  -f tag=v0.1.4-patch.1 -f approved=true
```

发行说明使用所选提交的 `docs/releases/<标签>.md`，简要列出已实现与未完成内容。编译、推送、打标签或历史发布的授权不能代替本次发行版发布授权。仅在明确要求替换时设置 `replace_existing=true`。

## 本地验证

程序修改执行格式、静态检查和相关前端、Rust 测试。导入与登录交互还须运行 `pnpm test:e2e`，覆盖 Chromium、WebKit、键盘与窄窗口。WSL 的 WPE WebKit 合成线程偶发崩溃时可用 `pnpm exec playwright test --workers 1` 串行复测，并检查系统日志；不要把浏览器进程退出误认为业务断言通过。

HTTPS 认证改动还须执行 `cargo build -p agenthub-cli` 和 `python3 scripts/test_git_auth.py`。它使用临时 TLS 服务与真实 Git 验证登录、双设备合并、上传权限失败、本地提交保留和凭据清除，不访问开发者的真实能力库。

用户明确要求先构建再创建标签时：推送已在本地验证的提交，手动触发 `build.yml`，设置 `platform=all`、`approved_pre_tag=true`。双平台成功后，在同一提交上运行 `release.yml`，显式传入尚未使用的补丁标签与发布授权；流程复用产物并创建标签和发行版。此授权入口不由普通推送自动触发，也不改变默认版本号。
