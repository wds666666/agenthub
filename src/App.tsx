import { useCallback, useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import {
  Activity,
  ArrowRight,
  Boxes,
  Braces,
  CheckCircle2,
  ChevronRight,
  CircleDot,
  Code2,
  Database,
  FileCode2,
  File,
  FileText,
  FolderTree,
  GitBranch,
  HardDrive,
  History,
  Eye,
  KeyRound,
  LayoutDashboard,
  Package,
  Pencil,
  PlugZap,
  Plus,
  Radio,
  RefreshCw,
  RotateCcw,
  Save,
  Settings as SettingsIcon,
  ShieldCheck,
  Shuffle,
  Sparkles,
  TerminalSquare,
  Upload,
  UserRound,
} from "lucide-react";
import { api, type Capability, type CapabilityDetail, type Dashboard, type GitIdentity, type Kind, type Plan, type RuleDocument, type ScanItem, type Target } from "./lib/api";
import { t } from "./lib/i18n";
import agentHubLogo from "./assets/agenthub-logo.png";
import { Button, Dialog, PageHeader, SearchField, StatusBadge, Toast } from "./components/ui";
import { SyncRail } from "./components/SyncRail";
import "./styles/tokens.css";
import "./styles/app.css";

type Page = "overview" | "inventory" | "sync" | "git" | "transactions" | "settings";
type Icon = typeof LayoutDashboard;

const nav: Array<{ id: Page; icon: Icon }> = [
  { id: "overview", icon: LayoutDashboard },
  { id: "inventory", icon: Boxes },
  { id: "sync", icon: Shuffle },
  { id: "git", icon: GitBranch },
  { id: "transactions", icon: History },
  { id: "settings", icon: SettingsIcon },
];

const kindMeta: Array<{ id: Kind; icon: Icon; accent: string }> = [
  { id: "skill", icon: Sparkles, accent: "blue" },
  { id: "mcp", icon: PlugZap, accent: "green" },
  { id: "plugin", icon: Package, accent: "purple" },
  { id: "rule", icon: FileText, accent: "amber" },
];

const targetMeta: Array<{ id: Target; mark: string; description: string }> = [
  { id: "cursor", mark: "CU", description: "Cursor" },
  { id: "codex", mark: "CX", description: "OpenAI Codex" },
  { id: "claude", mark: "CL", description: "Claude Code" },
];

function transactionStatus(status: string) {
  const keys: Record<string, string> = {
    applied: "statusApplied",
    rollback_applied: "statusRollbackApplied",
    rolled_back: "statusRolledBack",
    rollback_failed: "statusRollbackFailed",
    rollback_recovery_failed: "statusRollbackFailed",
    applying: "statusApplying",
    rolling_back: "statusRollingBack",
  };
  return keys[status] ? t(`transactions.${keys[status]}`) : status;
}

export default function App() {
  const [page, setPage] = useState<Page>("overview");
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [error, setError] = useState("");
  const [toast, setToast] = useState("");

  const refresh = useCallback(() => {
    setError("");
    api.dashboard().then(setDashboard).catch((value) => setError(String(value)));
  }, []);

  useEffect(refresh, [refresh]);
  useEffect(() => { document.title = `${t(`nav.${page}`)} · AgentHub`; }, [page]);
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(""), 3200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  if (error) {
    return (
      <main className="center-state">
        <span className="center-state__icon center-state__icon--danger"><Activity /></span>
        <h1>{t("toast.failed")}</h1>
        <p>{error}</p>
        <Button onClick={refresh}><RefreshCw size={17} /> {t("common.retry")}</Button>
      </main>
    );
  }

  if (!dashboard) {
    return (
      <main className="center-state" aria-busy="true">
        <span className="spinner" />
        <p>{t("common.loading")}</p>
      </main>
    );
  }

  if (!dashboard.initialized) return <Init onDone={refresh} />;

  const content = {
    overview: <Overview data={dashboard} />,
    inventory: <Inventory onChanged={refresh} onNotify={setToast} />,
    sync: <Sync onApplied={() => { setToast(t("toast.applied")); refresh(); }} />,
    git: <GitPage onCommitted={() => { setToast(t("toast.committed")); refresh(); }} />,
    transactions: <Transactions data={dashboard} onChanged={refresh} onNotify={setToast} />,
    settings: <SettingsPage data={dashboard} />,
  }[page];

  return (
    <div className="app-shell">
      <a className="skip-link" href="#main">{t("skip")}</a>
      <aside className="sidebar">
        <div className="brand">
          <span className="brand__mark"><img src={agentHubLogo} alt="" /></span>
          <span className="brand__copy"><strong>{t("appName")}</strong><small>LOCAL CONTROL</small></span>
          <span className="brand__version">0.1</span>
        </div>
        <nav aria-label="Primary">
          {nav.map(({ id, icon: NavIcon }) => (
            <button
              type="button"
              className={page === id ? "active" : ""}
              aria-current={page === id ? "page" : undefined}
              onClick={() => setPage(id)}
              key={id}
            >
              <NavIcon size={19} />
              <span>{t(`nav.${id}`)}</span>
              {page === id && <ChevronRight className="nav-chevron" size={15} aria-hidden="true" />}
            </button>
          ))}
        </nav>
        <div className="source-mark">
          <span className="source-mark__icon"><TerminalSquare size={18} /></span>
          <span><small>CANONICAL SOURCE</small><code>~/.agenthub</code></span>
          <span className="source-mark__pulse" aria-label={t("settings.healthy")} />
        </div>
      </aside>
      <main id="main" className="main-stage">
        <div className="page-stage" key={page}>{content}</div>
      </main>
      {toast && <Toast message={toast} />}
    </div>
  );
}

function Overview({ data }: { data: Dashboard }) {
  const total = kindMeta.reduce((sum, item) => sum + (data.inventory[item.id] ?? 0), 0);
  return (
    <>
      <PageHeader
        eyebrow={t("overview.eyebrow")}
        title={t("overview.title")}
        subtitle={t("overview.subtitle")}
        actions={<StatusBadge tone={data.dirty ? "warning" : "ok"}>{data.dirty ? t("overview.pending") : t("overview.clean")}</StatusBadge>}
      />

      <section className="metric-grid" aria-label={t("overview.allCapabilities")}>
        {kindMeta.map(({ id, icon: KindIcon, accent }) => (
          <article className={`metric-card metric-card--${accent}`} key={id}>
            <span className="metric-card__icon"><KindIcon size={20} /></span>
            <div><strong>{data.inventory[id] ?? 0}</strong><span>{t(`kinds.${id}`)}</span></div>
          </article>
        ))}
        <article className="metric-card metric-card--total">
          <span className="metric-card__icon"><CircleDot size={20} /></span>
          <div><strong>{total}</strong><span>{t("overview.totalResources")}</span></div>
        </article>
      </section>

      <SyncRail dashboard={data} />

      <section className="material activity-card">
        <header className="section-heading">
          <div><p className="eyebrow">ACTIVITY</p><h2>{t("overview.recent")}</h2></div>
          <span className="count-badge">{data.recent_transactions.length}</span>
        </header>
        {data.recent_transactions.length ? (
          <div className="transaction-list">
            {data.recent_transactions.map((tx) => (
              <article className="transaction-row" key={tx.id}>
                <span className="transaction-row__icon"><CheckCircle2 size={18} /></span>
                <div><strong>{t(`targets.${tx.target}`)}</strong><code>{tx.id.slice(0, 12)}</code></div>
                <time>{new Date(tx.created_at).toLocaleString()}</time>
                <StatusBadge tone={tx.status === "applied" || tx.status === "rollback_applied" ? "ok" : "warning"}>{transactionStatus(tx.status)}</StatusBadge>
              </article>
            ))}
          </div>
        ) : (
          <EmptyState icon={History} title={t("transactions.emptyTitle")} body={t("overview.emptyTx")} />
        )}
      </section>
    </>
  );
}

function Inventory({ onChanged, onNotify }: { onChanged: () => void; onNotify: (message: string) => void }) {
  const [items, setItems] = useState<Capability[]>([]);
  const [query, setQuery] = useState("");
  const [editor, setEditor] = useState<{ document: RuleDocument; create: boolean; imported: boolean } | null>(null);
  const [loadError, setLoadError] = useState("");
  const [loadingRule, setLoadingRule] = useState(false);
  const [detail, setDetail] = useState<CapabilityDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);
  const loadInventory = useCallback(() => { api.inventory().then(setItems).catch(() => setItems([])); }, []);
  useEffect(loadInventory, [loadInventory]);
  const filtered = useMemo(
    () => items.filter((item) => `${item.id} ${item.display_name} ${item.path}`.toLowerCase().includes(query.toLowerCase())),
    [items, query],
  );

  const blankRule = (): RuleDocument => ({
    schemaVersion: 1,
    id: "",
    displayName: "",
    activation: "always",
    paths: [],
    targets: ["cursor", "codex", "claude"],
    body: "",
  });
  const slug = (value: string) => value.toLowerCase().replace(/\.[^.]+$/, "").replace(/[^a-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 80);
  const importMarkdown = async (file?: File) => {
    if (!file) return;
    void api.debugEvent("rule_import_selected", `name=${file.name} size=${file.size}`);
    if (file.size > 1_000_000) { setLoadError(t("inventory.fileTooLarge")); return; }
    const body = await file.text();
    const name = file.name.replace(/\.(md|mdc)$/i, "");
    setEditor({ document: { ...blankRule(), id: slug(name), displayName: name, body }, create: true, imported: true });
    if (fileRef.current) fileRef.current.value = "";
  };
  const editRule = async (id: string) => {
    setLoadingRule(true); setLoadError("");
    try { setEditor({ document: await api.readRule(id), create: false, imported: false }); }
    catch { setLoadError(t("inventory.loadFailed")); }
    finally { setLoadingRule(false); }
  };
  const openDetail = async (item: Capability) => {
    setLoadingDetail(`${item.kind}:${item.id}`); setLoadError("");
    void api.debugEvent("capability_detail_open", `kind=${item.kind} id=${item.id}`);
    try { setDetail(await api.capabilityDetail(item.kind, item.id)); }
    catch { setLoadError(t("inventory.detailLoadFailed")); }
    finally { setLoadingDetail(""); }
  };
  const saved = () => {
    setEditor(null);
    loadInventory();
    onChanged();
    onNotify(t("toast.ruleSaved"));
  };

  return (
    <>
      <PageHeader
        title={t("inventory.title")}
        subtitle={t("inventory.subtitle")}
        actions={
          <div className="inventory-actions">
            <SearchField value={query} onChange={setQuery} />
            <input ref={fileRef} className="sr-only" type="file" accept=".md,.mdc,text/markdown,text/plain" onChange={(event) => void importMarkdown(event.target.files?.[0])} />
            <Button variant="secondary" onClick={() => fileRef.current?.click()}><Upload size={16} />{t("inventory.importRule")}</Button>
            <Button onClick={() => setEditor({ document: blankRule(), create: true, imported: false })}><Plus size={16} />{t("inventory.newRule")}</Button>
          </div>
        }
      />
      {loadError && <div className="inline-error" role="alert"><Activity size={18} /><span>{loadError}</span></div>}
      <div className="inventory-grid">
        {kindMeta.map(({ id, icon: KindIcon, accent }) => {
          const group = filtered.filter((item) => item.kind === id);
          return (
            <section className={`material inventory-card inventory-card--${accent}`} key={id}>
              <header className="inventory-card__header">
                <span className="inventory-card__icon"><KindIcon size={20} /></span>
                <div><h2>{t(`kinds.${id}`)}</h2><p>{group.length}</p></div>
                <span className="count-badge">{group.length}</span>
              </header>
              <div className="capability-list">
                {group.map((item) => (
                  <article className="capability-row" key={item.id}>
                    <button className="capability-open" type="button" disabled={Boolean(loadingDetail)} onClick={() => void openDetail(item)} aria-label={`${t("inventory.viewDetails")} ${item.display_name}`}>
                      <span className="capability-row__glyph">{loadingDetail === `${item.kind}:${item.id}` ? <RefreshCw className="spin" size={16} /> : <KindIcon size={17} />}</span>
                      <span className="capability-row__copy">
                        <strong>{item.display_name}</strong>
                        <small>{item.path}</small>
                      </span>
                      <code title={t("inventory.digest")}>{item.digest.slice(0, 8)}</code>
                      <ChevronRight size={15} />
                    </button>
                    {id === "rule" && (
                      <button className="capability-edit" type="button" disabled={loadingRule} onClick={() => void editRule(item.id)} aria-label={`${t("common.edit")} ${item.display_name}`}>
                        <Pencil size={15} />
                      </button>
                    )}
                  </article>
                ))}
                {!group.length && <EmptyState icon={KindIcon} title={t("inventory.empty")} body={t("inventory.emptyHint")} compact />}
              </div>
            </section>
          );
        })}
      </div>
      {editor && <RuleEditor key={`${editor.create ? "new" : "edit"}-${editor.document.id}`} initial={editor.document} create={editor.create} imported={editor.imported} onClose={() => setEditor(null)} onSaved={saved} />}
      {detail && <CapabilityDetailDialog detail={detail} onClose={() => setDetail(null)} onEditRule={detail.capability.kind === "rule" ? () => { const id = detail.capability.id; setDetail(null); void editRule(id); } : undefined} />}
    </>
  );
}

function CapabilityDetailDialog({ detail, onClose, onEditRule }: { detail: CapabilityDetail; onClose: () => void; onEditRule?: () => void }) {
  const meta = kindMeta.find((item) => item.id === detail.capability.kind) ?? kindMeta[0];
  const DetailIcon = meta.icon;
  const bytes = (size: number) => size < 1024 ? `${size} B` : `${(size / 1024).toFixed(size < 10_240 ? 1 : 0)} KB`;
  return (
    <Dialog
      open
      wide
      onClose={onClose}
      title={detail.capability.display_name}
      actions={<>{onEditRule && <Button variant="secondary" onClick={onEditRule}><Pencil size={15} />{t("inventory.editRule")}</Button>}<Button onClick={onClose}>{t("common.close")}</Button></>}
    >
      <div className="capability-detail__identity">
        <span className={`inventory-card__icon inventory-card__icon--${meta.accent}`}><DetailIcon size={21} /></span>
        <div><span>{t(`kinds.${detail.capability.kind}`)}</span><code>{detail.capability.id}</code></div>
        <StatusBadge tone="ok">{detail.files.length} {t("inventory.files")}</StatusBadge>
      </div>
      {detail.capability.kind === "plugin" && <div className="form-intro"><ShieldCheck size={17} /><span>{t("inventory.pluginPreviewHint")}</span></div>}
      <section className="capability-preview">
        <header><span><Eye size={15} />{detail.preview_path}</span>{detail.preview_truncated && <StatusBadge tone="warning">{t("inventory.previewTruncated")}</StatusBadge>}</header>
        <pre>{detail.preview}</pre>
      </section>
      <section className="capability-files">
        <header><FolderTree size={16} /><strong>{t("inventory.fileInventory")}</strong></header>
        <div>{detail.files.map((item) => <span key={item.path}><File size={14} /><code>{item.path}</code><small>{bytes(item.size)}</small></span>)}</div>
      </section>
    </Dialog>
  );
}

function RuleEditor({ initial, create, imported, onClose, onSaved }: { initial: RuleDocument; create: boolean; imported: boolean; onClose: () => void; onSaved: () => void }) {
  const [draft, setDraft] = useState(initial);
  const [pathsText, setPathsText] = useState(initial.paths.join("\n"));
  const [dirty, setDirty] = useState(imported);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [discardPrompt, setDiscardPrompt] = useState(false);

  const update = <K extends keyof RuleDocument>(key: K, value: RuleDocument[K]) => {
    setDraft((current) => ({ ...current, [key]: value }));
    setDirty(true); setError("");
    setErrors((current) => ({ ...current, [key]: "" }));
  };
  const requestClose = () => {
    if (discardPrompt) { setDiscardPrompt(false); return; }
    if (dirty) setDiscardPrompt(true); else onClose();
  };
  const submit = async (event: FormEvent) => {
    event.preventDefault();
    const paths = pathsText.split(/\r?\n/).map((value) => value.trim()).filter(Boolean);
    const nextErrors: Record<string, string> = {};
    if (!/^[a-z0-9][a-z0-9_-]{0,79}$/.test(draft.id)) nextErrors.id = t("inventory.invalidId");
    if (!draft.displayName.trim()) nextErrors.displayName = t("inventory.required");
    if (!draft.body.trim()) nextErrors.body = t("inventory.required");
    if (!draft.targets.length) nextErrors.targets = t("inventory.targetRequired");
    if (draft.activation === "paths" && !paths.length) nextErrors.paths = t("inventory.pathRequired");
    setErrors(nextErrors);
    if (Object.keys(nextErrors).length) {
      document.querySelector<HTMLElement>("[aria-invalid='true']")?.focus();
      return;
    }
    setBusy(true); setError("");
    void api.debugEvent("rule_save_click", `id=${draft.id} create=${create}`);
    try {
      await api.saveRule({ ...draft, displayName: draft.displayName.trim(), paths }, create);
      void api.debugEvent("rule_save_ok", `id=${draft.id}`);
      setDirty(false);
      onSaved();
    } catch (value) { void api.debugEvent("rule_save_failed", `id=${draft.id}`); setError(String(value)); }
    finally { setBusy(false); }
  };
  const toggleTarget = (target: Target) => {
    const targets = draft.targets.includes(target) ? draft.targets.filter((value) => value !== target) : [...draft.targets, target];
    update("targets", targets);
  };

  return (
    <Dialog
      open
      wide
      onClose={requestClose}
      title={discardPrompt ? t("inventory.discardTitle") : create ? t("inventory.newRule") : t("inventory.editRule")}
      actions={discardPrompt ? (
        <><Button variant="secondary" data-autofocus onClick={() => setDiscardPrompt(false)}>{t("inventory.keepEditing")}</Button><Button variant="danger" onClick={onClose}>{t("inventory.discard")}</Button></>
      ) : (
        <><Button variant="secondary" onClick={requestClose}>{t("common.cancel")}</Button><Button form="rule-form" type="submit" disabled={busy}>{busy ? <RefreshCw className="spin" size={16} /> : <Save size={16} />}{busy ? t("inventory.saving") : t("inventory.saveRule")}</Button></>
      )}
    >
      {discardPrompt ? <p>{t("inventory.discardBody")}</p> : (
        <form id="rule-form" className="rule-form" noValidate onSubmit={submit}>
          <div className="form-intro"><FileText size={18} /><span>{t("inventory.ruleFormHint")}</span>{imported && <StatusBadge tone="ok">{t("inventory.importedFile")}</StatusBadge>}</div>
          {error && <div className="inline-error" role="alert"><Activity size={17} /><span>{error}</span></div>}
          <div className="form-grid">
            <label className="field"><span>{t("inventory.id")}</span><input data-autofocus value={draft.id} readOnly={!create} placeholder={t("inventory.idPlaceholder")} aria-invalid={Boolean(errors.id)} aria-describedby={errors.id ? "rule-id-help rule-id-error" : "rule-id-help"} onChange={(event) => update("id", event.target.value)} /><small id="rule-id-help">{t("inventory.idHint")}</small>{errors.id && <em id="rule-id-error" role="alert">{errors.id}</em>}</label>
            <label className="field"><span>{t("inventory.displayName")}</span><input value={draft.displayName} aria-invalid={Boolean(errors.displayName)} aria-describedby={errors.displayName ? "rule-name-error" : undefined} onChange={(event) => update("displayName", event.target.value)} />{errors.displayName && <em id="rule-name-error" role="alert">{errors.displayName}</em>}</label>
          </div>
          <fieldset className="field-group"><legend>{t("inventory.activation")}</legend><div className="segmented-control">{(["always", "manual", "paths"] as const).map((value) => <label className={draft.activation === value ? "is-selected" : ""} key={value}><input type="radio" name="activation" checked={draft.activation === value} onChange={() => update("activation", value)} /><span>{t(`inventory.activation${value[0].toUpperCase()}${value.slice(1)}`)}</span></label>)}</div></fieldset>
          {draft.activation === "paths" && <label className="field"><span>{t("inventory.paths")}</span><textarea className="rule-paths resize-none" value={pathsText} aria-invalid={Boolean(errors.paths)} aria-describedby={errors.paths ? "rule-paths-help rule-paths-error" : "rule-paths-help"} onChange={(event) => { setPathsText(event.target.value); setDirty(true); setErrors((current) => ({ ...current, paths: "" })); }} /><small id="rule-paths-help">{t("inventory.pathsHint")}</small>{errors.paths && <em id="rule-paths-error" role="alert">{errors.paths}</em>}</label>}
          <fieldset className="field-group"><legend>{t("inventory.targets")}</legend><div className="target-checks">{targetMeta.map((item) => <label className={draft.targets.includes(item.id) ? "is-selected" : ""} key={item.id}><input type="checkbox" checked={draft.targets.includes(item.id)} onChange={() => toggleTarget(item.id)} /><span className="target-tab__mark">{item.mark}</span><strong>{t(`targets.${item.id}`)}</strong><CheckCircle2 size={17} /></label>)}</div>{errors.targets && <em role="alert">{errors.targets}</em>}</fieldset>
          <label className="field"><span>{t("inventory.body")}</span><textarea className="rule-body resize-none" value={draft.body} aria-invalid={Boolean(errors.body)} aria-describedby={errors.body ? "rule-body-help rule-body-error" : "rule-body-help"} onChange={(event) => update("body", event.target.value)} /><small id="rule-body-help">{t("inventory.bodyHint")}</small>{errors.body && <em id="rule-body-error" role="alert">{errors.body}</em>}</label>
        </form>
      )}
    </Dialog>
  );
}

function Sync({ onApplied }: { onApplied: () => void }) {
  const [target, setTarget] = useState<Target>("codex");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [error, setError] = useState("");
  const planSummary = useMemo(() => kindMeta.map((kind) => ({
    ...kind,
    summary: plan?.summary?.find((item) => item.kind === kind.id) ?? { kind: kind.id, affected: 0, create: 0, update: 0, delete: 0, skip: 0, files: 0 },
  })), [plan]);
  const affectedCapabilities = planSummary.reduce((sum, item) => sum + item.summary.affected, 0);

  const makePlan = async () => {
    setBusy(true);
    setError("");
    void api.debugEvent("sync_plan_click", `target=${target}`);
    try { const next = await api.plan(target); setPlan(next); void api.debugEvent("sync_plan_ready", `target=${target} steps=${next.steps.length}`); }
    catch (value) { void api.debugEvent("sync_plan_failed", `target=${target}`); setError(String(value)); }
    finally { setBusy(false); }
  };
  const apply = async () => {
    if (!plan) return;
    setBusy(true);
    setError("");
    void api.debugEvent("sync_apply_click", `target=${target} plan=${plan.id}`);
    try {
      await api.apply(plan.id);
      void api.debugEvent("sync_apply_ok", `target=${target} plan=${plan.id}`);
      setConfirm(false);
      setPlan(null);
      onApplied();
    } catch (value) { void api.debugEvent("sync_apply_failed", `target=${target} plan=${plan.id}`); setError(String(value)); } finally { setBusy(false); }
  };

  return (
    <>
      <PageHeader title={t("sync.title")} subtitle={t("sync.subtitle")} />
      <section className="sync-workspace">
        <div className="sync-targets">
          <div className="section-heading section-heading--compact"><div><p className="eyebrow">DESTINATION</p><h2>{t("sync.chooseTarget")}</h2></div></div>
          <div className="target-tabs" role="tablist">
            {targetMeta.map((item) => (
              <button
                type="button"
                role="tab"
                aria-selected={target === item.id}
                className={target === item.id ? "is-selected" : ""}
                onClick={() => { setTarget(item.id); setPlan(null); }}
                key={item.id}
              >
                <span className="target-tab__mark">{item.mark}</span>
                <span><strong>{t(`targets.${item.id}`)}</strong><small>{item.description}</small></span>
                <span className="target-tab__radio"><CheckCircle2 size={18} /></span>
              </button>
            ))}
          </div>
        </div>

        <div className="warning-band"><ShieldCheck size={19} /><span>{t("sync.warning")}</span></div>
        {error && <div className="inline-error" role="alert"><Activity size={18} /><span>{error}</span></div>}

        <section className={`material plan-panel ${plan ? "has-plan" : ""}`}>
          <div className="plan-panel__summary">
            <span className="plan-panel__icon">{plan ? <CheckCircle2 /> : <FileCode2 />}</span>
            <div>
              <p className="eyebrow">PLAN / {target.toUpperCase()}</p>
              <h2>{plan ? (plan.steps.length ? `${affectedCapabilities} ${t("sync.capabilityChanges")} · ${plan.steps.length} ${t("sync.steps")}` : t("sync.noChanges")) : t("sync.noPlan")}</h2>
              <p>{plan ? t("sync.ready") : t("sync.noPlanHint")}</p>
            </div>
            <Button onClick={makePlan} disabled={busy}>{busy ? <RefreshCw className="spin" size={17} /> : <Braces size={17} />} {busy ? t("sync.planning") : t("sync.plan")}</Button>
          </div>

          <div className="sync-phases" aria-label={t("sync.confirmBody")}>
            {[t("sync.review"), t("sync.backup"), t("sync.verify")].map((label, index) => (
              <div className={plan && index === 0 ? "is-current" : ""} key={label}>
                <span>{index + 1}</span><strong>{label}</strong>{index < 2 && <ArrowRight size={15} />}
              </div>
            ))}
          </div>

          {plan && (
            <>
              <div className="snapshot-line"><span>{t("sync.canonicalSnapshot")}</span><code>{plan.canonical_digest.slice(0, 20)}</code><StatusBadge tone={plan.git.dirty ? "warning" : "ok"}>{plan.git.dirty ? t("overview.pending") : t("overview.clean")}</StatusBadge></div>
              <div className="plan-capability-grid" aria-label={t("sync.capabilityChanges")}>
                {planSummary.map(({ id, icon: SummaryIcon, accent, summary }) => (
                  <article className={`plan-capability plan-capability--${accent}`} key={id}>
                    <span className="plan-capability__icon"><SummaryIcon size={19} /></span>
                    <div className="plan-capability__title"><strong>{t(`kinds.${id}`)}</strong><span>{summary.affected} {t("sync.affected")}</span></div>
                    <div className="plan-capability__counts">
                      <span className="is-create">+{summary.create} {t("sync.create")}</span>
                      <span className="is-update">~{summary.update} {t("sync.update")}</span>
                      <span className="is-delete">−{summary.delete} {t("sync.delete")}</span>
                    </div>
                    <small>{summary.files} {t("sync.files")}</small>
                  </article>
                ))}
              </div>
              {plan.steps.length > 0 && (
                <details className="plan-files">
                  <summary><span>{t("sync.fileDetails")}</span><strong>{plan.steps.length} {t("sync.files")}</strong></summary>
                  <div className="plan-list">
                    {plan.steps.map((step, index) => (
                      <article className="plan-row" key={`${step.path}-${step.action}-${index}`}>
                        <span className={`action-mark action-mark--${step.action}`}>{step.action}</span>
                        <div><strong>{step.capability_kind ? t(`kinds.${step.capability_kind}`) : step.detail}{step.capability_id && step.capability_id !== "__projection__" ? ` · ${step.capability_id}` : ""}</strong><code>{step.path}</code></div>
                      </article>
                    ))}
                  </div>
                </details>
              )}
              {plan.steps.length > 0 && <div className="apply-bar">
                <span><ShieldCheck size={18} /> {t("sync.confirmBody")}</span>
                <Button variant="danger" onClick={() => setConfirm(true)}>{t("sync.apply")} <ArrowRight size={17} /></Button>
              </div>}
            </>
          )}
        </section>
      </section>
      <Dialog
        open={confirm}
        onClose={() => setConfirm(false)}
        title={t("sync.confirmTitle")}
        actions={<><Button variant="secondary" onClick={() => setConfirm(false)}>{t("common.cancel")}</Button><Button variant="danger" disabled={busy} onClick={apply}>{t("sync.apply")}</Button></>}
      >
        <p>{t("sync.confirmBody")}</p>
        <code className="dialog-code">{plan?.id}</code>
      </Dialog>
    </>
  );
}

function GitPage({ onCommitted }: { onCommitted: () => void }) {
  const [status, setStatus] = useState("");
  const [diff, setDiff] = useState("");
  const [history, setHistory] = useState("");
  const [identity, setIdentity] = useState<GitIdentity>({});
  const [message, setMessage] = useState("");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    const [nextStatus, nextDiff, nextHistory, nextIdentity] = await Promise.all([api.gitStatus(), api.gitDiff(), api.gitLog(), api.gitIdentity()]);
    setStatus(nextStatus); setDiff(nextDiff); setHistory(nextHistory); setIdentity(nextIdentity);
    setName(nextIdentity.name ?? ""); setEmail(nextIdentity.email ?? "");
  }, []);
  useEffect(() => { void load().catch((value) => setError(String(value))); }, [load]);
  const dirty = Boolean(diff.trim()) || status.split("\n").some((line) => /^[ MARC?D!U]{1,2}\s/.test(line));
  const commit = async (event: FormEvent) => {
    event.preventDefault(); setError("");
    if (!message.trim()) { setError(t("inventory.required")); return; }
    if ((!identity.name && !name.trim()) || (!identity.email && !/^\S+@\S+\.\S+$/.test(email))) { setError(t("git.identityHint")); return; }
    setBusy(true);
    try {
      await api.gitCommit(message.trim(), identity.name ? undefined : name.trim(), identity.email ? undefined : email.trim());
      setMessage(""); await load(); onCommitted();
    } catch (value) { setError(String(value)); }
    finally { setBusy(false); }
  };
  const logEntries = history ? history.split("\n").map((line) => { const [sha, date, ...subject] = line.split("\t"); return { sha, date, subject: subject.join("\t") }; }) : [];
  return (
    <>
      <PageHeader title={t("git.title")} subtitle={t("git.subtitle")} />
      <section className="git-summary material">
        <span className="git-summary__icon"><GitBranch size={25} /></span>
        <div><p className="eyebrow">{t("git.branch")}</p><h2>main</h2><code>~/.agenthub/.git</code></div>
        <StatusBadge tone={dirty ? "warning" : "ok"}>{dirty ? t("overview.pending") : t("git.clean")}</StatusBadge>
      </section>
      <form className="material commit-card" noValidate onSubmit={commit}>
        <header><span className="commit-card__icon"><Save size={20} /></span><div><h2>{t("git.commitTitle")}</h2><p>{t("git.commitHint")}</p></div></header>
        {error && <div className="inline-error" role="alert"><Activity size={17} /><span>{error}</span></div>}
        <div className="commit-form-row">
          <label className="field"><span>{t("git.message")}</span><input value={message} placeholder={t("git.messagePlaceholder")} onChange={(event) => { setMessage(event.target.value); setError(""); }} /></label>
          <Button type="submit" disabled={busy || !dirty}>{busy ? <RefreshCw className="spin" size={16} /> : <GitBranch size={16} />}{busy ? t("git.committing") : t("git.commit")}</Button>
        </div>
        {(!identity.name || !identity.email) && (
          <fieldset className="identity-fields"><legend><UserRound size={15} />{t("git.identity")}</legend><p>{t("git.identityHint")}</p><div className="form-grid"><label className="field"><span>{t("git.name")}</span><input value={name} autoComplete="name" onChange={(event) => setName(event.target.value)} /></label><label className="field"><span>{t("git.email")}</span><input type="email" value={email} autoComplete="email" onChange={(event) => setEmail(event.target.value)} /></label></div></fieldset>
        )}
        {!dirty && <p className="commit-clean"><CheckCircle2 size={16} />{t("git.nothingToCommit")}</p>}
      </form>
      <div className="code-grid">
        <section className="material code-card">
          <header><span className="window-dots"><i /><i /><i /></span><h2>{t("git.status")}</h2><TerminalSquare size={16} /></header>
          <pre>{status || t("git.clean")}</pre>
        </section>
        <section className="material code-card code-card--wide">
          <header><span className="window-dots"><i /><i /><i /></span><h2>{t("git.diff")}</h2><Code2 size={16} /></header>
          <pre>{diff || t("git.noDiff")}</pre>
        </section>
      </div>
      <section className="material git-history-card">
        <header className="section-heading"><div><p className="eyebrow">VERSIONS</p><h2>{t("git.history")}</h2></div><span className="count-badge">{logEntries.length}</span></header>
        {logEntries.length ? <div className="git-log">{logEntries.map((entry) => <article key={entry.sha}><span className="git-log__point"><GitBranch size={15} /></span><div><strong>{entry.subject}</strong><span><code>{entry.sha}</code><time>{entry.date}</time></span></div></article>)}</div> : <EmptyState icon={GitBranch} title={t("git.noHistory")} body={t("git.commitHint")} compact />}
      </section>
    </>
  );
}

