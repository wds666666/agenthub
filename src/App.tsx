import { useCallback, useEffect, useMemo, useRef, useState, type Dispatch, type FormEvent, type SetStateAction, type WheelEvent } from "react";
import {
  Activity,
  AlertTriangle,
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
  Cloud,
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
  ListChecks,
  Trash2,
  Upload,
  UserRound,
} from "lucide-react";
import { api, type AutoSyncProfile, type Capability, type CapabilityDetail, type CapabilityChange, type VersionPreview, SavePreview, type CapabilityMutationResult, type Dashboard, type GitIdentity, type HostResource, type Kind, type Plan, type PolicySettings, type RuleDocument, type RuntimeDiagnostics, type RemoteSettings, type ScanItem, type SyncSelection, type Target, type Transaction } from "./lib/api";
import { t } from "./lib/i18n";
import agentHubLogo from "./assets/agenthub-logo.png";
import { Button, Dialog, PageHeader, SearchField, StatusBadge, Toast } from "./components/ui";
import { SyncRail } from "./components/SyncRail";
import { useSkillChanges } from "./lib/useSkillChanges";
import { RepositoryCredentials } from "./components/RepositoryCredentials";
import "./styles/tokens.css";
import "./styles/app.css";

type Page = "overview" | "inventory" | "hosts" | "sync" | "git" | "transactions" | "settings";
type Icon = typeof LayoutDashboard;

