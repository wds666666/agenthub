import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { RefreshCw, Radio } from "lucide-react";
import { backgroundApi, type BackgroundSettings, type BackgroundStatus } from "../lib/api";
import { Button, Dialog, StatusBadge } from "./ui";
import { t } from "../lib/i18n";

export function DesktopControls() {
  const [open, setOpen] = useState(false);
  const [remember, setRemember] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [tray, setTray] = useState(false);
  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) return;
    let disposed = false; let unlisten: (() => void) | undefined;
    void listen("close-requested", () => {
      setError(""); setOpen(true);
      void backgroundApi.status().then((s) => setTray(s.tray_available)).catch((e) => setError(String(e)));
    }).then((fn) => { if (disposed) fn(); else unlisten = fn; }).catch(() => undefined);
    return () => { disposed = true; unlisten?.(); };
  }, []);
  const choose = async (behavior: "exit" | "tray") => {
    setBusy(true); setError("");
    try { await backgroundApi.close(behavior, remember); setOpen(false); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };
  return <Dialog open={open} onClose={() => !busy && setOpen(false)} title={t("background.closeTitle")} actions={<>
    <Button variant="secondary" disabled={busy} onClick={() => setOpen(false)}>{t("background.cancel")}</Button>
    <Button variant="secondary" disabled={busy} onClick={() => void choose("exit")}>{t("background.exit")}</Button>
    <Button disabled={busy || !tray} onClick={() => void choose("tray")}>{t("background.tray")}</Button>
  </>}><p>{t("background.closeHint")}</p>{!tray && <p role="status">{t("background.unavailable")}</p>}
    <label className="background-check"><input type="checkbox" checked={remember} disabled={busy} onChange={(e) => setRemember(e.target.checked)} />{t("background.remember")}</label>
    {error && <p className="inline-error" role="alert">{error}</p>}
  </Dialog>;
}
export function BackgroundPanel() {
  const dirty = useRef(false);
  const [settings, setSettings] = useState<BackgroundSettings | null>(null);
  const [status, setStatus] = useState<BackgroundStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  useEffect(() => {
    let disposed = false;
    void backgroundApi.settings().then((s) => { if (!disposed) setSettings(s); }).catch((e) => { if (!disposed) setError(String(e)); });
    const update = () => {
      if (!document.hidden) {
        void backgroundApi.status().then((s) => { if (!disposed) setStatus(s); }).catch(() => undefined);
        if (!dirty.current) void backgroundApi.settings().then((s) => { if (!disposed && !dirty.current) setSettings(s); }).catch(() => undefined);
      }
    };
    update(); const timer = window.setInterval(update, 2000);
    return () => { disposed = true; window.clearInterval(timer); };
  }, []);
  const save = async () => {
    if (!settings) return;
    setBusy(true); setError(""); setNotice("");
    try { setSettings(await backgroundApi.save(settings)); dirty.current=false; setNotice(t("background.saved")); }
    catch (e) { setError(String(e)); } finally { setBusy(false); }
  };
  const refresh = async () => {
    setBusy(true); setError(""); setNotice("");
    try { await backgroundApi.refresh(); setNotice(t("background.accepted")); }
    catch (e) { setError(String(e)); } finally { setBusy(false); }
  };
  const date = (value?: string) => value ? new Date(value).toLocaleString() : "—";
  return <section className="material policy-panel background-panel">
    <header><div><h2><Radio size={20} /> {t("background.title")}</h2><p>{t("background.hint")}</p></div>
      <StatusBadge tone={status?.error ? "warning" : "ok"}>{t(`background.${status?.running ? "running" : status?.paused ? "paused" : status?.pending ? "pending" : "idle"}`)}</StatusBadge>
    </header>
    {settings && <div className="form-grid">
      <label className="field"><span>{t("background.close")}</span><select value={settings.close_behavior} disabled={busy} onChange={(e) => { dirty.current=true; setSettings({ ...settings, close_behavior: e.target.value as BackgroundSettings["close_behavior"] }); }}>
        {["ask", "exit", "tray"].map((value) => <option key={value} value={value} disabled={value === "tray" && status?.tray_available === false}>{t(`background.${value}`)}</option>)}
      </select></label>
      <label className="field"><span>{t("background.checks")}</span><select value={settings.check_seconds} disabled={busy} onChange={(e) => { dirty.current=true; setSettings({ ...settings, check_seconds: Number(e.target.value) }); }}>
        {[60, 120, 300, 600, 1800].map((seconds) => <option key={seconds} value={seconds}>{seconds / 60} {t("background.minutes")}</option>)}
      </select></label>
      <label className="field"><span>{t("background.delivery")}</span><select value={settings.sync_seconds} disabled={busy} onChange={(e) => { dirty.current=true; setSettings({ ...settings, sync_seconds: Number(e.target.value) }); }}>
        <option value={0}>{t("background.immediate")}</option>{[60, 120, 300, 600, 1800].map((seconds) => <option key={seconds} value={seconds}>{seconds / 60} {t("background.minutes")}</option>)}
      </select></label>
      <label className="background-check"><input type="checkbox" checked={settings.paused} disabled={busy} onChange={(e) => { dirty.current=true; setSettings({ ...settings, paused: e.target.checked }); }} />{t("background.paused")}</label>
    </div>}
    {status?.tray_available === false && <p>{t("background.unavailable")}</p>}
    <p>{t("background.last")}: {date(status?.last_success)} · {t("background.next")}: {date(status?.next_sync)}</p>
    <p>{t("background.refreshHint")}</p>
    <div className="remote-actions"><Button variant="secondary" disabled={busy || !settings} onClick={() => void save()}>{t("background.save")}</Button>
      <Button disabled={busy || status?.running} onClick={() => void refresh()}><RefreshCw size={16} className={status?.running ? "spin" : ""} />{t("background.refresh")}</Button></div>
    {notice && <p role="status">{notice}</p>}
    {(error || status?.error || status?.manager_error) && <p className="inline-error" role="alert">{[error, status?.error, status?.manager_error].filter(Boolean).join(" · ")}</p>}
  </section>;
}
