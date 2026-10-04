import { Eye, ShieldCheck } from "lucide-react";
import { useState } from "react";
import { api } from "../lib/api";
import { t } from "../lib/i18n";
import { Button } from "./ui";

/** Shared by Versions sign-in and first-run restoration. No browser persistence. */
export function RepositoryCredentials({ url, busy, platform, onPlatform, username, onUsername, token, onToken, onError, errorId }: {
  url: string; busy: boolean; platform: "github" | "git"; onPlatform: (platform: "github" | "git") => void;
  username: string; onUsername: (value: string) => void; token: string; onToken: (value: string) => void;
  onError: (value: string) => void; errorId?: string;
}) {
  const [visible, setVisible] = useState(false);
  return <>
    <fieldset className="git-login-platform" disabled={busy}><legend>{t("git.platform")}</legend><div><Button variant={platform === "github" ? "primary" : "secondary"} aria-pressed={platform === "github"} onClick={() => onPlatform("github")}>GitHub</Button><Button variant={platform === "git" ? "primary" : "secondary"} aria-pressed={platform === "git"} onClick={() => onPlatform("git")}>{t("git.selfHosted")}</Button></div></fieldset>
    <label className="field"><span>{t("git.username")}</span><input data-autofocus autoComplete="username" disabled={busy} value={username} aria-invalid={Boolean(errorId)} aria-describedby={errorId} onChange={(event) => onUsername(event.target.value)} /></label>
    <label className="field"><span>{t("git.token")}</span><span className="git-token-input"><input type={visible ? "text" : "password"} autoComplete="off" spellCheck={false} disabled={busy} value={token} aria-invalid={Boolean(errorId)} aria-describedby={errorId} onChange={(event) => onToken(event.target.value)} /><Button variant="secondary" disabled={busy} aria-label={t(visible ? "git.hideToken" : "git.showToken")} aria-pressed={visible} onClick={() => setVisible((value) => !value)}><Eye size={16} /></Button></span></label>
    <p>{t(platform === "github" ? "git.tokenHintGithub" : "git.tokenHintGit")}</p>
    <Button variant="secondary" disabled={busy || !url.trim()} onClick={() => { void api.openTokenSettings(url.trim(), platform).catch((value) => onError(String(value))); }}>{t("git.tokenSettings")}</Button>
    <p className="git-login-privacy"><ShieldCheck size={17} />{t("git.credentialHint")}</p>
  </>;
}