const nav: Array<{ id: Page; icon: Icon }> = [
  { id: "overview", icon: LayoutDashboard },
  { id: "inventory", icon: Boxes },
  { id: "hosts", icon: HardDrive },
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

const fullSelection = (items: Capability[], authoritative = false, target: Target = "codex"): SyncSelection => ({
  skills_managed: true,
  skills: items.filter((item) => item.kind === "skill").map((item) => item.id),
  plugins_managed: target === "cursor",
  plugins: target === "cursor" ? items.filter((item) => item.kind === "plugin").map((item) => item.id) : [],
  mcp_managed: target !== "agents",
  mcp: target === "agents" ? [] : items.filter((item) => item.kind === "mcp").map((item) => item.id),
  rules_managed: target !== "agents",
  rules: false,
  authoritative: false,
  mode: authoritative ? "replace" : "preserve",
  rule_ids: target === "agents" ? [] : items.filter((item) => item.kind === "rule").map((item) => item.id),
});
const hasSelection = (selection: SyncSelection) => selection.skills_managed || selection.plugins_managed || selection.mcp_managed || selection.rules_managed || selection.skills.length + selection.plugins.length + selection.mcp.length + (selection.rule_ids?.length ?? 0) > 0;
const sameIds = (left: string[], right: string[]) => JSON.stringify([...left].sort()) === JSON.stringify([...right].sort());
const sameSelection = (left?: SyncSelection | null, right?: SyncSelection | null) => Boolean(left && right
  && (left.mode ?? "preserve") === (right.mode ?? "preserve")
  && sameIds(left.rule_ids ?? [], right.rule_ids ?? [])
  && left.rules === right.rules
  && left.rules_managed === right.rules_managed
  && left.skills_managed === right.skills_managed
  && left.plugins_managed === right.plugins_managed
  && left.mcp_managed === right.mcp_managed
  && left.authoritative === right.authoritative
  && sameIds(left.skills, right.skills)
  && sameIds(left.plugins, right.plugins)
  && sameIds(left.mcp, right.mcp));

const handoffBoundaryWheel = (event: WheelEvent<HTMLElement>) => {
  const node = event.currentTarget;
  const atTop = node.scrollTop <= 0;
  const atBottom = Math.ceil(node.scrollTop + node.clientHeight) >= node.scrollHeight;
  if ((event.deltaY < 0 && atTop) || (event.deltaY > 0 && atBottom)) {
    event.preventDefault();
    window.scrollBy({ top: event.deltaY, behavior: "auto" });
  }
};
const planWarning = (warning: string) => warning === "claude_plugins_cli_managed" ? t("sync.claudePluginConstraint") : warning === "codex_plugins_marketplace_managed" ? t("sync.codexPluginConstraint") : warning;

const targetMeta: Array<{ id: Target; mark: string; description: string }> = [
  { id: "agents", mark: "AG", description: "Shared Agent Skills" },
  { id: "cursor", mark: "CU", description: "Cursor" },
  { id: "codex", mark: "CX", description: "OpenAI Codex" },
  { id: "claude", mark: "CL", description: "Claude Code" },
];
const ruleTargetMeta = targetMeta.filter((item) => item.id !== "agents");

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

function transactionMode(mode: Transaction["mode"]) {
  return t(`transactions.${mode === "auto_sync" || mode === "default_sync" ? "modeAuto" : mode === "rollback" ? "modeRollback" : "modeReviewed"}`);
}

export default function App() {
  const [resetRecovery, setResetRecovery] = useState("");
  const [page, setPage] = useState<Page>("overview");
  const [dashboard, setDashboard] = useState<Dashboard | null>(null);
  const [diagnostics, setDiagnostics] = useState<RuntimeDiagnostics | null>(null);
  const [error, setError] = useState("");
  const [repairing, setRepairing] = useState(false);
  const [toast, setToast] = useState("");
  const skillChanges = useSkillChanges(Boolean(dashboard?.initialized), page);

  const refresh = useCallback(() => {
    setError("");
    api.dashboard().then(setDashboard).catch((value) => setError(String(value)));
  }, []);

  useEffect(refresh, [refresh]);
  useEffect(() => { api.runtimeDiagnostics().then(setDiagnostics).catch(() => undefined); }, []);
  useEffect(() => { document.title = `${t(`nav.${page}`)} · AgentHub`; }, [page]);
  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(""), 3200);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const resetFailedInitialization = async () => {
    if (!window.confirm(t("init.legacyResetConfirm"))) return;
    setRepairing(true);
    try { await api.resetFailedInitialization(); setDashboard(null); refresh(); }
    catch (value) { setError(String(value)); }
    finally { setRepairing(false); }
  };

  if (error) {
    return (
      <main className="center-state">
        <span className="center-state__icon center-state__icon--danger"><Activity /></span>
        <h1>{t("toast.failed")}</h1>
        <p>{error}</p>
        {diagnostics && <p className="diagnostic-hint">{t("common.logHint")}<code>{diagnostics.log_dir}</code></p>}
        <div className="center-state__actions">
          <Button variant="secondary" disabled={repairing} onClick={refresh}><RefreshCw size={17} /> {t("common.retry")}</Button>
          {error.includes("rule body is required") && <Button variant="danger" disabled={repairing} onClick={() => void resetFailedInitialization()}>{repairing ? <RefreshCw className="spin" size={17} /> : <RotateCcw size={17} />}{t("init.legacyReset")}</Button>}
        </div>
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

  if (!dashboard.initialized) return <Init recoveryPath={resetRecovery} onDone={(count, restored) => { setResetRecovery(""); if (restored) setPage("inventory"); setToast(`${count} ${t(restored ? "init.restored" : "inventory.scanImported")}`); refresh(); }} />;

  const content = {
    overview: <Overview data={dashboard} />,
    inventory: <Inventory onChanged={refresh} onNotify={setToast} skillChanges={skillChanges} />,
    hosts: <HostResources onNotify={setToast} />,
    sync: <Sync onApplied={refresh} onNotify={setToast} />,
    git: <GitPage onRecovered={() => void refresh()} onCommitted={() => { setToast(t("toast.committed")); refresh(); }} />,
    transactions: <Transactions data={dashboard} onChanged={refresh} onNotify={setToast} />,
    settings: <SettingsPage data={dashboard} onReset={(path) => { setResetRecovery(path); setPage("overview"); void refresh(); }} />,
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
              aria-label={id === "inventory" && skillChanges.changes.length ? `${t(`nav.${id}`)} · ${t("inventory.skillChanges")} ${skillChanges.changes.length}` : t(`nav.${id}`)}
              onClick={() => setPage(id)}
              key={id}
            >
              <NavIcon size={19} />
              <span>{t(`nav.${id}`)}</span>
              {id === "inventory" && skillChanges.changes.length > 0 && <span className="skill-change-dot" aria-hidden="true" />}
              {page === id && <ChevronRight className="nav-chevron" size={15} aria-hidden="true" />}
            </button>
          ))}
        </nav>
        <div className="source-mark">
          <span className="source-mark__icon"><TerminalSquare size={18} /></span>
          <span><small>AGENTHUB LIBRARY</small><code>~/.agenthub</code></span>
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
                <div><strong>{t(`targets.${tx.target}`)} · {transactionMode(tx.mode ?? "reviewed")}</strong><code>{tx.id.slice(0, 12)}</code></div>
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

function Inventory({ onChanged, onNotify, skillChanges }: { onChanged: () => void; onNotify: (message: string) => void; skillChanges: ReturnType<typeof useSkillChanges> }) {
  const [items, setItems] = useState<Capability[]>([]);
  const [query, setQuery] = useState("");
  const [editor, setEditor] = useState<{ document: RuleDocument; create: boolean; imported: boolean } | null>(null);
  const [loadError, setLoadError] = useState("");
  const [loadingRule, setLoadingRule] = useState(false);
  const [detail, setDetail] = useState<CapabilityDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState("");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [batchCandidate, setBatchCandidate] = useState<Capability[] | null>(null);
  const [deleteBackup, setDeleteBackup] = useState("");
  const keyOf = (item: Capability) => `${item.kind}:${item.id}`;
  const toggleSelection = (group: Capability[]) => setSelected((current) => {
    const next = new Set(current);
    const clear = group.every((item) => next.has(keyOf(item)));
    group.forEach((item) => clear ? next.delete(keyOf(item)) : next.add(keyOf(item)));
    return next;
  });
  const [deleteCandidate, setDeleteCandidate] = useState<Capability | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [scanItems, setScanItems] = useState<ScanItem[] | null>(null);
  const [localSkillOpen, setLocalSkillOpen] = useState(false);
  const [changeReview, setChangeReview] = useState(false);
  const [scanSelected, setScanSelected] = useState<Set<string>>(new Set());
  const [scanKind, setScanKind] = useState<Kind>("skill");
  const [scanSource, setScanSource] = useState("all");
  const [scanBusy, setScanBusy] = useState(false);
  const scanPending = useRef(false);
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
  const saved = (result: CapabilityMutationResult) => {
    setEditor(null);
    loadInventory();
    onChanged();
    const failed = result.auto_sync.filter((outcome) => outcome.error);
    const changed = result.auto_sync.filter((outcome) => outcome.changed);
    if (failed.length) onNotify(`${t("toast.ruleSavedAutoFailed")} ${failed.map((outcome) => t(`targets.${outcome.target}`)).join("、")}`);
    else if (changed.length) onNotify(`${t("toast.ruleSavedAutoSynced")} ${changed.map((outcome) => t(`targets.${outcome.target}`)).join("、")}`);
    else onNotify(t("toast.ruleSaved"));
  };
  const removeCapability = async () => {
    if (!deleteCandidate) return;
    setDeleting(true); setLoadError("");
    try {
      const result = await api.deleteCapability(deleteCandidate.kind, deleteCandidate.id);
      setDeleteCandidate(null); setDetail(null); setSelected((current) => { const next = new Set(current); next.delete(keyOf(deleteCandidate)); return next; }); loadInventory(); onChanged();
      const failed = result.auto_sync.filter((outcome) => outcome.error);
      onNotify(failed.length ? `${t("toast.capabilityDeletedAutoFailed")} ${failed.map((outcome) => t(`targets.${outcome.target}`)).join("、")}` : t("toast.capabilityDeleted"));
    } catch (value) { setLoadError(String(value)); }
    finally { setDeleting(false); }
  };
  const removeBatch = async () => {
    if (!batchCandidate) return;
    setDeleting(true); setLoadError("");
    try {
      const result = await api.deleteCapabilities(batchCandidate.map(({ kind, id }) => ({ kind, id })));
      setBatchCandidate(null); setSelected(new Set()); setDetail(null); setDeleteBackup(result.backup_path);
      loadInventory(); onChanged();
      const failed = result.auto_sync.filter((outcome) => outcome.error);
      if (result.auto_sync_error) setLoadError(result.auto_sync_error);
      onNotify(failed.length || result.auto_sync_error ? t("toast.capabilityDeletedAutoFailed") : `${result.deleted.length} ${t("inventory.batchDeleted")}`);
    } catch (value) { setLoadError(String(value)); }
    finally { setDeleting(false); }
  };
  const scanForImport = async () => {
    if (scanPending.current) return;
    scanPending.current = true;
    setScanBusy(true); setLoadError("");
    try { setScanItems(await api.scan()); setChangeReview(false); setScanSelected(new Set()); setScanSource("all"); }
    catch (value) { setLoadError(String(value)); }
    finally { scanPending.current = false; setScanBusy(false); }
  };
  const importScanned = async () => {
    if (scanPending.current) return;
    scanPending.current = true;
    setScanBusy(true); setLoadError("");
    try {
      const result = await api.importScanned([...scanSelected]);
      setScanItems(null); setScanSelected(new Set()); loadInventory(); onChanged(); void skillChanges.check(true);
      const autoFailures = result.auto_sync.filter((outcome) => outcome.error).length;
      onNotify(`${result.imported.length} ${t("inventory.scanImported")} · ${result.skipped_duplicates} ${t("inventory.scanSkipped")}${autoFailures ? ` · ${autoFailures} ${t("inventory.scanAutoFailed")}` : ""}`);
    } catch (value) { setLoadError(String(value)); }
    finally { scanPending.current = false; setScanBusy(false); }
  };
  const reviewChanges = (id?: string) => {
    setScanItems(skillChanges.changes.filter((change) => !id || change.canonical_id === id).map((change) => change.item));
    setChangeReview(true); setScanSelected(new Set()); setScanSource("all"); setScanKind("skill");
  };
  const canonicalDigests = useMemo(() => new Set(items.map((item) => `${item.kind}:${item.comparison_digest ?? item.digest}`)), [items]);
  const visibleScanItems = (scanItems ?? []).filter((item) => item.kind === scanKind && (scanSource === "all" || item.source === scanSource));
  const scanImportableItems = (scanItems ?? []).filter((item) => item.importable && !canonicalDigests.has(`${item.kind}:${item.comparison_digest ?? item.digest}`));

  return (
    <>
      <PageHeader
        title={t("inventory.title")}
        subtitle={t("inventory.subtitle")}
        actions={
          <div className="inventory-actions">
            <SearchField value={query} onChange={setQuery} />
            <Button variant="secondary" disabled={scanBusy} onClick={() => void scanForImport()}><RefreshCw className={scanBusy ? "spin" : ""} size={16} />{t("inventory.scanImport")}</Button>
            <Button variant="secondary" onClick={() => setLocalSkillOpen(true)}><Upload size={16} />{t("inventory.importSkill")}</Button>
            <input ref={fileRef} className="sr-only" type="file" accept=".md,.mdc,text/markdown,text/plain" onChange={(event) => void importMarkdown(event.target.files?.[0])} />
            <Button variant="secondary" onClick={() => fileRef.current?.click()}><Upload size={16} />{t("inventory.importRule")}</Button>
            <Button onClick={() => setEditor({ document: blankRule(), create: true, imported: false })}><Plus size={16} />{t("inventory.newRule")}</Button>
          </div>
        }
      />
      {loadError && <div className="inline-error" role="alert"><Activity size={18} /><span>{loadError}</span></div>}
      {(skillChanges.changes.length > 0 || skillChanges.checking || skillChanges.error || skillChanges.errors.length > 0) && <div className="skill-change-notice material">
        <div role="status">{skillChanges.changes.length > 0 && <p><span className="skill-change-dot" aria-hidden="true" /><strong>{skillChanges.changes.length} {t("inventory.skillChanges")}</strong></p>}{skillChanges.checking && <p><RefreshCw className="spin" size={15} />{t("inventory.checkingSkills")}</p>}{(skillChanges.error || skillChanges.errors.length > 0) && <p className="skill-check-error">{t("inventory.skillCheckFailed")}<span>{skillChanges.error || `${skillChanges.errors.length} ${t("inventory.skillCheckUnavailable")}`}</span></p>}</div>
        <div className="skill-change-actions">{skillChanges.changes.length > 0 && <Button variant="secondary" disabled={scanBusy} onClick={() => reviewChanges()}>{t("inventory.reviewSkillChanges")}</Button>}{(skillChanges.error || skillChanges.errors.length > 0) && <Button variant="quiet" disabled={skillChanges.checking} onClick={() => void skillChanges.check(true)}>{t("common.retry")}</Button>}</div>
      </div>}
      <div className="inventory-selection material" role="group" aria-label={t("inventory.batchManage")}>
        <span role="status">{t("inventory.selectedCount")} <strong>{items.filter((item) => selected.has(keyOf(item))).length}</strong></span>
        <Button variant="quiet" disabled={deleting || !filtered.length} onClick={() => toggleSelection(filtered)}>{filtered.length > 0 && filtered.every((item) => selected.has(keyOf(item))) ? t("inventory.clearVisible") : t("inventory.selectVisible")}</Button>
        <Button variant="quiet" disabled={deleting || !selected.size} onClick={() => setSelected(new Set())}>{t("init.clear")}</Button>
        <Button variant="danger" disabled={deleting || !items.some((item) => selected.has(keyOf(item)))} onClick={() => setBatchCandidate(items.filter((item) => selected.has(keyOf(item))))}><Trash2 size={16} />{t("inventory.deleteSelected")}</Button>
      </div>
      {deleteBackup && <div className="warning-banner" role="status"><ShieldCheck size={17} /><span>{t("inventory.deleteBackup")} <code>{deleteBackup}</code></span></div>}
      <div className="inventory-grid">
        {kindMeta.map(({ id, icon: KindIcon, accent }) => {
          const group = filtered.filter((item) => item.kind === id);
          return (
            <section className={`material inventory-card inventory-card--${accent}`} key={id}>
              <header className="inventory-card__header">
                <span className="inventory-card__icon"><KindIcon size={20} /></span>
                <div><h2>{t(`kinds.${id}`)}</h2><p>{group.length}</p></div>
                <Button variant="quiet" disabled={deleting || !group.length} onClick={() => toggleSelection(group)} aria-label={`${group.length > 0 && group.every((item) => selected.has(keyOf(item))) ? t("inventory.clearCategory") : t("inventory.selectCategory")} ${t(`kinds.${id}`)}`}>{group.length > 0 && group.every((item) => selected.has(keyOf(item))) ? t("inventory.clearCategory") : t("inventory.selectCategory")}</Button>
              </header>
              <div className="capability-list">
                {group.map((item) => (
                  <article className={`capability-row ${selected.has(keyOf(item)) ? "is-selected" : ""}`} key={item.id}>
                    <input className="capability-select" type="checkbox" aria-label={`${t("inventory.selectItem")} ${t(`kinds.${item.kind}`)} ${item.display_name}`} checked={selected.has(keyOf(item))} disabled={deleting} onChange={() => toggleSelection([item])} />
                    <button className="capability-open" type="button" disabled={Boolean(loadingDetail)} onClick={() => void openDetail(item)} aria-label={`${t("inventory.viewDetails")} ${item.display_name}`}>
                      <span className="capability-row__glyph">{loadingDetail === `${item.kind}:${item.id}` ? <RefreshCw className="spin" size={16} /> : <KindIcon size={17} />}</span>
                      <span className="capability-row__copy">
                        <strong>{item.display_name}</strong>
                        <small>{item.path}</small>
                      </span>
                      {item.duplicate_of && <span className="source-pill source-pill--constraint">{t("inventory.duplicateOf")} {item.duplicate_of}</span>}<code title={t("inventory.digest")}>{item.digest.slice(0, 8)}</code>
                      <ChevronRight size={15} />
                    </button>
                    {id === "skill" && skillChanges.changes.some((change) => change.canonical_id === item.id) && <button type="button" className="skill-change-trigger" disabled={scanBusy} aria-label={`${t("inventory.reviewSkillChanges")} ${item.display_name}`} title={t("inventory.skillChanges")} onClick={() => reviewChanges(item.id)}><span className="skill-change-dot" aria-hidden="true" /></button>}
                    {id === "rule" && (
                      <button className="capability-edit" type="button" disabled={loadingRule} onClick={() => void editRule(item.id)} aria-label={`${t("common.edit")} ${item.display_name}`}>
                        <Pencil size={15} />
                      </button>
                    )}
                    <button className="capability-delete" type="button" disabled={deleting} onClick={() => setDeleteCandidate(item)} aria-label={`${t("inventory.delete")} ${item.display_name}`}>
                      <Trash2 size={15} />
                    </button>
                  </article>
                ))}
                {!group.length && <EmptyState icon={KindIcon} title={t("inventory.empty")} body={t("inventory.emptyHint")} compact />}
              </div>
            </section>
          );
        })}
      </div>
      {editor && <RuleEditor key={`${editor.create ? "new" : "edit"}-${editor.document.id}`} initial={editor.document} create={editor.create} imported={editor.imported} onClose={() => setEditor(null)} onSaved={saved} />}
      {localSkillOpen && <LocalSkillImportDialog onClose={() => setLocalSkillOpen(false)} onImported={(result) => {
        setLocalSkillOpen(false); loadInventory(); onChanged();
        onNotify(`${result.imported.length} ${t("inventory.scanImported")} · ${result.skipped_duplicates} ${t("inventory.scanSkipped")}`);
      }} />}
      {detail && <CapabilityDetailDialog detail={detail} onClose={() => setDetail(null)} onDelete={() => setDeleteCandidate(detail.capability)} onEditRule={detail.capability.kind === "rule" ? () => { const id = detail.capability.id; setDetail(null); void editRule(id); } : undefined} />}
      <Dialog open={batchCandidate !== null} onClose={() => !deleting && setBatchCandidate(null)} title={t("inventory.batchTitle")} actions={<><Button variant="secondary" disabled={deleting} onClick={() => setBatchCandidate(null)}>{t("common.cancel")}</Button><Button variant="danger" disabled={deleting} onClick={() => void removeBatch()}>{deleting ? <RefreshCw className="spin" size={16} /> : <Trash2 size={16} />}{t("inventory.deleteSelected")} ({batchCandidate?.length ?? 0})</Button></>}>
        <p>{t("inventory.batchBody")}</p>
        <ul className="batch-delete-list">{batchCandidate?.map((item) => <li key={keyOf(item)}><strong>{item.display_name}</strong><span>{t(`kinds.${item.kind}`)} · {item.id}</span></li>)}</ul>
        {loadError && <div className="inline-error" role="alert">{loadError}</div>}
      </Dialog>
      <Dialog open={Boolean(deleteCandidate)} onClose={() => !deleting && setDeleteCandidate(null)} title={t("inventory.deleteTitle")} actions={<><Button variant="secondary" disabled={deleting} onClick={() => setDeleteCandidate(null)}>{t("common.cancel")}</Button><Button variant="danger" disabled={deleting} onClick={() => void removeCapability()}>{deleting ? <RefreshCw className="spin" size={16} /> : <Trash2 size={16} />}{t("inventory.deleteAction")}</Button></>}>
        <p>{t("inventory.deleteBody")}</p>
        <div className="delete-capability-preview"><span className="target-tab__mark">{deleteCandidate ? kindMeta.find((kind) => kind.id === deleteCandidate.kind)?.id.slice(0, 2).toUpperCase() : ""}</span><span><strong>{deleteCandidate?.display_name}</strong><code>{deleteCandidate?.id}</code></span></div>
      </Dialog>
      <Dialog open={scanItems !== null} wide onClose={() => !scanBusy && setScanItems(null)} title={t(changeReview ? "inventory.reviewSkillChanges" : "inventory.scanImportTitle")} actions={<><Button variant="secondary" disabled={scanBusy} onClick={() => setScanItems(null)}>{t("common.cancel")}</Button><Button disabled={scanBusy || scanSelected.size === 0} onClick={() => void importScanned()}>{scanBusy ? <RefreshCw className="spin" size={16} /> : <Upload size={16} />}{scanBusy ? t("inventory.importingScan") : t("inventory.importScanned")}</Button></>}>
        <p>{t(changeReview ? "inventory.skillChangeImportHint" : "inventory.scanImportBody")}</p>
        <ImportSourcePicker items={scanItems ?? []} available={scanImportableItems} selected={scanSelected} source={scanSource} onSource={setScanSource} onSelected={setScanSelected} busy={scanBusy} />
        <div className="scan-kind-tabs" role="tablist" aria-label={t("init.groups")}>
          {kindMeta.map(({ id, icon: KindIcon }) => { const group = (scanItems ?? []).filter((item) => item.kind === id && (scanSource === "all" || item.source === scanSource)); return <button type="button" role="tab" aria-selected={scanKind === id} className={scanKind === id ? "is-selected" : ""} disabled={scanBusy} onClick={() => setScanKind(id)} key={id}><KindIcon size={17} /><span><strong>{t(`kinds.${id}`)}</strong><small>{group.filter((item) => scanSelected.has(item.id)).length} / {group.length}</small></span></button>; })}
        </div>
        <div className="scan-group-toolbar"><span>{visibleScanItems.length} {t("init.discovered")}</span><Button variant="quiet" disabled={scanBusy || !scanImportableItems.some((item) => item.kind === scanKind && (scanSource === "all" || item.source === scanSource))} onClick={() => setScanSelected((current) => { const next = new Set(current); const available = scanImportableItems.filter((item) => item.kind === scanKind && (scanSource === "all" || item.source === scanSource)); const allSelected = available.every((item) => next.has(item.id)); available.forEach((item) => allSelected ? next.delete(item.id) : next.add(item.id)); return next; })}>{t("init.selectGroup")}</Button></div>
        <div className="scan-list scan-list--dialog">
          {visibleScanItems.length === 0 && <EmptyState icon={Boxes} title={t("init.emptyGroup")} body={t("init.emptyGroupHint")} compact />}
          {visibleScanItems.map((item) => { const exists = canonicalDigests.has(`${item.kind}:${item.comparison_digest ?? item.digest}`); const disabled = !item.importable || exists; const checked = scanSelected.has(item.id); const name = item.source_key?.replace(/^(server|rule):/, "") ?? item.path.split(/[\\/]/).pop(); return <label className={`${checked ? "is-selected" : ""} ${disabled ? "is-disabled" : ""}`} key={item.id}><input type="checkbox" checked={checked} disabled={scanBusy || disabled} onChange={() => setScanSelected((current) => { const next = new Set(current); if (next.has(item.id)) next.delete(item.id); else next.add(item.id); return next; })} /><span className="scan-check">{disabled ? <AlertTriangle size={15} /> : <CheckCircle2 size={17} />}</span><span className="scan-copy"><strong>{name}</strong><small><b>{item.source}</b><span>{item.path}</span></small>{exists && <em>{t("inventory.alreadyCanonical")}</em>}{!exists && <ScanWarning item={item} />}</span><code>{item.digest.slice(0, 8)}</code></label>; })}
        </div>
      </Dialog>
    </>
  );
}

function LocalSkillImportDialog({ onClose, onImported }: { onClose: () => void; onImported: (result: Awaited<ReturnType<typeof api.importLocalSkill>>) => void }) {
  const [preview, setPreview] = useState<Awaited<ReturnType<typeof api.pickLocalSkill>>>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const choose = async () => {
    if (pending.current) return;
    pending.current = true; setBusy(true); setError(""); setPreview(null);
    try { setPreview(await api.pickLocalSkill()); }
    catch (value) { setError(String(value)); }
    finally { pending.current = false; setBusy(false); }
  };
  const apply = async () => {
    if (pending.current || !preview?.item.importable || preview.duplicate_of) return;
    pending.current = true; setBusy(true); setError("");
    try { onImported(await api.importLocalSkill(preview.id)); }
    catch (value) { setError(String(value)); }
    finally { pending.current = false; setBusy(false); }
  };
  return <Dialog open wide title={t("inventory.importSkill")} dismissible={!busy} onClose={() => !busy && onClose()} actions={<>
    <Button variant="secondary" disabled={busy} onClick={onClose}>{t("common.cancel")}</Button>
    <Button disabled={busy || !preview?.item.importable || Boolean(preview?.duplicate_of)} onClick={() => void apply()}>{busy ? <RefreshCw className="spin" size={16} /> : <Upload size={16} />}{t("inventory.confirmSkillImport")}</Button>
  </>}>
    <p>{t("inventory.localSkillHint")}</p>
    <Button variant="secondary" disabled={busy} data-autofocus onClick={() => void choose()}><FolderTree size={16} />{t("inventory.chooseSkillFolder")}</Button>
    {busy && <p role="status">{t("inventory.localSkillBusy")}</p>}
    {error && <div className="inline-error" role="alert">{error}</div>}
    {preview && <div className="local-skill-preview">
      <p className="local-skill-source"><strong>{t("inventory.skillSource")}</strong><code>{preview.item.path}</code></p>
      {preview.duplicate_of && <div className="warning-banner" role="status"><CheckCircle2 size={17} /><span>{t("inventory.alreadyCanonical")} · {preview.duplicate_of}</span></div>}
      {preview.item.warning === "capability_modified" && <p>{t("inventory.skillNameConflict")}</p>}
      <details open><summary>{t("inventory.fileInventory")} · {preview.files.length} {t("inventory.files")}</summary>
        <ul className="local-skill-files">{preview.files.map((file) => <li key={file.path}><code>{file.path}</code><small>{file.size.toLocaleString()} B</small></li>)}</ul>
      </details>
      {preview.excluded.length > 0 && <details><summary>{t("inventory.skillExclusions")} · {preview.excluded.length}</summary><ul className="local-skill-files">{preview.excluded.map((path) => <li key={path}><code>{path}</code></li>)}</ul></details>}
    </div>}
  </Dialog>;
}

function HostResources({ onNotify }: { onNotify: (message: string) => void }) {
  const [target, setTarget] = useState<Target>("cursor");
  const [items, setItems] = useState<HostResource[]>([]);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [acknowledged, setAcknowledged] = useState(false);
  const load = useCallback(async (nextTarget = target) => {
    setLoading(true); setError(""); setSelected(new Set());
    try { setItems(await api.hostInventory(nextTarget)); }
    catch (value) { setError(String(value)); setItems([]); }
    finally { setLoading(false); }
  }, [target]);
  useEffect(() => { void load(); }, [load]);
  const chosen = items.filter((item) => selected.has(item.id));
  // "Not imported" means host-only content that the adapter allows us to delete; Canonical matches and protected constraints are never auto-selected.
  const deletable = items.filter((item) => item.deletable);
  const unimported = items.filter((item) => item.relation === "host_only" && item.deletable);
  const openConfirm = () => { setAcknowledged(false); setConfirming(true); };
  const remove = async () => {
    setDeleting(true); setError("");
    try {
      const result = await api.cleanupHostResources(target, [...selected]);
      setConfirming(false); await load(target);
      onNotify(`${result.deleted.length} ${t("hosts.deleted")} · ${result.backup_path}`);
    } catch (value) { setError(String(value)); }
    finally { setDeleting(false); }
  };
  return <>
    <PageHeader eyebrow="HOST INVENTORY" title={t("hosts.title")} subtitle={t("hosts.subtitle")} actions={<><Button variant="danger" disabled={loading || deleting || !deletable.length} onClick={() => { setSelected(new Set(deletable.map((item) => item.id))); openConfirm(); }}><Trash2 size={16} />{t("hosts.quickClean")} ({deletable.length})</Button><Button variant="secondary" disabled={loading || deleting || !unimported.length} title={unimported.length ? undefined : t("hosts.noUnimported")} onClick={() => setSelected(new Set(unimported.map((item) => item.id)))}><ListChecks size={16} />{t("hosts.selectUnimported")} ({unimported.length})</Button><Button variant="secondary" disabled={loading || deleting} onClick={() => void load(target)}><RefreshCw className={loading ? "spin" : ""} size={16} />{t("hosts.rescan")}</Button></>} />
    <div className="target-tabs host-target-tabs" role="tablist" aria-label={t("hosts.chooseTarget")}>{targetMeta.map((meta) => <button type="button" role="tab" aria-selected={target === meta.id} className={target === meta.id ? "is-selected" : ""} onClick={() => { if (deleting) return; setConfirming(false); setAcknowledged(false); setTarget(meta.id); }} key={meta.id}><span className="target-tab__mark">{meta.mark}</span><span><strong>{t(`targets.${meta.id}`)}</strong><small>{meta.description}</small></span><CheckCircle2 size={17} /></button>)}</div>
    <div className="host-legend"><span className="source-pill source-pill--canonical">{t("hosts.canonicalMatch")}</span><span className="source-pill source-pill--host">{t("hosts.hostOnly")}</span><span className="source-pill source-pill--constraint">{t("hosts.constraint")}</span><p>{t("hosts.legendHint")}</p></div>
    {error && <div className="inline-error" role="alert"><Activity size={18} /><span>{error}</span></div>}
    {loading ? <div className="material host-loading"><span className="spinner" /><p>{t("hosts.scanning")}</p></div> : <div className="inventory-grid host-grid">{kindMeta.map(({ id, icon: KindIcon, accent }) => { const group = items.filter((item) => item.kind === id); return <section className={`material inventory-card inventory-card--${accent}`} key={id}><header className="inventory-card__header"><span className="inventory-card__icon"><KindIcon size={20} /></span><div><h2>{t(`kinds.${id}`)}</h2><p>{group.length} {t("hosts.resources")}</p></div><span className="count-badge">{group.length}</span></header><div className="host-resource-list">{group.map((item) => <label className={`${selected.has(item.id) ? "is-selected" : ""} ${!item.deletable ? "is-protected" : ""}`} key={item.id}><input type="checkbox" disabled={!item.deletable || deleting} checked={selected.has(item.id)} onChange={() => setSelected((current) => { const next = new Set(current); if (next.has(item.id)) next.delete(item.id); else next.add(item.id); return next; })} /><span className="host-resource__icon">{item.deletable ? <CheckCircle2 size={17} /> : <ShieldCheck size={17} />}</span><span className="host-resource__copy"><strong>{item.display_name}</strong><code>{item.path}</code><small>{item.canonical_id ? `${t("hosts.canonicalId")}: ${item.canonical_id}` : item.constraint ? t(`hosts.constraints.${item.constraint}`) : item.relation === "generated" ? t("hosts.relations.generated") : t("hosts.notCanonical")}</small></span><span className={`source-pill source-pill--${item.relation === "canonical_match" ? "canonical" : item.relation === "host_only" ? "host" : "constraint"}`}>{t(`hosts.relations.${item.relation}`)}</span></label>)}{!group.length && <EmptyState icon={KindIcon} title={t("hosts.empty")} body={t("hosts.emptyHint")} compact />}</div></section>; })}</div>}
    {selected.size > 0 && <div className="host-cleanup-bar"><div><strong>{selected.size} {t("hosts.selected")}</strong><span>{t("hosts.cleanupHint")}</span></div><span className="host-cleanup-bar__actions"><Button variant="secondary" disabled={deleting} onClick={() => setSelected(new Set())}>{t("hosts.clearSelection")}</Button><Button variant="danger" disabled={deleting} onClick={openConfirm}><Trash2 size={16} />{t("hosts.deleteSelected")}</Button></span></div>}
    <Dialog open={confirming} onClose={() => !deleting && setConfirming(false)} title={t("hosts.deleteTitle")} actions={<><Button variant="secondary" disabled={deleting} onClick={() => setConfirming(false)}>{t("common.cancel")}</Button><Button variant="danger" disabled={deleting || !acknowledged || !chosen.length} onClick={() => void remove()}>{deleting ? <RefreshCw className="spin" size={16} /> : <Trash2 size={16} />}{t("hosts.deleteAction")}</Button></>}><p>{t("hosts.deleteBody")}</p><p>{t("hosts.protectedHint")}</p><div className="host-delete-preview">{chosen.map((item) => <span key={item.id}><strong>{item.display_name}</strong><code>{item.path}</code></span>)}</div>{chosen.some((item) => item.relation === "canonical_match") && <div className="warning-banner"><AlertTriangle size={17} /><span>{t("hosts.driftWarning")}</span></div>}<label className="confirm-ack"><input type="checkbox" checked={acknowledged} disabled={deleting} onChange={(event) => setAcknowledged(event.target.checked)} /><span>{chosen.length} · {t("hosts.cleanupAck")}</span></label></Dialog>
  </>;
}

function CapabilityDetailDialog({ detail, onClose, onEditRule, onDelete }: { detail: CapabilityDetail; onClose: () => void; onEditRule?: () => void; onDelete: () => void }) {
  const meta = kindMeta.find((item) => item.id === detail.capability.kind) ?? kindMeta[0];
  const DetailIcon = meta.icon;
  const bytes = (size: number) => size < 1024 ? `${size} B` : `${(size / 1024).toFixed(size < 10_240 ? 1 : 0)} KB`;
  return (
    <Dialog
      open
      wide
      onClose={onClose}
      title={detail.capability.display_name}
      actions={<><Button variant="danger" onClick={onDelete}><Trash2 size={15} />{t("inventory.delete")}</Button>{onEditRule && <Button variant="secondary" onClick={onEditRule}><Pencil size={15} />{t("inventory.editRule")}</Button>}<Button onClick={onClose}>{t("common.close")}</Button></>}
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

function RuleEditor({ initial, create, imported, onClose, onSaved }: { initial: RuleDocument; create: boolean; imported: boolean; onClose: () => void; onSaved: (result: CapabilityMutationResult) => void }) {
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
      const result = await api.saveRule({ ...draft, displayName: draft.displayName.trim(), paths }, create);
      void api.debugEvent("rule_save_ok", `id=${draft.id}`);
      setDirty(false);
      onSaved(result);
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
          <fieldset className="field-group"><legend>{t("inventory.targets")}</legend><div className="target-checks">{ruleTargetMeta.map((item) => <label className={draft.targets.includes(item.id) ? "is-selected" : ""} key={item.id}><input type="checkbox" checked={draft.targets.includes(item.id)} onChange={() => toggleTarget(item.id)} /><span className="target-tab__mark">{item.mark}</span><strong>{t(`targets.${item.id}`)}</strong><CheckCircle2 size={17} /></label>)}</div>{errors.targets && <em role="alert">{errors.targets}</em>}</fieldset>
          <label className="field"><span>{t("inventory.body")}</span><textarea className="rule-body resize-none" value={draft.body} aria-invalid={Boolean(errors.body)} aria-describedby={errors.body ? "rule-body-help rule-body-error" : "rule-body-help"} onChange={(event) => update("body", event.target.value)} /><small id="rule-body-help">{t("inventory.bodyHint")}</small>{errors.body && <em id="rule-body-error" role="alert">{errors.body}</em>}</label>
        </form>
      )}
    </Dialog>
  );
}

