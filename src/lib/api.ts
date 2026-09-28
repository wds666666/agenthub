import { invoke } from "@tauri-apps/api/core";
export type Target = "cursor" | "codex" | "claude";
export type Kind = "skill" | "mcp" | "plugin" | "rule";
export interface Capability { id: string; kind: Kind; display_name: string; digest: string; path: string; compatible_targets: Target[] }
export interface CapabilityDetail { capability: Capability; preview_path: string; preview: string; preview_truncated: boolean; files: Array<{ path: string; size: number }> }
export interface Transaction { id: string; plan_id: string; target: Target; status: string; backup_path: string; verification?: string; created_at: string }
export interface Dashboard { initialized: boolean; inventory: Record<string, number>; enabled_targets: Target[]; dirty: boolean; recent_transactions: Transaction[] }
export interface ScanItem { id: string; kind: Kind; source: string; path: string; digest: string; selected: boolean; warning?: string }
export interface PlanStep { action: string; capability_kind?: Kind; capability_id?: string; path: string; detail: string }
export interface PlanCapabilitySummary { kind: Kind; affected: number; create: number; update: number; delete: number; skip: number; files: number }
export interface Plan { id: string; target: Target; steps: PlanStep[]; summary: PlanCapabilitySummary[]; warnings: string[]; canonical_digest: string; git: { head?: string; dirty: boolean } }
export interface RuleDocument { schemaVersion: number; id: string; displayName: string; activation: "always" | "manual" | "paths"; paths: string[]; targets: Target[]; body: string }
export interface GitIdentity { name?: string; email?: string }
const isTauri = () => "__TAURI_INTERNALS__" in window;
const demo: Dashboard = { initialized: true, inventory: { skill: 4, mcp: 2, plugin: 1, rule: 3 }, enabled_targets: ["codex"], dirty: true, recent_transactions: [] };
async function call<T>(name: string, args?: Record<string, unknown>): Promise<T> { if (!isTauri()) { if (name === "dashboard") return demo as T; if (name === "inventory" || name === "transaction_history") return [] as T; if (name === "git_status") return "## main\n M rules/safety/rule.md" as T; if (name === "git_diff") return "Canonical diff is available in the desktop app." as T; if (name === "git_log") return "a1b2c3d\t2026-09-27 18:30:00 +0800\tInitialize Canonical" as T; if (name === "git_identity") return { name: "AgentHub User", email: "user@example.com" } as T; throw new Error("This action requires the AgentHub desktop runtime."); } return invoke<T>(name, args); }
export const api = {
  dashboard: () => call<Dashboard>("dashboard"),
  inventory: () => call<Capability[]>("inventory"),
  capabilityDetail: (kind: Kind, id: string) => call<CapabilityDetail>("capability_detail", { kind, id }),
  readRule: (id: string) => call<RuleDocument>("read_rule", { id }),
  saveRule: (rule: RuleDocument, create: boolean) => call<Capability>("save_rule", { rule, create }),
  scan: () => call<ScanItem[]>("initial_scan"),
  finishInit: (selectedIds: string[]) => call<string[]>("finish_init", { selectedIds }),
  discardIncompleteInit: () => call<void>("discard_incomplete_init"),
  setTarget: (target: Target, enabled: boolean) => call<void>("set_target", { target, enabled }),
  plan: (target: Target) => call<Plan>("create_plan", { target }),
  apply: (planId: string) => call<Transaction>("apply_plan", { planId }),
  transactionHistory: (limit = 100) => call<Transaction[]>("transaction_history", { limit }),
  rollbackTransaction: (transactionId: string) => call<Transaction>("rollback_transaction", { transactionId }),
  gitStatus: () => call<string>("git_status"),
  gitDiff: () => call<string>("git_diff"),
  gitIdentity: () => call<GitIdentity>("git_identity"),
  gitLog: () => call<string>("git_log"),
  gitCommit: (message: string, name?: string, email?: string) => call<string>("git_commit", { message, name, email }),
  debugEvent: (event: string, context?: string) => {
    if (!isTauri()) { console.debug(`[AgentHub] ${event}`, context ?? ""); return Promise.resolve(); }
    return invoke<void>("debug_event", { event, context });
  },
};
