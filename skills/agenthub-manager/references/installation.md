# Installation and executable discovery

## Windows

Prefer the complete `AgentHub_<version>_x64-setup.exe` installer. It installs the UI, `agenthub.exe`, and `skills/agenthub-manager/` together. NSIS registers the installation directory in the current user's PATH and its CLI location in `HKCU\Software\AgentHub\CLI`. MSI also supplies both executables and registers a user PATH entry; its discovery key is `HKCU\Software\AgentHub\MSI`. Uninstall removes the installer-owned registration without deleting the user's library.

Existing Cursor/Codex/Claude processes inherit their old environment; restarting a terminal is not always enough to refresh the parent app. Use the supplied wrapper instead of asking the user to install the CLI again:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "<skill-directory>\scripts\agenthub.ps1" --resolve
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "<skill-directory>\scripts\agenthub.ps1" doctor --json
```

`--resolve` returns only the absolute executable path. Invoke that executable with PowerShell's `&` operator and an argument array, or forward further commands through the wrapper. It checks an explicit `AGENTHUB_CLI` override first, then a neighboring portable executable, installer registry entries, PATH and fixed installation locations. The installed companion CLI takes precedence over an older standalone command on PATH. Invalid explicit overrides fail rather than silently selecting another installation. Do not scan arbitrary disks or invoke a shell command assembled from user strings.

The portable ZIP contains `agenthub-desktop.exe`, `agenthub.exe`, a neighboring `agenthub.ps1` and the skill. Extract the whole directory. Use the executable's absolute path or set `AGENTHUB_CLI` in the agent process; portable extraction does not modify PATH. The data root remains `%USERPROFILE%\.agenthub`, independent of the program folder. Independent CLI downloads are optional for users who do not want the UI.

## Ubuntu

The `.deb` installs both the UI and `/usr/bin/agenthub`. The skill is supplied at `/usr/share/agenthub/skills/agenthub-manager`. For a locally built CLI, use its absolute executable path. The normal root is `~/.agenthub`.

## Make the skill discoverable

Installing AgentHub supplies this folder but does not silently modify another tool's Skills. Copy the complete `agenthub-manager/` folder into the chosen tool's user-level Skills directory after the user requests it. Preserve an existing customized copy. A skill copied into a tool does not become a second application installation.

Confirm `agenthub --version`, inspect `--help` for that installed version, then `doctor --json`. `AGENTHUB_HOME` redirects library storage only; it does not change the user-global host scan roots. HTTPS authentication can be configured in the desktop repository sign-in dialog for GitHub and self-hosted Git/Gitea. Encrypted credentials are shared with the installed CLI; existing system helpers and SSH remain supported. Author name/email are not login credentials. Never embed tokens in repository URLs, logs or Agent prompts. Git children are hidden and noninteractive; re-enter expired tokens in the desktop and explicitly retry syncing.

## Upgrade and restore

Reinstall upgrades package-owned application files, command registration and management-skill resources. It preserves the library, local history, credentials, backups and unknown user files. It does not perform an AgentHub reset. Windows NSIS retires obsolete resource files only when their previous package hash matches; edited or unknown files are preserved. Ubuntu deb and MSI use package-manager file ownership.

On a new device, the welcome page supports restoring an existing AgentHub repository with its history directly. `agenthub bootstrap <URL> [--branch <branch>] [--username <name>]` offers the same read-only operation. With a username, supply the token through user-controlled stdin. Without a username it uses system Git credentials or SSH. Branch omission prefers `agenthub`, otherwise remote HEAD. This does not scan or write any host, create a local initial commit, or push to the repository.