function Sync({ onApplied, onNotify }: { onApplied: () => void; onNotify: (message: string) => void }) {
  const [target, setTarget] = useState<Target>("codex");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [inventory, setInventory] = useState<Capability[]>([]);
  const [profiles, setProfiles] = useState<AutoSyncProfile[]>([]);
  const [policy, setPolicy] = useState<PolicySettings>({ strict_authoritative: false, sync_after_reverse_import: false });
  const [selection, setSelection] = useState<SyncSelection | null>(null);
  const [selectionQuery, setSelectionQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [confirm, setConfirm] = useState<"reviewed" | "enable_auto" | "disable_auto" | null>(null);
  const [error, setError] = useState("");
  const planSummary = useMemo(() => kindMeta.map((kind) => ({
    ...kind,
    summary: plan?.summary?.find((item) => item.kind === kind.id) ?? { kind: kind.id, affected: 0, create: 0, update: 0, delete: 0, skip: 0, files: 0 },
  })), [plan]);
  const planCapabilityChanges = useMemo(() => {
    const changes = new Map<string, { kind: Kind; id: string; action: string; files: number }>();
    for (const step of plan?.steps ?? []) {
      if (!step.capability_kind) continue;
      const id = step.capability_id && step.capability_id !== "__projection__" ? step.capability_id : t("sync.domainProjection");
      const key = `${step.capability_kind}:${id}`;
      const existing = changes.get(key);
      const action = step.action === "update" ? "replace" : step.action;
      if (existing) {
        existing.files += 1;
        if (existing.action !== action) existing.action = "replace";
      } else {
        changes.set(key, { kind: step.capability_kind, id, action, files: 1 });
      }
    }
    for (const item of plan?.removed_resources ?? []) {
      const id = item.id === "__user_text__" ? t("sync.userRuleText") : item.id;
      changes.set(`${item.kind}:${id}`, { kind: item.kind, id, action: "delete", files: 0 });
    }
    return [...changes.values()].sort((left, right) => `${left.kind}:${left.id}`.localeCompare(`${right.kind}:${right.id}`));
  }, [plan]);
  const affectedCapabilities = planSummary.reduce((sum, item) => sum + item.summary.affected, 0);
  useEffect(() => {
    Promise.all([api.inventory(), api.autoSyncProfiles(), api.policySettings()]).then(([items, nextProfiles, nextPolicy]) => {
      setInventory(items);
      setProfiles(nextProfiles);
      setPolicy(nextPolicy);
      const saved = nextProfiles.find((profile) => profile.target === target);
      setSelection(saved && hasSelection(saved.selection) ? saved.selection : { ...fullSelection(items, false, target), mode: nextPolicy.sync_mode ?? "preserve", rules_managed: false, rule_ids: [] });
    }).catch((value) => setError(String(value)));
  }, [target]);
  const currentProfile = profiles.find((profile) => profile.target === target);
  const autoEnabled = Boolean(currentProfile?.enabled);
  const scopeChanged = autoEnabled && !sameSelection(selection, currentProfile?.selection);
  const selectionItems = useMemo(() => inventory.filter((item) => `${item.id} ${item.display_name}`.toLowerCase().includes(selectionQuery.toLowerCase())), [inventory, selectionQuery]);
  const selectedCount = selection ? selection.skills.length + selection.plugins.length + selection.mcp.length + (selection.rule_ids?.length ?? 0) : 0;
  const profileSelectionCount = (profile?: AutoSyncProfile) => profile ? profile.selection.skills.length + profile.selection.plugins.length + profile.selection.mcp.length + (profile.selection.rule_ids?.length ?? 0) : 0;
  const visibleKinds: Array<Kind> = target === "agents" ? ["skill"] : ["skill", "plugin", "mcp", "rule"];
  const toggleCapability = (item: Capability) => {
    if (!selection) return;
    if (item.kind === "plugin" && target !== "cursor") return;
    const key = item.kind === "skill" ? "skills" : item.kind === "plugin" ? "plugins" : item.kind === "rule" ? "rule_ids" : "mcp";
    const managedKey = (key === "rule_ids" ? "rules_managed" : `${key}_managed`) as "skills_managed" | "plugins_managed" | "mcp_managed" | "rules_managed";
    const values = selection[key] ?? [];
    setSelection({ ...selection, [managedKey]: true, [key]: values.includes(item.id) ? values.filter((id) => id !== item.id) : [...values, item.id] });
    setPlan(null);
  };
  const chooseTarget = (nextTarget: Target) => {
    setTarget(nextTarget);
    setPlan(null);
    const saved = profiles.find((profile) => profile.target === nextTarget);
    setSelection(saved && hasSelection(saved.selection) ? saved.selection : { ...fullSelection(inventory, false, nextTarget), mode: policy.sync_mode ?? "preserve", rules_managed: false, rule_ids: [] });
  };

  const makePlan = async () => {
    setBusy(true);
    setError("");
    void api.debugEvent("sync_plan_click", `target=${target}`);
    if (!selection) { setBusy(false); return; }
    try { const next = await api.plan(target, selection); setPlan(next); void api.debugEvent("sync_plan_ready", `target=${target} steps=${next.steps.length}`); }
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
      setConfirm(null);
      setPlan(null);
      onApplied();
      onNotify(t("toast.applied"));
    } catch (value) { void api.debugEvent("sync_apply_failed", `target=${target} plan=${plan.id}`); setError(String(value)); } finally { setBusy(false); }
  };
  const reviewAutoSync = async () => {
    if (!selection) return;
    if (selection.mode === "replace") {
      setBusy(true); setError("");
      try { setPlan(await api.plan(target, selection)); setConfirm("enable_auto"); }
      catch (value) { setError(String(value)); }
      finally { setBusy(false); }
    } else { setConfirm("enable_auto"); }
  };
  const configureAutoSync = async (enabled: boolean) => {
    if (!selection) return;
    setBusy(true);
    setError("");
    void api.debugEvent("auto_sync_configure_click", `target=${target} enabled=${enabled} selected=${selectedCount}`);
    try {
      const effectiveSelection = enabled ? selection : currentProfile?.selection ?? selection;
      const result = await api.setAutoSync(target, effectiveSelection, enabled, enabled && effectiveSelection.mode === "replace" ? plan?.id : undefined);
      setProfiles((current) => [...current.filter((profile) => profile.target !== target), result.profile]);
      setConfirm(null);
      setPlan(null);
      if (result.initial_sync?.changed) {
        onApplied();
        onNotify(t("toast.autoEnabledSynced"));
      } else if (enabled) {
        onNotify(t("toast.autoEnabled"));
      } else {
        onNotify(t("toast.autoDisabled"));
      }
    } catch (value) {
      void api.debugEvent("auto_sync_configure_failed", `target=${target} enabled=${enabled}`);
      setError(String(value));
    } finally { setBusy(false); }
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
                onClick={() => chooseTarget(item.id)}
                key={item.id}
              >
                <span className="target-tab__mark">{item.mark}</span>
                <span><strong>{t(`targets.${item.id}`)}</strong><small className={profiles.find((profile) => profile.target === item.id)?.enabled ? "is-auto" : undefined}>{profiles.find((profile) => profile.target === item.id)?.enabled ? `${t("sync.autoOn")} · ${profileSelectionCount(profiles.find((profile) => profile.target === item.id))} ${t("sync.selected")}` : item.description}</small></span>
                <span className="target-tab__radio"><CheckCircle2 size={18} /></span>
              </button>
            ))}
          </div>
        </div>

        <section className="material sync-selection">
          <header className="sync-selection__header">
            <div><p className="eyebrow">SYNC SCOPE</p><h2>{t("sync.scopeTitle")}</h2><p>{target === "agents" ? t("sync.agentsHint") : t("sync.scopeHint")}</p></div>
            <StatusBadge tone={selection?.mode === "replace" ? "warning" : "ok"}>{`${selectedCount} / ${inventory.length} ${t("sync.selected")}`}</StatusBadge>
          </header>
          <div className="sync-selection__toolbar">
            <SearchField value={selectionQuery} onChange={setSelectionQuery} />
            <Button variant="quiet" disabled={!selection} onClick={() => { setSelection({ ...fullSelection(inventory, false, target), mode: selection?.mode ?? "preserve" }); setPlan(null); }}>{t("sync.selectAll")}</Button>
            <Button variant="quiet" disabled={!selection} onClick={() => { setPlan(null); if (selection) setSelection({ ...selection, skills_managed: true, skills: [], plugins_managed: target === "cursor", plugins: [], mcp_managed: target !== "agents", mcp: [], rules_managed: target !== "agents", rules: false, rule_ids: [], authoritative: false }); }}>{t("sync.clearManaged")}</Button>
          </div>
          <SyncModePicker value={selection?.mode ?? "preserve"} disabled={busy} onChange={(mode) => { if (selection) setSelection({ ...selection, mode }); setPlan(null); }} />
          {currentProfile?.needs_review && <p className="warning-banner" role="status">{t("sync.needsReview")}</p>}
          <p className="scope-impact-note"><ShieldCheck size={15} />{t(selection?.mode === "replace" ? "sync.replaceHint" : "sync.preserveHint")}</p>
          <div className="sync-selection__groups">
            {visibleKinds.map((kind) => {
              const items = selectionItems.filter((item) => item.kind === kind);
              const key = kind === "skill" ? "skills" : kind === "plugin" ? "plugins" : kind === "rule" ? "rule_ids" : "mcp";
              const managedKey = (key === "rule_ids" ? "rules_managed" : `${key}_managed`) as "skills_managed" | "plugins_managed" | "mcp_managed" | "rules_managed";
              const selected = selection?.[key] ?? [];
              const locked = kind === "plugin" && target !== "cursor";
              const managed = !locked && Boolean(selection?.[managedKey]);
              return <details open key={kind} className={`sync-selection__group ${managed ? "is-managed" : ""} ${locked ? "is-locked" : ""}`}><summary><span>{t(`kinds.${kind}`)}</span><small>{locked ? t("sync.cliManaged") : managed ? `${selected.length} / ${inventory.filter((item) => item.kind === kind).length}` : t("sync.notManaged")}</small></summary><div onWheel={handoffBoundaryWheel}>{locked ? <div className="sync-domain-constraint"><ShieldCheck size={16} /><span>{t(target === "claude" ? "sync.claudePluginConstraint" : "sync.codexPluginConstraint")}</span></div> : <><label className={`sync-domain-toggle ${managed ? "is-selected" : ""}`}><input type="checkbox" checked={managed} disabled={busy} onChange={() => { if (selection) setSelection({ ...selection, [managedKey]: !managed, [key]: managed ? [] : selected, authoritative: false }); setPlan(null); }} /><span className="sync-selection__check"><CheckCircle2 size={15} /></span><span><strong>{t("sync.manageDomain")}</strong><small>{managed ? t(selection?.mode === "replace" ? "sync.emptyMeansClear" : "sync.preserveHint") : t("sync.domainUntouched")}</small></span></label>{items.map((item) => <label key={item.id} className={selected.includes(item.id) ? "is-selected" : ""}><input type="checkbox" disabled={!managed} checked={selected.includes(item.id)} onChange={() => toggleCapability(item)} /><span className="sync-selection__check"><CheckCircle2 size={15} /></span><span><strong>{item.display_name}</strong><small>{item.id}</small></span></label>)}</>}</div></details>;
            })}

          </div>
        </section>

        <div className="warning-band"><ShieldCheck size={19} /><span>{t(selection?.mode === "replace" ? "sync.warning" : "sync.preserveHint")}</span></div>
        {error && <div className="inline-error" role="alert"><Activity size={18} /><span>{error}</span></div>}

        <section className={`material plan-panel ${plan ? "has-plan" : ""}`}>
          <div className="plan-panel__summary">
            <span className="plan-panel__icon">{plan ? <CheckCircle2 /> : <FileCode2 />}</span>
            <div>
              <p className="eyebrow">PLAN / {target.toUpperCase()}</p>
              <h2>{plan ? (plan.steps.length ? `${affectedCapabilities} ${t("sync.capabilityChanges")} · ${plan.steps.length} ${t("sync.steps")}` : t("sync.noChanges")) : t("sync.noPlan")}</h2>
              <p>{plan ? t("sync.ready") : t("sync.noPlanHint")}</p>
            </div>
            <div className="plan-panel__actions">
              <Button variant="secondary" onClick={makePlan} disabled={busy || !selection}>{busy ? <RefreshCw className="spin" size={17} /> : <Braces size={17} />} {busy ? t("sync.planning") : t("sync.plan")}</Button>
              <Button onClick={reviewAutoSync} disabled={busy || !selection || !hasSelection(selection) || (autoEnabled && !scopeChanged)}><Radio size={17} /> {autoEnabled ? (scopeChanged ? t("sync.updateAuto") : t("sync.autoEnabled")) : t("sync.enableAuto")}</Button>
              {autoEnabled && <Button variant="quiet" onClick={() => setConfirm("disable_auto")} disabled={busy}>{t("sync.disableAuto")}</Button>}
            </div>
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
              {plan.warnings.length > 0 && <div className="plan-warnings">{plan.warnings.map((warning) => <p key={warning}><AlertTriangle size={15} />{planWarning(warning)}</p>)}</div>}
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
              {planCapabilityChanges.length > 0 && (
                <section className="plan-resource-changes">
                  <header><div><p className="eyebrow">CAPABILITY DIFF</p><h3>{t("sync.exactChanges")}</h3></div><StatusBadge tone="warning">{planCapabilityChanges.length} {t("sync.capabilityChanges")}</StatusBadge></header>
                  <div>
                    {kindMeta.map(({ id: kind, icon: ChangeIcon }) => {
                      const changes = planCapabilityChanges.filter((change) => change.kind === kind);
                      if (!changes.length) return null;
                      return <section className="plan-resource-group" key={kind}><h4><ChangeIcon size={15} />{t(`kinds.${kind}`)}<span>{changes.length}</span></h4><div>{changes.map((change) => { const capability = inventory.find((item) => item.kind === kind && item.id === change.id); return <article key={`${kind}:${change.id}`}><span className={`action-mark action-mark--${change.action}`}>{t(`sync.${change.action === "replace" ? "update" : change.action}`)}</span><span><strong>{capability?.display_name ?? change.id}</strong>{capability && capability.display_name !== change.id && <code>{change.id}</code>}</span><small>{change.files} {t("sync.files")}</small></article>; })}</div></section>;
                    })}
                  </div>
                </section>
              )}
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
                <Button variant="danger" onClick={() => setConfirm("reviewed")}>{t("sync.apply")} <ArrowRight size={17} /></Button>
              </div>}
            </>
          )}
        </section>
      </section>
      <Dialog
        open={Boolean(confirm)}
        onClose={() => { if (!busy) setConfirm(null); }}
        title={t(confirm === "enable_auto" ? "sync.autoConfirmTitle" : confirm === "disable_auto" ? "sync.disableAutoTitle" : "sync.confirmTitle")}
        actions={<><Button variant="secondary" disabled={busy} onClick={() => setConfirm(null)}>{t("common.cancel")}</Button><Button variant={confirm === "disable_auto" || confirm === "reviewed" ? "danger" : "primary"} disabled={busy} onClick={confirm === "enable_auto" ? () => configureAutoSync(true) : confirm === "disable_auto" ? () => configureAutoSync(false) : apply}>{busy ? <RefreshCw className="spin" size={16} /> : <ShieldCheck size={16} />}{t(confirm === "enable_auto" ? "sync.autoConfirmAction" : confirm === "disable_auto" ? "sync.disableAutoAction" : "sync.apply")}</Button></>}
      >
        <p>{t(confirm === "enable_auto" ? "sync.autoConfirmBody" : confirm === "disable_auto" ? "sync.disableAutoBody" : "sync.confirmBody")}</p>
        <div className="rollback-target"><span className="target-tab__mark">{targetMeta.find((item) => item.id === target)?.mark}</span><span><strong>{t(`targets.${target}`)}</strong><small>{selectedCount} {t("sync.selected")}</small></span></div>
        {confirm === "enable_auto" && selection && <div className="auto-scope-review">{(["skill", "plugin", "mcp", "rule"] as Kind[]).map((kind) => { const ids = kind === "skill" ? selection.skills : kind === "plugin" ? selection.plugins : kind === "mcp" ? selection.mcp : selection.rule_ids ?? []; return <section key={kind}><strong>{t(`kinds.${kind}`)}</strong><span>{ids.length ? ids.map((id) => inventory.find((item) => item.kind === kind && item.id === id)?.display_name ?? id).join("、") : t("sync.noneSelected")}</span></section>; })}</div>}
        {confirm === "enable_auto" && selection?.mode === "replace" && <div className="auto-scope-review"><strong>{t("sync.delete")}</strong>{planCapabilityChanges.filter((item) => item.action === "delete").length ? planCapabilityChanges.filter((item) => item.action === "delete").map((item) => <section key={`${item.kind}:${item.id}`}><strong>{t(`kinds.${item.kind}`)}</strong><span>{item.id}</span></section>) : <p>{t("sync.noneSelected")}</p>}</div>}
        {confirm === "reviewed" && <code className="dialog-code">{plan?.id}</code>}
      </Dialog>
    </>
  );
}

