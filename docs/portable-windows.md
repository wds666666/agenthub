# AgentHub Windows 便携版

完整解压 `AgentHub/` 目录后，双击 `agenthub-desktop.exe` 打开界面。目录内的 `agenthub.exe` 是同版本 CLI，无需再次安装。使用前准备系统 Git 和 WebView2。

PowerShell 中可以直接使用：

```powershell
./agenthub.exe --version
./agenthub.exe doctor --json
./agenthub.exe inventory --json
```

便携版不会自动修改 PATH。需要让外部 Agent 调用时，使用 `agenthub.exe` 的绝对路径，或为调用进程设置 `AGENTHUB_CLI` 指向该文件。能力库默认仍是用户目录下的 `.agenthub`；程序目录不是能力库。

[管理 Skill](../skills/agenthub-manager/SKILL.md) 单独提供。按需获取完整 `agenthub-manager` 文件夹并手动复制到工具目录；便携包不附带或自动生成 Skill／规则。已有同名目录时先检查内容，保留自己的修改。

升级时退出界面，将新版完整解压到新的目录；能力库与旧程序目录独立。Windows 安装版已经包含以上能力，并负责命令路径的注册和移除。