function Transactions({ data, onChanged, onNotify }: { data: Dashboard; onChanged: () => void; onNotify: (message: string) => void }) {
  const [transactions, setTransactions] = useState(data.recent_transactions);
  const [rollbackCandidate, setRollbackCandidate] = useState<(typeof data.recent_transactions)[number] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    try { setTransactions(await api.transactionHistory()); setError(""); }
    catch (value) { setError(`${t("transactions.historyLoadFailed")} ${String(value)}`); }
  }, []);
  useEffect(() => { void load(); }, [load]);
  const rollback = async () => {
    if (!rollbackCandidate) return;
    setBusy(true); setError("");
    void api.debugEvent("transaction_rollback_click", `transaction=${rollbackCandidate.id} target=${rollbackCandidate.target}`);
    try {
      await api.rollbackTransaction(rollbackCandidate.id);
      setRollbackCandidate(null);
      await load();
      onChanged();
      onNotify(t("transactions.rollbackDone"));
    } catch (value) {
      void api.debugEvent("transaction_rollback_failed", `transaction=${rollbackCandidate.id}`);
      setError(String(value));
    } finally { setBusy(false); }
  };
  return (
    <>
      <PageHeader title={t("nav.transactions")} subtitle={t("transactions.subtitle")} />
      <section className="material history-card">
        {error && <div className="inline-error" role="alert"><Activity size={18} /><span>{error}</span></div>}
        {transactions.length ? (
          <div className="history-timeline">
            {transactions.map((tx) => (
              <article key={tx.id}>
                <span className={`history-timeline__point ${tx.status === "rollback_applied" ? "is-rollback" : ""}`}>{tx.status === "rollback_applied" ? <RotateCcw size={18} /> : <CheckCircle2 size={18} />}</span>
                <div className="history-timeline__copy">
                  <span><strong>{t(`targets.${tx.target}`)}</strong><StatusBadge tone={tx.status === "applied" || tx.status === "rollback_applied" ? "ok" : "warning"}>{transactionStatus(tx.status)}</StatusBadge></span>
                  <code>{tx.id}</code>
                  <time>{new Date(tx.created_at).toLocaleString()}</time>
                  {(tx.status === "applied" || tx.status === "rollback_applied") && <small><ShieldCheck size={13} /> {tx.status === "rollback_applied" ? t("transactions.verifiedRollback") : t("transactions.verifiedApply")}</small>}
                </div>
                {(tx.status === "applied" || tx.status === "rollback_applied") && <Button variant="secondary" onClick={() => setRollbackCandidate(tx)}><RotateCcw size={15} />{tx.status === "applied" ? t("transactions.rollbackApply") : t("transactions.rollbackRollback")}</Button>}
              </article>
            ))}
          </div>
        ) : <EmptyState icon={History} title={t("transactions.emptyTitle")} body={t("transactions.emptyBody")} />}
      </section>
      <Dialog
        open={Boolean(rollbackCandidate)}
        onClose={() => { if (!busy) setRollbackCandidate(null); }}
        title={t("transactions.rollbackTitle")}
        actions={<><Button variant="secondary" disabled={busy} onClick={() => setRollbackCandidate(null)}>{t("common.cancel")}</Button><Button variant="danger" disabled={busy} onClick={rollback}>{busy ? <RefreshCw className="spin" size={16} /> : <RotateCcw size={16} />}{busy ? t("transactions.rollingBack") : t("transactions.rollbackAction")}</Button></>}
      >
        <p>{t("transactions.rollbackBody")}</p>
        <div className="rollback-target"><span className="target-tab__mark">{targetMeta.find((item) => item.id === rollbackCandidate?.target)?.mark}</span><span><strong>{rollbackCandidate ? t(`targets.${rollbackCandidate.target}`) : ""}</strong><small>{t("transactions.backupAvailable")}</small></span></div>
        <code className="dialog-code">{rollbackCandidate?.id}</code>
      </Dialog>
    </>
  );
}