function VersionChangeList({ changes, selected, onSelect, disabled = false }: { changes: CapabilityChange[]; selected?: string[]; onSelect?: (key: string) => void; disabled?: boolean }) {
  if (!changes.length) return <p className="commit-clean">{t("git.noDiff")}</p>;
  return <div className="version-capability-changes">{([...kindMeta.map((item) => item.id), "library"] as const).map((kind) => {
    const items = changes.filter((item) => (item.kind ?? "library") === kind);
    if (!items.length) return null;
    return <section key={kind}><h3>{kind === "library" ? t("git.libraryConfig") : t(`kinds.${kind}`)} <span className="count-badge">{items.length}</span></h3>{items.map((item) => <div key={item.id} className="version-change-entry">{onSelect && item.kind && <label className="version-selection"><input type="checkbox" disabled={disabled} checked={selected?.includes(`${item.kind}:${item.id}`) ?? false} onChange={() => onSelect(`${item.kind}:${item.id}`)} /><span>{t("git.selectCapability")} {item.id}</span></label>}<details><summary><ChevronRight size={15} /><span className={`action-mark action-mark--${item.action}`}>{t(`sync.${item.action}`)}</span><strong>{item.id}</strong><small>{item.files.length} {t("sync.files")}</small></summary><ul>{item.files.map((file) => <li key={file}><code>{file}</code></li>)}</ul></details></div>)}</section>;
  })}</div>;
}

