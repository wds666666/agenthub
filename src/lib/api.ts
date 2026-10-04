import { invoke } from "@tauri-apps/api/core";
export type Target = "agents" | "cursor" | "codex" | "claude";
export type Kind = "skill" | "mcp" | "plugin" | "rule";
export interface Capability { id: string; kind: Kind; display_name: string; digest: string; path: string; compatible_targets: Target[] }
export interface CapabilityDetail { capability: Capability; preview_path: string; preview: string; preview_truncated: boolean; files: Array<{ path: string; size: number }> }
export type TransactionMode = "reviewed" | "auto_sync" | "default_sync" | "rollback";
export interface Transaction { id: string; plan_id: string; target: Target; status: string; mode: TransactionMode; backup_path: string; verification?: string; created_at: string }
export interface SyncRunResult { changed: boolean; plan: Plan; transaction?: Transaction }
export interface Dashboard { initialized: boolean; inventory: Record<string, number>; enabled_targets: Target[]; auto_sync_targets: Target[]; dirty: boolean; recent_transactions: Transaction[] }
export interface ScanItem { id: string; kind: Kind; source: string; path: string; digest: string; selected: boolean; importable: boolean; source_key?: string; warning?: string; warning_detail?: string }
export type HostRelation = "canonical_match" | "host_only" | "constraint";
export interface HostResource { id: string; target: Target; kind: Kind; display_name: string; path: string; digest: string; relation: HostRelation; canonical_id?: string; deletable: boolean; constraint?: string }
export interface HostCleanupResult { id: string; deleted: string[]; backup_path: string }
export interface RuntimeDiagnostics { log_dir: string; canonical_root: string; git_available: boolean; platform: string }
export interface PlanStep { action: string; capability_kind?: Kind; capability_id?: string; path: string; detail: string }
export interface PlanCapabilitySummary { kind: Kind; affected: number; create: number; update: number; delete: number; skip: number; files: number }
export interface SyncSelection { skills_managed: boolean; skills: string[]; plugins_managed: boolean; plugins: string[]; mcp_managed: boolean; mcp: string[]; rules_managed: boolean; rules: boolean; authoritative: boolean }
export interface AutoSyncProfile { target: Target; enabled: boolean; selection: SyncSelection }
export interface AutoSyncOutcome { target: Target; changed: boolean; transaction_id?: string; error?: string }
export interface AutoSyncUpdateResult { profile: AutoSyncProfile; initial_sync?: SyncRunResult }
export interface CapabilityMutationResult { capability: Capability; auto_sync: AutoSyncOutcome[] }
export interface CapabilityKey { kind: Kind; id: string }
export interface CapabilityBatchDeleteResult { deleted: CapabilityKey[]; backup_path: string; auto_sync: AutoSyncOutcome[]; auto_sync_error?: string }
export interface CapabilityDeleteResult { id: string; kind: Kind; auto_sync: AutoSyncOutcome[] }
export interface ScanImportResult { imported: string[]; skipped_duplicates: number; auto_sync: AutoSyncOutcome[] }
export interface PolicySettings { strict_authoritative: boolean; sync_after_reverse_import: boolean }
export interface Plan { id: string; target: Target; steps: PlanStep[]; summary: PlanCapabilitySummary[]; selection?: SyncSelection; warnings: string[]; canonical_digest: string; git: { head?: string; dirty: boolean } }
export interface RuleDocument { schemaVersion: number; id: string; displayName: string; activation: "always" | "manual" | "paths"; paths: string[]; targets: Target[]; body: string }
export interface RemoteSettings { url?: string; branch: string; state?: "disconnected" | "unverified" | "read_verified" | "synced" | "auth_failed" | "network_error" | "sync_failed"; credential_saved?: boolean }
export interface CommitResult { local_saved: boolean; remote_synced: boolean; remote_error?: string }
export interface GitIdentity { name?: string; email?: string }
const isTauri = () => "__TAURI_INTERNALS__" in window;
const demo: Dashboard = { initialized: true, inventory: { skill: 4, mcp: 2, plugin: 1, rule: 3 }, enabled_targets: ["codex"], auto_sync_targets: ["codex"], dirty: true, recent_transactions: [] };
async function call<T>(name: string, args?: Record<string, unknown>): Promise<T> { if (!isTauri()) { if (name === "dashboard") return demo as T; if (name === "runtime_diagnostics") return { log_dir: "~/.local/share/dev.agenthub.desktop/logs", canonical_root: "~/.agenthub", git_available: true, platform: "linux" } as T; if (name === "inventory" || name === "transaction_history" || name === "auto_sync_profiles") return [] as T; if (name === "git_status") return "## main\n M rules/safety/rule.md" as T; if (name === "git_diff") return "Canonical diff is available in the desktop app." as T; if (name === "git_log") return "a1b2c3d\t2026-09-27 18:30:00 +0800\tInitialize Canonical" as T; if (name === "remote_settings") return { branch: "main" } as T; if (name === "git_identity") return { name: "AgentHub User", email: "user@example.com" } as T; throw new Error("This action requires the AgentHub desktop runtime."); } return invoke<T>(name, args); }
export const api = {
  dashboard: () => call<Dashboard>("dashboard"),
  runtimeDiagnostics: () => call<RuntimeDiagnostics>("runtime_diagnostics"),
  inventory: () => call<Capability[]>("inventory"),
  capabilityDetail: (kind: Kind, id: string) => call<CapabilityDetail>("capability_detail", { kind, id }),
  readRule: (id: string) => call<RuleDocument>("read_rule", { id }),
  saveRule: (rule: RuleDocument, create: boolean) => call<CapabilityMutationResult>("save_rule", { rule, create }),
  deleteCapability: (kind: Kind, id: string) => call<CapabilityDeleteResult>("delete_capability", { kind, id }),
  deleteCapabilities: (selected: CapabilityKey[]) => call<CapabilityBatchDeleteResult>("delete_capabilities", { selected }),
  scan: () => call<ScanItem[]>("initial_scan"),
  hostInventory: (target: Target) => call<HostResource[]>("host_inventory", { target }),
  cleanupHostResources: (target: Target, selectedIds: string[]) => call<HostCleanupResult>("cleanup_host_resources", { target, selectedIds }),
  importScanned: (selectedIds: string[]) => call<ScanImportResult>("import_scanned", { selectedIds }),
  finishInit: (selectedIds: string[]) => call<string[]>("finish_init", { selectedIds }),
  discardIncompleteInit: () => call<void>("discard_incomplete_init"),
  resetFailedInitialization: () => call<void>("reset_failed_initialization"),
  setTarget: (target: Target, enabled: boolean) => call<void>("set_target", { target, enabled }),
  plan: (target: Target, selection?: SyncSelection) => call<Plan>("create_plan", { target, selection }),
  apply: (planId: string) => call<Transaction>("apply_plan", { planId }),
  autoSyncProfiles: () => call<AutoSyncProfile[]>("auto_sync_profiles"),
  policySettings: () => call<PolicySettings>("policy_settings"),
  setPolicySettings: (settings: PolicySettings) => call<PolicySettings>("set_policy_settings", { settings }),
  setAutoSync: (target: Target, selection: SyncSelection, enabled: boolean) => call<AutoSyncUpdateResult>("set_auto_sync", { target, selection, enabled }),
  transactionHistory: (limit = 100) => call<Transaction[]>("transaction_history", { limit }),
  rollbackTransaction: (transactionId: string) => call<Transaction>("rollback_transaction", { transactionId }),
  resetAgenthub: (confirmation: string) => call<string>("reset_agenthub", { confirmation }),
  remoteSettings: () => call<RemoteSettings>("remote_settings"),
  connectRemote: (url: string, branch: string) => call<RemoteSettings>("connect_remote", { url, branch }),
  loginRemote: (url: string, branch: string, username: string, token: string) => call<RemoteSettings>("login_remote", { url, branch, username, token }),
  forgetRemoteCredentials: () => call<void>("forget_remote_credentials"),
  openTokenSettings: (url: string, platform: "github" | "git") => call<void>("open_token_settings", { url, platform }),
  disconnectRemote: () => call<void>("disconnect_remote"),
  syncRemote: () => call<void>("sync_remote"),
  gitStatus: () => call<string>("git_status"),
  gitDiff: () => call<string>("git_diff"),
  gitIdentity: () => call<GitIdentity>("git_identity"),
  gitLog: () => call<string>("git_log"),
  gitCommit: (message: string, name?: string, email?: string) => call<CommitResult>("git_commit", { message, name, email }),
  debugEvent: (event: string, context?: string) => {
    if (!isTauri()) { console.debug(`[AgentHub] ${event}`, context ?? ""); return Promise.resolve(); }
    return invoke<void>("debug_event", { event, context });
  },
};