function SettingsPage({ data }: { data: Dashboard }) {
  const cards = [
    { icon: HardDrive, title: t("settings.canonical"), hint: t("settings.canonicalHint"), value: "~/.agenthub", meta: "Git", tone: "blue" },
    { icon: Database, title: t("settings.database"), hint: t("settings.databaseHint"), value: "state/agenthub.db", meta: t("settings.localOnly"), tone: "purple" },
    { icon: Radio, title: t("settings.targets"), hint: t("settings.targetsHint"), value: `${data.enabled_targets.length} ${t("settings.enabled")}`, meta: t("settings.healthy"), tone: "green" },
  ];
  return (
    <>
      <PageHeader title={t("nav.settings")} subtitle={t("settings.subtitle")} />
      <div className="settings-grid">
        {cards.map(({ icon: CardIcon, title, hint, value, meta, tone }) => (
          <section className={`material settings-card settings-card--${tone}`} key={title}>
            <span className="settings-card__icon"><CardIcon size={23} /></span>
            <div><h2>{title}</h2><p>{hint}</p></div>
            <code>{value}</code>
            <span className="settings-card__meta"><CheckCircle2 size={15} />{meta}</span>
          </section>
        ))}
      </div>
      <section className="material security-note">
        <span><KeyRound size={21} /></span>
        <div><h2>MCP Secrets</h2><p>XChaCha20-Poly1305 · <code>secrets/master.key</code></p></div>
        <StatusBadge tone="ok">{t("settings.localOnly")}</StatusBadge>
      </section>
    </>
  );
}