function GitPage({ onCommitted, onRecovered }: { onCommitted: () => void; onRecovered: () => void }) {
  const [changes, setChanges] = useState<CapabilityChange[]>([]);
  const [selected, setSelected] = useState<string[]>([]);
  const [pushAfterSave, setPushAfterSave] = useState(false);
  const [projectAfterSave, setProjectAfterSave] = useState(false);
  const [savePreview, setSavePreview] = useState<SavePreview | null>(null);
  const [changesError, setChangesError] = useState("");
  const [recovery, setRecovery] = useState<VersionPreview | null>(null);
  const [confirmation, setConfirmation] = useState("");
  const [recoveryError, setRecoveryError] = useState("");
  const [recoveryNotice, setRecoveryNotice] = useState("");
  const [status, setStatus] = useState("");
  const [diff, setDiff] = useState("");
  const [history, setHistory] = useState("");
  const [remote, setRemote] = useState<RemoteSettings>({ branch: "main" });
  const [remoteUrl, setRemoteUrl] = useState("");
  const [remoteBranch, setRemoteBranch] = useState("main");
  const [remoteError, setRemoteError] = useState("");
  const [remoteNotice, setRemoteNotice] = useState("");
  const [loginOpen, setLoginOpen] = useState(false);
  const [loginPlatform, setLoginPlatform] = useState<"github" | "git">("git");
  const [loginUsername, setLoginUsername] = useState("");
  const [loginToken, setLoginToken] = useState("");

  const [loginError, setLoginError] = useState("");
  const [retryAfterLogin, setRetryAfterLogin] = useState(false);
  const openLogin = (retry = false) => {
    setLoginPlatform(/^https:\/\/github\.com\//i.test(remoteUrl) ? "github" : "git");
    setRetryAfterLogin(retry); setLoginError(""); setLoginToken("");  setLoginOpen(true);
  };
  const closeLogin = () => { if (!busy) { setLoginOpen(false); setLoginToken(""); setLoginError("");  } };
  const remoteAction = async (action: "connect" | "disconnect" | "sync" | "receive") => {
    setBusy(true); setRemoteError(""); setRemoteNotice("");
    try {
      if (action === "connect") { setRemote(await api.connectRemote(remoteUrl.trim(), remoteBranch.trim())); setRemoteNotice(t("git.connected")); }
      else if (action === "disconnect") { await api.disconnectRemote(); setRemote({ branch: remoteBranch }); }
      else if (action === "receive") { await api.receiveRemote(); await load(); onCommitted(); setRemoteNotice(t("git.received")); }
      else { await receiveAndRefresh(); }
    } catch (value) {
      setRemoteError(String(value));
      const next = await api.remoteSettings().catch(() => remote);
      setRemote(next);
      if (remoteUrl.startsWith("https://") && (next.state === "auth_failed" || /authentication|could not read (username|password)|401|403|access denied|repository not found/i.test(String(value)))) openLogin(action === "sync");
    }
    finally { setBusy(false); }
  };
  const [identity, setIdentity] = useState<GitIdentity>({});
  const [message, setMessage] = useState("");
  const [name, setName] = useState("");
  const [email, setEmail] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const load = useCallback(async () => {
    const [nextStatus, nextDiff, nextHistory, nextIdentity, nextRemote] = await Promise.all([api.gitStatus(), api.gitDiff(), api.gitLog(), api.gitIdentity(), api.remoteSettings().catch((value) => { setRemoteError(String(value)); return { branch: "main" } as RemoteSettings; })]);
    setRemote(nextRemote); setRemoteUrl(nextRemote.url ?? ""); setRemoteBranch(nextRemote.branch);
    setStatus(nextStatus); setDiff(nextDiff); setHistory(nextHistory); setIdentity(nextIdentity);
    setName(nextIdentity.name ?? ""); setEmail(nextIdentity.email ?? "");
    try { const pending = await api.gitChanges(); setChanges(pending); setSelected((previous) => previous.filter((key) => pending.some((item) => `${item.kind}:${item.id}` === key))); setChangesError(""); }
    catch (value) { setChanges([]); setChangesError(String(value)); }
  }, []);
  useEffect(() => { void load().catch((value) => setError(String(value))); }, [load]);
  const dirty = Boolean(diff.trim()) || status.split("\n").some((line) => /^[ MARC?D!U]{1,2}\s/.test(line));
  const receiveAndRefresh = async () => {
    const result = await api.syncRemote();
    setError(result?.auto_sync_error ?? result?.auto_sync?.filter((outcome) => outcome.error).map((outcome) => `${t(`targets.${outcome.target}`)}: ${outcome.error}`).join("\n") ?? "");
    await load(); onCommitted(); setRemoteNotice(t("git.synced"));
  };
  const commit = async (event: FormEvent) => {
    event.preventDefault(); setError("");
    if (!message.trim()) { setError(t("inventory.required")); return; }
    if ((!identity.name && !name.trim()) || (!identity.email && !/^\S+@\S+\.\S+$/.test(email))) { setError(t("git.identityHint")); return; }
    setBusy(true);
    try {
      if (!selected.length) { setError(t("git.chooseCapabilities")); return; }
      const only = selected.map((key) => { const [kind, id] = key.split(":"); return { kind: kind as Kind, id }; });
      setSavePreview(await api.versionSavePlan(message.trim(), { only, push: pushAfterSave, host_sync: projectAfterSave ? "enabled" : "none" }, identity.name ? undefined : name.trim(), identity.email ? undefined : email.trim()));
    } catch (value) { setError(String(value)); }
    finally { setBusy(false); }
  };
  const applySave = async () => {
    if (!savePreview || busy) return;
    setBusy(true); setError("");
    try {
      const result = await api.versionSaveApply(savePreview.id);
      setRemoteError(result.remote_error ? `${t("git.localSaved")} ${result.remote_error}` : "");
      setRemoteNotice(result.remote_synced ? t("git.synced") : t("git.localSaved"));
      setError(result.auto_sync_error ?? result.auto_sync?.filter((outcome) => outcome.error).map((outcome) => `${t(`targets.${outcome.target}`)}: ${outcome.error}`).join("\n") ?? "");
      setSavePreview(null); setMessage(""); setSelected([]); await load(); onCommitted();
    } catch (value) { setError(String(value)); }
    finally { setBusy(false); }
  };
  const login = async (event: FormEvent) => {
    event.preventDefault(); setLoginError("");
    if (!loginUsername.trim() || !loginToken.trim()) { setLoginError(t("git.loginRequired")); return; }
    if (loginPlatform === "github" && !/^https:\/\/github\.com\//i.test(remoteUrl)) { setLoginError(t("git.tokenHintGithub")); return; }
    setBusy(true);
    let verified = false;
    try {
      const next = await api.loginRemote(remoteUrl.trim(), remoteBranch.trim(), loginUsername.trim(), loginToken);
      verified = true; setRemote(next); setLoginOpen(false); setLoginToken("");  setRemoteError(""); setRemoteNotice(t("git.loginDone"));
      if (retryAfterLogin) { await receiveAndRefresh(); }
    } catch (value) {
      if (verified) { setRemoteError(String(value)); setRemote(await api.remoteSettings().catch(() => remote)); }
      else { setLoginError(String(value)); setRemote(await api.remoteSettings().catch(() => remote)); }
    } finally { setBusy(false); }
  };
  const previewRecovery = async (action: "discard" | "remote") => {
    setBusy(true); setError(""); setRecoveryError(""); setRecoveryNotice("");
    try { setRecovery(await api.versionRecoveryPlan(action)); setConfirmation(""); }
    catch (value) { setRecoveryError(String(value)); }
    finally { setBusy(false); }
  };
  const applyRecovery = async () => {
    if (!recovery || confirmation !== (recovery.action === "discard" ? "DISCARD" : "REMOTE")) return;
    setBusy(true); setRecoveryError("");
    try {
      const result = await api.versionRecoveryApply(recovery.id, confirmation);
      setRecovery(null); setConfirmation(""); setMessage(""); await load(); onRecovered();
      setRecoveryNotice(`${t("git.recoveryDone")} ${result.backup_path}${result.pending_changes ? ` · ${t("git.recoveryPending")}` : ""}`);
    } catch (value) { setRecoveryError(String(value)); }
    finally { setBusy(false); }
  };
  const forgetCredentials = async () => {
    setBusy(true); setRemoteError("");
    try { await api.forgetRemoteCredentials(); setRemote(await api.remoteSettings()); setRemoteNotice(t("git.forgotCredential")); }
    catch (value) { setRemoteError(String(value)); }
    finally { setBusy(false); }
  };
  const remoteStateLabel = remote.state === "synced" ? "git.syncedState" : remote.state === "read_verified" ? "git.connected" : remote.state === "auth_failed" ? "git.authFailed" : remote.state === "network_error" || remote.state === "sync_failed" ? "git.networkError" : "git.unverified";
  const remoteStateTone = remote.state === "synced" || remote.state === "read_verified" ? "ok" : "warning";
  const logEntries = history ? history.split("\n").map((line) => { const [sha, date, ...subject] = line.split("\t"); return { sha, date, subject: subject.join("\t") }; }) : [];
  return (
    <>
      <PageHeader title={t("git.title")} subtitle={t("git.subtitle")} />
      <section className="git-summary material">
        <span className="git-summary__icon"><GitBranch size={25} /></span>
        <div><p className="eyebrow">{t("git.branch")}</p><h2>{status.split("\n")[0]?.replace(/^## /, "") || t("git.title")}</h2><code>~/.agenthub/.git</code></div>
        <StatusBadge tone={dirty ? "warning" : "ok"}>{dirty ? t("overview.pending") : t("git.clean")}</StatusBadge>
      </section>
      <section className="material commit-card">
        <header className="version-changes-heading"><div><h2>{t("git.libraryChanges")}</h2><p>{t("git.recoveryScope")}</p></div><Button variant="secondary" disabled={busy} onClick={() => { setBusy(true); void load().catch((value) => setError(String(value))).finally(() => setBusy(false)); }}><RefreshCw size={16} />{t("git.refresh")}</Button></header>
        <div className="remote-actions"><Button variant="danger" disabled={busy || !dirty || !history} onClick={() => void previewRecovery("discard")}>{t("git.discard")}</Button><Button variant="secondary" disabled={busy || !remote.url} onClick={() => void previewRecovery("remote")}>{t("git.useRemote")}</Button></div>
        {!recovery && recoveryError && <p role="alert" className="inline-error">{recoveryError}</p>}
        {recoveryNotice && <p role="status" className="git-recovery-notice">{recoveryNotice}</p>}
        {changesError ? <p role="alert" className="inline-error">{changesError}</p> : <><div className="remote-actions"><Button variant="secondary" disabled={busy || !changes.length} onClick={() => setSelected(changes.filter((item) => item.kind).map((item) => `${item.kind}:${item.id}`))}>{t("common.selectAll")}</Button><Button variant="secondary" disabled={busy || !selected.length} onClick={() => setSelected([])}>{t("common.clearSelection")}</Button><span>{t("git.selectedCount")} {selected.length}</span></div><VersionChangeList changes={changes} selected={selected} disabled={busy} onSelect={(key) => setSelected((previous) => previous.includes(key) ? previous.filter((item) => item !== key) : [...previous, key])} /></>}
      </section>
      <Dialog wide open={Boolean(recovery)} title={t(recovery?.action === "discard" ? "git.discard" : "git.useRemote")} dismissible={!busy} onClose={() => { if (!busy) { setRecovery(null); setConfirmation(""); } }} actions={<><Button variant="secondary" disabled={busy} onClick={() => { setRecovery(null); setConfirmation(""); }}>{t("common.cancel")}</Button><Button variant="danger" disabled={busy || !recovery || confirmation !== (recovery.action === "discard" ? "DISCARD" : "REMOTE")} onClick={() => void applyRecovery()}>{busy ? <RefreshCw className="spin" size={16} /> : null}{t(recovery?.action === "discard" ? "git.discard" : "git.useRemote")}</Button></>}>
        <p>{t(recovery?.action === "discard" ? "git.discardBody" : "git.useRemoteBody")}</p>
        <code className="dialog-code">{recovery?.source_commit}</code>
        <VersionChangeList changes={recovery?.changes ?? []} />
        {recoveryError && <div className="inline-error" role="alert">{recoveryError}</div>}
        <label className="field"><span>{t("git.confirmRecovery")} {recovery?.action === "discard" ? "DISCARD" : "REMOTE"}</span><input value={confirmation} disabled={busy} autoComplete="off" onChange={(event) => setConfirmation(event.target.value)} /></label>
      </Dialog>
      <Dialog wide open={Boolean(savePreview)} title={t("git.reviewSave")} dismissible={!busy} onClose={() => { if (!busy) { setSavePreview(null); setError(""); } }} actions={<><Button variant="secondary" disabled={busy} onClick={() => { setSavePreview(null); setError(""); }}>{t("common.cancel")}</Button><Button disabled={busy} onClick={() => void applySave()}>{busy ? <RefreshCw className="spin" size={16} /> : null}{t("git.applySave")}</Button></>}>
        <p>{t("git.saveScope")}</p>
        <VersionChangeList changes={savePreview?.changes ?? []} />
        {savePreview?.file_review?.some((file) => file.credential_warning) && <p className="warning-banner" role="status">{t("git.credentialReview")}</p>}
        {savePreview?.file_review?.some((file) => file.binary) && <details><summary>{t("git.binaryFiles")}</summary><ul>{savePreview.file_review.filter((file) => file.binary).map((file) => <li key={file.path}><code>{file.path}</code> · {file.size} B · {file.digest.slice(0, 12)}</li>)}</ul></details>}
        <h3>{t("git.excludedChanges")}</h3><VersionChangeList changes={savePreview?.excluded_pending_changes ?? []} />
        <p>{t("git.localHead")} <code>{savePreview?.head ?? "—"}</code></p>
        <p>{savePreview?.options.push ? `${t("git.pushSelected")} · ${savePreview.remote_url} · ${savePreview.remote_branch}` : t("git.localOnly")}</p>
        {savePreview?.remote_head && <p>{t("git.remoteHead")} <code>{savePreview.remote_head}</code></p>}
        <h3>{t("git.toolProjection")}</h3>
        {savePreview?.host_plans.length ? savePreview.host_plans.map((plan) => <section key={plan.target}><h4>{t(`targets.${plan.target}`)}</h4><ul>{plan.resource_changes?.map((item) => <li key={`${item.kind}:${item.id}`}>{t(`kinds.${item.kind}`)} · {item.id} · {t(`sync.${item.action}`)}</li>)}</ul><details><summary>{t("sync.fileDetails")}</summary><ul>{plan.steps.map((step) => <li key={step.path}><span>{t(`sync.${step.action}`)}</span> <code>{step.path}</code></li>)}</ul></details></section>) : <p>{t("git.noToolWrites")}</p>}
        {error && <p className="inline-error" role="alert">{error}</p>}
      </Dialog>
      <section className="material commit-card">
        <header><span className="commit-card__icon"><Cloud size={20} /></span><div><h2>{t("git.remoteTitle")}</h2><p>{t("git.remoteHint")}</p></div></header>
        <p>{t("git.authHint")}</p>
        <form noValidate onSubmit={(event) => { event.preventDefault(); void remoteAction("connect"); }}>
          <div className="form-grid"><label className="field"><span>{t("git.remoteUrl")}</span><input value={remoteUrl} disabled={busy || Boolean(remote.url)} placeholder="https://github.com/user/agenthub.git" onChange={(event) => setRemoteUrl(event.target.value)} /></label><label className="field"><span>{t("git.remoteBranch")}</span><input value={remoteBranch} disabled={busy || Boolean(remote.url)} onChange={(event) => setRemoteBranch(event.target.value)} /></label></div>
          <div className="remote-actions">{remote.url ? <><StatusBadge tone={remoteStateTone}>{t(remoteStateLabel)}</StatusBadge><Button type="button" disabled={busy || !history} onClick={() => void remoteAction("sync")}><RefreshCw size={16} />{t("git.syncNow")}</Button><Button type="button" variant="secondary" disabled={busy || dirty || !history} onClick={() => void remoteAction("receive")}>{t("git.receiveOnly")}</Button><Button type="button" variant="secondary" disabled={busy} onClick={() => void remoteAction("disconnect")}>{t("git.disconnect")}</Button></> : <Button type="submit" disabled={busy || !remoteUrl.trim() || !remoteBranch.trim()}>{busy ? <RefreshCw className="spin" size={16} /> : <Cloud size={16} />}{t("git.connect")}</Button>}</div>
        </form>
        {remoteUrl.trim().startsWith("https://") && <div className="remote-auth-actions"><Button type="button" variant="secondary" disabled={busy || !remoteBranch.trim()} onClick={() => openLogin(remote.state === "auth_failed" && !dirty && Boolean(history))}><KeyRound size={16} />{t(remote.state === "auth_failed" && !dirty && Boolean(history) ? "git.loginRetry" : "git.login")}</Button>{remote.credential_saved && <><span className="remote-credential-label"><ShieldCheck size={15} />{t("git.savedCredential")}</span><Button type="button" variant="secondary" disabled={busy} onClick={() => void forgetCredentials()}>{t("git.forgetCredential")}</Button></>}</div>}
        <p>{t("git.remoteScope")}</p>
        {remoteNotice && <p role="status">{remoteNotice}</p>}
        {remoteError && <div className="inline-error" role="alert"><AlertTriangle size={17} /><div><p>{remoteError}</p><p>{t("git.retryHint")}</p></div></div>}
      </section>
      <Dialog open={loginOpen} title={t("git.loginTitle")} onClose={closeLogin} dismissible={!busy} actions={<><Button variant="secondary" disabled={busy} onClick={closeLogin}>{t("common.cancel")}</Button><Button type="submit" form="git-login-form" disabled={busy}>{busy ? <RefreshCw className="spin" size={16} /> : <KeyRound size={16} />}{busy ? t("git.signingIn") : retryAfterLogin ? t("git.loginRetry") : t("git.verifyLogin")}</Button></>}>
        <form id="git-login-form" className="git-login-form" noValidate onSubmit={(event) => void login(event)} aria-busy={busy}>
          <code className="git-login-repository">{remoteUrl}</code>
          {retryAfterLogin && <p>{t("git.retryLoginHint")}</p>}
          {loginError && <div id="git-login-error" className="inline-error" role="alert"><AlertTriangle size={17} /><span>{loginError}</span></div>}
          <RepositoryCredentials url={remoteUrl} busy={busy} platform={loginPlatform} onPlatform={setLoginPlatform} username={loginUsername} onUsername={setLoginUsername} token={loginToken} onToken={setLoginToken} onError={setLoginError} errorId={loginError ? "git-login-error" : undefined} />
        </form>
      </Dialog>
      <form className="material commit-card" noValidate onSubmit={commit}>
        <header><span className="commit-card__icon"><Save size={20} /></span><div><h2>{t("git.commitTitle")}</h2><p>{t("git.commitHint")}</p></div></header>
        {error && <div className="inline-error" role="alert"><Activity size={17} /><span>{error}</span></div>}
        <div className="commit-form-row">
          <label className="field"><span>{t("git.message")}</span><input value={message} placeholder={t("git.messagePlaceholder")} onChange={(event) => { setMessage(event.target.value); setError(""); }} /></label>
          <Button type="submit" disabled={busy || !dirty || !selected.length}>{busy ? <RefreshCw className="spin" size={16} /> : <GitBranch size={16} />}{busy ? t("git.committing") : t("git.reviewSave")}</Button>
        </div>
        <div className="remote-actions"><label className="version-selection"><input type="checkbox" disabled={busy || !remote.url} checked={pushAfterSave} onChange={(event) => setPushAfterSave(event.target.checked)} /><span>{t("git.pushSelected")}</span></label><label className="version-selection"><input type="checkbox" disabled={busy} checked={projectAfterSave} onChange={(event) => setProjectAfterSave(event.target.checked)} /><span>{t("git.projectSelected")}</span></label></div>
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
  const historyGroups = useMemo(() => {
    const groups: Array<{ key: string; quick: boolean; items: Transaction[] }> = [];
    for (const tx of transactions) {
      const quick = ((tx.mode ?? "reviewed") === "auto_sync" || tx.mode === "default_sync") && tx.status === "applied";
      const day = new Date(tx.created_at).toLocaleDateString();
      const key = quick ? `${tx.target}-${day}` : tx.id;
      const previous = groups.at(-1);
      if (quick && previous?.quick && previous.key === key) previous.items.push(tx);
      else groups.push({ key, quick, items: [tx] });
    }
    return groups;
  }, [transactions]);
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
  const transactionRow = (tx: Transaction) => (
    <article key={tx.id}>
      <span className={`history-timeline__point ${tx.status === "rollback_applied" ? "is-rollback" : ""}`}>{tx.status === "rollback_applied" ? <RotateCcw size={18} /> : <CheckCircle2 size={18} />}</span>
      <div className="history-timeline__copy">
        <span><strong>{t(`targets.${tx.target}`)}</strong><StatusBadge tone={tx.status === "applied" || tx.status === "rollback_applied" ? "ok" : "warning"}>{transactionStatus(tx.status)}</StatusBadge><StatusBadge>{transactionMode(tx.mode ?? "reviewed")}</StatusBadge></span>
        <code>{tx.id}</code>
        <time>{new Date(tx.created_at).toLocaleString()}</time>
        {(tx.status === "applied" || tx.status === "rollback_applied") && <small><ShieldCheck size={13} /> {tx.status === "rollback_applied" ? t("transactions.verifiedRollback") : t("transactions.verifiedApply")}</small>}
      </div>
      {(tx.status === "applied" || tx.status === "rollback_applied") && <Button variant="secondary" onClick={() => setRollbackCandidate(tx)}><RotateCcw size={15} />{tx.status === "applied" ? t("transactions.rollbackApply") : t("transactions.rollbackRollback")}</Button>}
    </article>
  );
  return (
    <>
      <PageHeader title={t("nav.transactions")} subtitle={t("transactions.subtitle")} />
      <section className="material history-card">
        {error && <div className="inline-error" role="alert"><Activity size={18} /><span>{error}</span></div>}
        {transactions.length ? (
          <div className="history-timeline">
            {historyGroups.map((group) => group.quick && group.items.length > 1 ? (
              <details className="history-batch" key={group.key}>
                <summary><span><Shuffle size={16} /><strong>{t(`targets.${group.items[0].target}`)} · {t("transactions.autoBatch")}</strong></span><span>{group.items.length} {t("transactions.runs")}<ChevronRight size={15} /></span></summary>
                <div>{group.items.map(transactionRow)}</div>
              </details>
            ) : transactionRow(group.items[0]))}
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

function SettingsPage({ data, onReset }: { data: Dashboard; onReset: (path: string) => void }) {
  const [resetOpen, setResetOpen] = useState(false);
  const [resetConfirmation, setResetConfirmation] = useState("");
  const [resetBusy, setResetBusy] = useState(false);
  const [resetError, setResetError] = useState("");
  const reset = async () => {
    setResetBusy(true); setResetError("");
    try { const path = await api.resetAgenthub(resetConfirmation); onReset(path); }
    catch (value) { setResetError(String(value)); }
    finally { setResetBusy(false); }
  };
  const [diagnostics, setDiagnostics] = useState<RuntimeDiagnostics | null>(null);
  const [policy, setPolicy] = useState<PolicySettings>({ strict_authoritative: false, sync_after_reverse_import: false });
  const [savingPolicy, setSavingPolicy] = useState(false);
  const [policyError, setPolicyError] = useState("");
  useEffect(() => { api.runtimeDiagnostics().then(setDiagnostics).catch(() => undefined); }, []);
  useEffect(() => { api.policySettings().then(setPolicy).catch((value) => setPolicyError(String(value))); }, []);
  const updatePolicy = async (next: PolicySettings) => {
    setSavingPolicy(true); setPolicyError("");
    try { setPolicy(await api.setPolicySettings(next)); }
    catch (value) { setPolicyError(String(value)); }
    finally { setSavingPolicy(false); }
  };
  const cards = [
    { icon: HardDrive, title: t("settings.canonical"), hint: t("settings.canonicalHint"), value: diagnostics?.canonical_root ?? "~/.agenthub", meta: diagnostics ? (diagnostics.git_available ? t("settings.gitReady") : t("settings.gitMissing")) : "Git", tone: diagnostics && !diagnostics.git_available ? "amber" : "blue" },
    { icon: Database, title: t("settings.database"), hint: t("settings.databaseHint"), value: "state/agenthub.db", meta: t("settings.localOnly"), tone: "purple" },
    { icon: Radio, title: t("settings.targets"), hint: t("settings.targetsHint"), value: `${data.enabled_targets.length} ${t("settings.enabled")}`, meta: t("settings.healthy"), tone: "green" },
    { icon: TerminalSquare, title: t("settings.logs"), hint: t("settings.logsHint"), value: diagnostics?.log_dir ?? t("settings.loadingPath"), meta: diagnostics?.platform ?? t("settings.localOnly"), tone: "blue" },
  ];
  return (
    <>
      <PageHeader title={t("nav.settings")} subtitle={t("settings.subtitle")} />
      <section className="material commit-card"><header><span className="commit-card__icon"><RefreshCw size={20} /></span><div><h2>{t("settings.resetTitle")}</h2><p>{t("settings.resetHint")}</p></div></header><Button variant="danger" onClick={() => { setResetConfirmation(""); setResetError(""); setResetOpen(true); }}>{t("settings.resetAction")}</Button></section>
      <Dialog open={resetOpen} onClose={() => !resetBusy && setResetOpen(false)} title={t("settings.resetTitle")} actions={<><Button variant="secondary" disabled={resetBusy} onClick={() => setResetOpen(false)}>{t("common.cancel")}</Button><Button variant="danger" disabled={resetBusy || resetConfirmation !== "AGENTHUB"} onClick={() => void reset()}>{resetBusy ? <RefreshCw className="spin" size={16} /> : <Trash2 size={16} />}{t("settings.resetAction")}</Button></>}><p>{t("settings.resetBody")}</p><p>{t("settings.resetBackup")}</p><label className="field"><span>{t("settings.resetType")}</span><input value={resetConfirmation} disabled={resetBusy} onChange={(event) => setResetConfirmation(event.target.value)} /></label>{resetError && <div className="inline-error" role="alert">{resetError}</div>}</Dialog>
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
        <div><h2>{t("settings.mcpCredentials")}</h2><p>XChaCha20-Poly1305 · <code>secrets/master.key</code></p></div>
        <StatusBadge tone="ok">{t("settings.localOnly")}</StatusBadge>
      </section>
      <section className="material policy-panel">
        <header><div><p className="eyebrow">AUTHORITY POLICY</p><h2>{t("settings.coverageTitle")}</h2><p>{t("settings.coverageHint")}</p></div><StatusBadge tone={policy.sync_mode === "replace" ? "warning" : "ok"}>{policy.sync_mode === "replace" ? t("sync.replaceMode") : t("sync.preserveMode")}</StatusBadge></header>
        {policyError && <div className="inline-error" role="alert"><Activity size={17} /><span>{policyError}</span></div>}
        <button type="button" className={`policy-toggle ${policy.sync_mode === "replace" ? "is-enabled" : ""}`} disabled={savingPolicy} onClick={() => void updatePolicy({ ...policy, strict_authoritative: false, sync_mode: policy.sync_mode === "replace" ? "preserve" : "replace" })}>
          <span><ShieldCheck size={20} /></span><span><strong>{t("sync.replaceMode")}</strong><small>{t("sync.replaceHint")}</small></span><i aria-hidden="true"><b /></i>
        </button>

        <div className="policy-guards"><span><CheckCircle2 size={15} />{t("settings.planGuard")}</span><span><CheckCircle2 size={15} />{t("settings.backupGuard")}</span><span><CheckCircle2 size={15} />{t("settings.vendorGuard")}</span></div>
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

function ScanWarning({ item }: { item: ScanItem }) {
  if (!item.warning) return null;
  return <em>{t(`init.${item.warning}`)}{item.warning_detail && <span className="scan-warning-detail">{item.warning_detail}</span>}</em>;
}

function ImportSourcePicker({ items, available, selected, source, onSource, onSelected, busy }: { items: ScanItem[]; available: ScanItem[]; selected: Set<string>; source: string; onSource: (source: string) => void; onSelected: Dispatch<SetStateAction<Set<string>>>; busy: boolean }) {
  const sources = ["agents", "cursor", "codex", "claude", ...new Set(items.map((item) => item.source).filter((id) => !["agents", "cursor", "codex", "claude"].includes(id)))];
  const chosen = items.filter((item) => selected.has(item.id));
  const unique = new Set(chosen.map((item) => `${item.kind}:${item.comparison_digest ?? item.digest}`)).size;
  const current = available.filter((item) => source === "all" || item.source === source);
  return <section className="import-sources" aria-label={t("init.sources")}>
    <div className="scan-kind-tabs scan-source-tabs" role="group" aria-label={t("init.sources")}>
      {["all", ...sources].map((id) => { const group = available.filter((item) => id === "all" || item.source === id); return <button type="button" aria-label={`${id === "all" ? t("init.allSources") : targetMeta.some((target) => target.id === id) ? t(`targets.${id}`) : id} ${group.filter((item) => selected.has(item.id)).length} / ${group.length}`} aria-pressed={source === id} className={source === id ? "is-selected" : ""} disabled={busy} key={id} onClick={() => onSource(id)}><span><strong>{id === "all" ? t("init.allSources") : targetMeta.some((target) => target.id === id) ? t(`targets.${id}`) : id}</strong><small>{group.filter((item) => selected.has(item.id)).length} / {group.length}</small></span></button>; })}
    </div>
    <p className="import-source-hint">{t("init.scannedCount")} <strong>{items.filter((item) => source === "all" || item.source === source).length}</strong> · {t("init.newCount")} <strong>{current.length}</strong>{items.every((item) => source !== "all" && item.source !== source) && <span> · {t("init.emptySource")}</span>}</p>
    <div className="scan-group-toolbar"><p role="status">{t("init.totalSelected")} <strong>{selected.size}</strong> · {t("init.uniqueSelected")} <strong>{unique}</strong><small>{t("init.selectionHint")}</small></p><Button variant="quiet" disabled={busy || !current.length} onClick={() => { onSelected((previous) => { const next = new Set(previous); const allSelected = current.every((item) => next.has(item.id)); current.forEach((item) => allSelected ? next.delete(item.id) : next.add(item.id)); return next; }); }}>{current.length && current.every((item) => selected.has(item.id)) ? t("init.clearSource") : t("init.selectSource")}</Button></div>
  </section>;
}

function Init({ onDone, recoveryPath }: { onDone: (count: number, restored?: boolean) => void; recoveryPath?: string }) {
  const [mode, setMode] = useState<"choose" | "local" | "remote">("choose");
  const [url, setUrl] = useState("");
  const [branch, setBranch] = useState("");
  const [auth, setAuth] = useState<"token" | "system">("token");
  const [platform, setPlatform] = useState<"github" | "git">("github");
  const [username, setUsername] = useState("");
  const [token, setToken] = useState("");
  const restoreForm = useRef<HTMLFormElement>(null);
  const [serverFailure, setServerFailure] = useState(false);
  const [items, setItems] = useState<ScanItem[] | null>(null);
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const [error, setError] = useState("");
  const [recover, setRecover] = useState(false);
  const [scanKind, setScanKind] = useState<Kind>("skill");
  const [scanSource, setScanSource] = useState("all");
  const importableItems = items?.filter((item) => item.importable) ?? [];
  const visibleScanItems = items?.filter((item) => item.kind === scanKind && (scanSource === "all" || item.source === scanSource)) ?? [];
  const visibleImportableItems = visibleScanItems.filter((item) => item.importable);
  const visibleSelectedCount = visibleImportableItems.filter((item) => selected.has(item.id)).length;
  useEffect(() => {
    if (mode === "remote" && serverFailure && error && !busy) document.getElementById("init-error")?.focus();
  }, [mode, serverFailure, error, busy]);

  const scan = async () => {
    if (pending.current) return;
    pending.current = true;
    setMode("local"); setBusy(true); setError("");
    try { setItems(await api.scan()); setSelected(new Set()); setScanSource("all"); } catch (value) { setError(String(value)); } finally { pending.current = false; setBusy(false); }
  };
  const finish = async () => {
    if (pending.current) return;
    pending.current = true;
    setBusy(true); setError("");
    try { const imported = await api.finishInit([...selected]); onDone(imported.length); } catch (value) {
      const message = String(value); setError(message); if (message.includes("incomplete Canonical")) setRecover(true);
    } finally { pending.current = false; setBusy(false); }
  };
  const back = () => {
    if (pending.current) return;
    setMode("choose"); setItems(null); setToken(""); setUsername(""); setSelected(new Set()); setError("");
  };
  const restore = async (event: FormEvent) => {
    event.preventDefault();
    if (pending.current) return;
    setServerFailure(false);
    const useToken = auth === "token" && url.trim().startsWith("https://");
    if (!url.trim() || (useToken && (!username.trim() || !token.trim()))) {
      setError(t("init.restoreRequired"));
      restoreForm.current?.querySelector<HTMLInputElement>(!url.trim() ? "input[name=repository]" : !username.trim() ? "input[autocomplete=username]" : "input[autocomplete=off]")?.focus();
      return;
    }
    if (useToken && platform === "github" && !/^https:\/\/github\.com\//i.test(url.trim())) { setError(t("git.tokenHintGithub")); return; }
    pending.current = true; setBusy(true); setError("");
    try {
      const result = await api.restoreLibrary(url.trim(), branch.trim(), useToken ? username.trim() : undefined, useToken ? token : undefined);
      setToken(""); onDone(result.imported, true);
    } catch (value) { setServerFailure(true); setError(String(value)); }
    finally { pending.current = false; setBusy(false); }
  };
  const discard = async () => {
    if (pending.current) return;
    pending.current = true;
    setBusy(true); setError("");
    try { await api.discardIncompleteInit(); setRecover(false); pending.current = false; await scan(); } catch (value) { setError(String(value)); }
    finally { pending.current = false; setBusy(false); }
  };

  return (
    <main className="init-shell">
      <section className="init-panel">
        <header className="init-topbar">
          <span className="brand__mark"><img src={agentHubLogo} alt="" /></span>
          <span><strong>AgentHub</strong><small>FIRST RUN</small></span>
          <span className="init-step">{mode === "choose" || (mode === "local" && items === null) ? "01" : "02"} / 02</span>
        </header>
        <PageHeader eyebrow="LIBRARY SETUP" title={t(mode === "remote" ? "init.restoreTitle" : mode === "choose" ? "init.welcome" : "init.title")} subtitle={t(mode === "remote" ? "init.restoreHint" : mode === "choose" ? "init.chooseHint" : "init.subtitle")} />
        {recoveryPath && <div className="warning-banner" role="status"><ShieldCheck size={17} /><span>{t("settings.recoveryLocation")} <code>{recoveryPath}</code></span></div>}
        <div className="init-progress" aria-hidden="true"><i className={mode === "remote" || items !== null ? "is-done" : "is-current"} /><i className={mode === "remote" || items !== null ? "is-current" : ""} /></div>

        {error && mode !== "remote" && (
          <div id="init-error" className="inline-error" role="alert">
            <Activity size={18} /><span><strong>{t("toast.failed")}</strong>{error}</span>
            {error.includes("incomplete Canonical") && <Button variant="danger" onClick={() => setRecover(true)}>{t("init.recover")}</Button>}
          </div>
        )}

        {mode === "remote" ? <form ref={restoreForm} className="git-login-form init-restore-form" noValidate onSubmit={(event) => void restore(event)} aria-busy={busy}>
          <label className="field"><span>{t("git.remoteUrl")}</span><input name="repository" autoFocus value={url} disabled={busy} placeholder="https://github.com/user/agenthub.git" aria-invalid={Boolean(error)} aria-describedby={error ? "init-error" : "restore-scope"} onChange={(event) => { const value = event.target.value; setUrl(value); setToken(""); setPlatform(/^https:\/\/github\.com\//i.test(value.trim()) ? "github" : "git"); setError(""); }} /></label>
          <label className="field"><span>{t("git.remoteBranch")}</span><input value={branch} disabled={busy} placeholder={t("init.branchAuto")} aria-describedby="restore-branch-hint" onChange={(event) => setBranch(event.target.value)} /></label>
          <p id="restore-branch-hint">{t("init.branchHint")}</p>
          {url.trim().startsWith("https://") && <fieldset className="git-login-platform" disabled={busy}><legend>{t("init.authentication")}</legend><div><Button variant={auth === "token" ? "primary" : "secondary"} aria-pressed={auth === "token"} onClick={() => setAuth("token")}>{t("git.token")}</Button><Button variant={auth === "system" ? "primary" : "secondary"} aria-pressed={auth === "system"} onClick={() => { setAuth("system"); setToken(""); }}>{t("init.systemAuth")}</Button></div></fieldset>}
          {url.trim().startsWith("https://") && auth === "token" ? <RepositoryCredentials url={url} busy={busy} platform={platform} onPlatform={setPlatform} username={username} onUsername={setUsername} token={token} onToken={setToken} onError={setError} errorId={error ? "init-error" : undefined} /> : <p>{t("git.authHint")}</p>}
          <p id="restore-scope">{t("init.restoreScope")}</p>
          {error && <div id="init-error" className="inline-error" role="alert" tabIndex={-1}><Activity size={18} /><span><strong>{t("toast.failed")}</strong>{error}</span></div>}
          {busy && <p role="status">{t("init.restoring")}</p>}
          <div className="remote-actions"><Button variant="secondary" disabled={busy} onClick={back}>{t("init.back")}</Button><Button type="submit" disabled={busy}>{busy ? <RefreshCw className="spin" size={17} /> : <Cloud size={17} />}{t("init.restoreAction")}</Button></div>
        </form> : mode === "choose" ? <div className="init-choices">
          <article><Sparkles size={26} /><h2>{t("init.newLibrary")}</h2><p>{t("init.newHint")}</p><Button disabled={busy} onClick={() => void scan()}><ArrowRight size={17} />{t("init.scan")}</Button><Button variant="quiet" disabled={busy} onClick={() => void finish()}>{t("init.emptyLibrary")}</Button></article>
          <article><Cloud size={26} /><h2>{t("init.existingLibrary")}</h2><p>{t("init.existingHint")}</p><Button variant="secondary" disabled={busy} onClick={() => { setMode("remote"); setError(""); }}>{t("init.restoreTitle")}</Button></article>
        </div> : items === null ? (
          <div className="init-intro">
            <div className="init-orbit" aria-hidden="true"><span><Sparkles /></span><i /><i /><i /></div>
            <div><h2>{t("init.scan")}</h2><p>{t("init.subtitle")}</p></div>
            <Button disabled={busy} onClick={scan}>{busy ? <RefreshCw className="spin" size={17} /> : <ArrowRight size={17} />} {busy ? t("common.loading") : t("init.scan")}</Button>
            <Button variant="quiet" disabled={busy} onClick={back}>{t("init.back")}</Button>
          </div>
        ) : (
          <section className="scan-results">
            <header className="scan-toolbar">
              <div><h2>{t("init.found")}</h2><small>{selected.size} {t("init.selected")} · {importableItems.length} {t("init.importable")} · {items.length - importableItems.length} {t("init.rejected")}</small></div>
              <div><Button variant="quiet" disabled={busy || selected.size === importableItems.length} onClick={() => setSelected(new Set(importableItems.map((item) => item.id)))}>{t("init.selectAll")}</Button><Button variant="quiet" disabled={busy || selected.size === 0} onClick={() => setSelected(new Set())}>{t("init.clear")}</Button></div>
            </header>
            <ImportSourcePicker items={items} available={importableItems} selected={selected} source={scanSource} onSource={setScanSource} onSelected={setSelected} busy={busy} />
            <div className="scan-kind-tabs" role="tablist" aria-label={t("init.groups")}>
              {kindMeta.map(({ id, icon: KindIcon }) => {
                const group = items.filter((item) => item.kind === id && (scanSource === "all" || item.source === scanSource));
                const groupSelected = group.filter((item) => item.importable && selected.has(item.id)).length;
                return (
                  <button type="button" role="tab" aria-selected={scanKind === id} className={scanKind === id ? "is-selected" : ""} disabled={busy} onClick={() => setScanKind(id)} key={id}>
                    <KindIcon size={17} /><span><strong>{t(`kinds.${id}`)}</strong><small>{groupSelected} / {group.filter((item) => item.importable).length}</small></span>
                  </button>
                );
              })}
            </div>
            <div className="scan-group-toolbar">
              <span>{t(`kinds.${scanKind}`)} · {visibleScanItems.length} {t("init.discovered")}</span>
              <Button variant="quiet" disabled={busy || visibleImportableItems.length === 0} onClick={() => setSelected((current) => { const next = new Set(current); if (visibleImportableItems.every((item) => current.has(item.id))) visibleImportableItems.forEach((item) => next.delete(item.id)); else visibleImportableItems.forEach((item) => next.add(item.id)); return next; })}>{visibleSelectedCount === visibleImportableItems.length && visibleImportableItems.length > 0 ? t("init.clearGroup") : t("init.selectGroup")}</Button>
            </div>
            <div className="scan-list">
              {!items.length && <EmptyState icon={Boxes} title={t("inventory.empty")} body={t("init.none")} />}
              {items.length > 0 && visibleScanItems.length === 0 && <EmptyState icon={kindMeta.find((kind) => kind.id === scanKind)?.icon ?? Boxes} title={t("init.emptyGroup")} body={t("init.emptyGroupHint")} compact />}
              {visibleScanItems.map((item) => {
                const checked = selected.has(item.id);
                const displayName = item.source_key?.replace(/^(server|rule):/, "") ?? item.path.split(/[\\/]/).pop();
                const duplicate = items.some((candidate) => candidate.id !== item.id && candidate.kind === item.kind && (candidate.comparison_digest ?? candidate.digest) === (item.comparison_digest ?? item.digest));
                return (
                  <label className={`${checked ? "is-selected" : ""} ${!item.importable ? "is-disabled" : ""}`} key={item.id}>
                    <input type="checkbox" disabled={busy || !item.importable} checked={checked} onChange={() => setSelected((old) => { const next = new Set(old); if (next.has(item.id)) next.delete(item.id); else next.add(item.id); return next; })} />
                    <span className="scan-check">{item.importable ? <CheckCircle2 size={17} /> : <AlertTriangle size={15} />}</span>
                    <span className="scan-copy"><strong title={displayName}>{displayName}</strong><small><b>{item.source}</b><span>{item.path}</span></small><ScanWarning item={item} />{duplicate && !item.warning && <em>{t("init.duplicate")}</em>}</span>
                    <code>{item.importable ? item.digest.slice(0, 8) : t("init.notImportable")}</code>
                  </label>
                );
              })}
            </div>
            <footer className="scan-footer"><Button variant="quiet" disabled={busy} onClick={back}>{t("init.back")}</Button><span>{selected.size} / {importableItems.length}</span><Button disabled={busy} onClick={finish}>{busy ? <RefreshCw className="spin" size={17} /> : <ArrowRight size={17} />} {busy ? t("init.importing") : t("init.import")}</Button></footer>
          </section>
        )}
      </section>
      <Dialog open={recover} onClose={() => !busy && setRecover(false)} title={t("init.recoverTitle")} actions={<><Button variant="secondary" disabled={busy} onClick={() => setRecover(false)}>{t("common.cancel")}</Button><Button variant="danger" disabled={busy} onClick={discard}>{t("init.recoverAction")}</Button></>}><p>{t("init.recoverBody")}</p></Dialog>
    </main>
  );
}

function SyncModePicker({ value, disabled, onChange }: { value: "preserve" | "replace"; disabled?: boolean; onChange: (value: "preserve" | "replace") => void }) {
  return <fieldset className="sync-mode-picker"><legend>{t("sync.modeTitle")}</legend>{(["preserve", "replace"] as const).map((mode) => <label key={mode}><input type="radio" name="sync-mode" checked={value === mode} disabled={disabled} onChange={() => onChange(mode)} /><span><strong>{t(`sync.${mode}Mode`)}</strong><small>{t(`sync.${mode}Hint`)}</small></span></label>)}</fieldset>;
}
