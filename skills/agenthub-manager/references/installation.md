# Installation and executable discovery

## Windows

Prefer the complete `AgentHub_<version>_x64-setup.exe` installer. It installs the UI, `agenthub.exe`, and `skills/agenthub-manager/` together. NSIS registers the installation directory in the current user's PATH and its CLI location in `HKCU\Software\AgentHub\CLI`. MSI also supplies both executables and registers a user PATH entry; its discovery key is `HKCU\Software\AgentHub\MSI`. Uninstall removes the installer-owned registration without deleting the user's library.

Existing Cursor/Codex/Claude processes inherit their old environment; restarting a terminal is not always enough to refresh the parent app. Use the supplied wrapper instead of asking the user to install the CLI again:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "<skill-directory>\scripts\agenthub.ps1" --resolve
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "<skill-directory>\scripts\agenthub.ps1" doctor --json
```

`--resolve` returns only the absolute executable path. Invoke that executable with PowerShell's `&` operator and an argument array, or forward further commands through the wrapper. It checks an explicit `AGENTHUB_CLI` override first, then a neighboring portable executable, PATH, installer registry entries and fixed installation locations. Invalid explicit overrides fail rather than silently selecting another installation. Do not scan arbitrary disks or invoke a shell command assembled from user strings.

The portable ZIP contains `AgentHub.exe`, `agenthub.exe`, a neighboring `agenthub.ps1` and the skill. Extract the whole directory. Use the executable's absolute path or set `AGENTHUB_CLI` in the agent process; portable extraction does not modify PATH. The data root remains `%USERPROFILE%\.agenthub`, independent of the program folder. Independent CLI downloads are optional for users who do not want the UI.

## Ubuntu

The `.deb` installs both the UI and `/usr/bin/agenthub`. The skill is supplied at `/usr/share/agenthub/skills/agenthub-manager`. For a locally built CLI, use its absolute executable path. The normal root is `~/.agenthub`.

## Make the skill discoverable

Installing AgentHub supplies this folder but does not silently modify another tool's Skills. Copy the complete `agenthub-manager/` folder into the chosen tool's user-level Skills directory after the user requests it. Preserve an existing customized copy. A skill copied into a tool does not become a second application installation.

Confirm `agenthub --version`, inspect `--help` for that installed version, then `doctor --json`. `AGENTHUB_HOME` redirects library storage only; it does not change the user-global host scan roots. Git authentication uses the existing system credential helper or SSH agent; author name/email are not login credentials. Never embed tokens in repository URLs. Desktop Git children are hidden and noninteractive; authentication failures require credential setup outside AgentHub.