function EmptyState({ icon: EmptyIcon, title, body, compact = false }: { icon: Icon; title: string; body: string; compact?: boolean }) {
  return (
    <div className={`empty-state ${compact ? "empty-state--compact" : ""}`}>
      <span><EmptyIcon size={compact ? 18 : 24} /></span>
      <div><strong>{title}</strong><p>{body}</p></div>
    </div>
  );
}

function Init({ onDone }: { onDone: () => void }) {
  const [items, setItems] = useState<ScanItem[] | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [recover, setRecover] = useState(false);

  const scan = async () => {
    setBusy(true); setError("");
    try { setItems(await api.scan()); setSelected(new Set()); } catch (value) { setError(String(value)); } finally { setBusy(false); }
  };
  const finish = async () => {
    setBusy(true); setError("");
    try { await api.finishInit([...selected]); onDone(); } catch (value) {
      const message = String(value); setError(message); if (message.includes("incomplete Canonical")) setRecover(true);
    } finally { setBusy(false); }
  };
  const discard = async () => {
    setBusy(true);
    try { await api.discardIncompleteInit(); setRecover(false); await scan(); } catch (value) { setError(String(value)); setBusy(false); }
  };

  return (
    <main className="init-shell">
      <section className="init-panel">
        <header className="init-topbar">
          <span className="brand__mark"><img src={agentHubLogo} alt="" /></span>
          <span><strong>AgentHub</strong><small>FIRST RUN</small></span>
          <span className="init-step">{items === null ? "01" : "02"} / 02</span>
        </header>
        <PageHeader eyebrow="CANONICAL SETUP" title={t("init.title")} subtitle={t("init.subtitle")} />
        <div className="init-progress" aria-hidden="true"><i className={items === null ? "is-current" : "is-done"} /><i className={items !== null ? "is-current" : ""} /></div>

        {error && (
          <div className="inline-error" role="alert">
            <Activity size={18} /><span><strong>{t("toast.failed")}</strong>{error}</span>
            {error.includes("incomplete Canonical") && <Button variant="danger" onClick={() => setRecover(true)}>{t("init.recover")}</Button>}
          </div>
        )}

        {items === null ? (
          <div className="init-intro">
            <div className="init-orbit" aria-hidden="true"><span><Sparkles /></span><i /><i /><i /></div>
            <div><h2>{t("init.scan")}</h2><p>{t("init.subtitle")}</p></div>
            <Button disabled={busy} onClick={scan}>{busy ? <RefreshCw className="spin" size={17} /> : <ArrowRight size={17} />} {busy ? t("common.loading") : t("init.scan")}</Button>
          </div>
        ) : (
          <section className="scan-results">
            <header className="scan-toolbar">
              <div><h2>{t("init.found")}</h2><small>{selected.size} {t("init.selected")} · {items.length} total</small></div>
              <div><Button variant="quiet" disabled={busy || selected.size === items.length} onClick={() => setSelected(new Set(items.map((item) => item.id)))}>{t("init.selectAll")}</Button><Button variant="quiet" disabled={busy || selected.size === 0} onClick={() => setSelected(new Set())}>{t("init.clear")}</Button></div>
            </header>
            <div className="scan-list">
              {!items.length && <EmptyState icon={Boxes} title={t("inventory.empty")} body={t("init.none")} />}
              {items.map((item) => {
                const checked = selected.has(item.id);
                return (
                  <label className={checked ? "is-selected" : ""} key={item.id}>
                    <input type="checkbox" disabled={busy} checked={checked} onChange={() => setSelected((old) => { const next = new Set(old); if (next.has(item.id)) next.delete(item.id); else next.add(item.id); return next; })} />
                    <span className="scan-check"><CheckCircle2 size={17} /></span>
                    <span className="scan-copy"><strong>{item.path.split("/").pop()}</strong><small>{item.source} · {item.kind}</small></span>
                    <code>{item.digest.slice(0, 8)}</code>
                  </label>
                );
              })}
            </div>
            <footer className="scan-footer"><span>{selected.size} / {items.length}</span><Button disabled={busy} onClick={finish}>{busy ? <RefreshCw className="spin" size={17} /> : <ArrowRight size={17} />} {busy ? t("init.importing") : t("init.import")}</Button></footer>
          </section>
        )}
      </section>
      <Dialog open={recover} onClose={() => !busy && setRecover(false)} title={t("init.recoverTitle")} actions={<><Button variant="secondary" disabled={busy} onClick={() => setRecover(false)}>{t("common.cancel")}</Button><Button variant="danger" disabled={busy} onClick={discard}>{t("init.recoverAction")}</Button></>}><p>{t("init.recoverBody")}</p></Dialog>
    </main>
  );
}
