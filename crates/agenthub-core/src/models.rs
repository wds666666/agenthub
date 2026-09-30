use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum CapabilityKind {
    Skill,
    Mcp,
    Plugin,
    Rule,
}

impl CapabilityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Skill => "skill",
            Self::Mcp => "mcp",
            Self::Plugin => "plugin",
            Self::Rule => "rule",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Agents,
    Cursor,
    Codex,
    Claude,
}

impl Target {
    pub const ALL: [Target; 4] = [
        Target::Agents,
        Target::Cursor,
        Target::Codex,
        Target::Claude,
    ];
    pub const HOSTS: [Target; 3] = [Target::Cursor, Target::Codex, Target::Claude];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

impl std::str::FromStr for Target {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "agents" => Ok(Self::Agents),
            "cursor" => Ok(Self::Cursor),
            "codex" => Ok(Self::Codex),
            "claude" => Ok(Self::Claude),
            _ => anyhow::bail!("unknown target: {value}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capability {
    pub id: String,
    pub kind: CapabilityKind,
    pub display_name: String,
    pub digest: String,
    pub path: PathBuf,
    pub compatible_targets: Vec<Target>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityFile {
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDetail {
    pub capability: Capability,
    pub preview_path: String,
    pub preview: String,
    pub preview_truncated: bool,
    pub files: Vec<CapabilityFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleDocument {
    #[serde(rename = "schemaVersion", default = "schema_one")]
    pub schema_version: u32,
    pub id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub activation: String,
    #[serde(default)]
    pub paths: Vec<String>,
    pub targets: Vec<Target>,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitIdentity {
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanItem {
    pub id: String,
    pub kind: CapabilityKind,
    pub source: String,
    pub path: PathBuf,
    pub digest: String,
    pub selected: bool,
    #[serde(default = "default_true")]
    pub importable: bool,
    #[serde(default)]
    pub source_key: Option<String>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HostResourceRelation {
    CanonicalMatch,
    HostOnly,
    Constraint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostResource {
    pub id: String,
    pub target: Target,
    pub kind: CapabilityKind,
    pub display_name: String,
    pub path: PathBuf,
    pub digest: String,
    pub relation: HostResourceRelation,
    pub canonical_id: Option<String>,
    pub deletable: bool,
    pub constraint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostCleanupResult {
    pub id: String,
    pub deleted: Vec<String>,
    pub backup_path: PathBuf,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanAction {
    Create,
    Update,
    Replace,
    Delete,
    Skip,
    Constraint,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanStep {
    pub action: PlanAction,
    pub capability_kind: Option<CapabilityKind>,
    pub capability_id: Option<String>,
    pub path: PathBuf,
    pub current_digest: Option<String>,
    pub desired_digest: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanCapabilitySummary {
    pub kind: CapabilityKind,
    pub affected: usize,
    pub create: usize,
    pub update: usize,
    pub delete: usize,
    pub skip: usize,
    pub files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncSelection {
    #[serde(default)]
    pub skills_managed: bool,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub plugins_managed: bool,
    #[serde(default)]
    pub plugins: Vec<String>,
    #[serde(default)]
    pub mcp_managed: bool,
    #[serde(default)]
    pub mcp: Vec<String>,
    #[serde(default)]
    pub rules_managed: bool,
    #[serde(default)]
    pub rules: bool,
    #[serde(default)]
    pub authoritative: bool,
}

impl SyncSelection {
    pub fn manages_skills(&self) -> bool {
        self.authoritative || self.skills_managed || !self.skills.is_empty()
    }
    pub fn manages_plugins(&self) -> bool {
        self.authoritative || self.plugins_managed || !self.plugins.is_empty()
    }
    pub fn manages_mcp(&self) -> bool {
        self.authoritative || self.mcp_managed || !self.mcp.is_empty()
    }
    pub fn manages_rules(&self) -> bool {
        self.authoritative || self.rules_managed || self.rules
    }
}
impl Default for PlanCapabilitySummary {
    fn default() -> Self {
        Self {
            kind: CapabilityKind::Skill,
            affected: 0,
            create: 0,
            update: 0,
            delete: 0,
            skip: 0,
            files: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GitSnapshot {
    pub head: Option<String>,
    pub dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Plan {
    pub id: String,
    pub target: Target,
    pub canonical_digest: String,
    pub expected_digest: String,
    pub git: GitSnapshot,
    pub steps: Vec<PlanStep>,
    #[serde(default)]
    pub selection: Option<SyncSelection>,
    #[serde(default)]
    pub summary: Vec<PlanCapabilitySummary>,
    pub warnings: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum TransactionMode {
    #[default]
    Reviewed,
    #[serde(rename = "auto_sync", alias = "default_sync")]
    AutoSync,
    Rollback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub id: String,
    pub plan_id: String,
    pub target: Target,
    pub status: String,
    #[serde(default)]
    pub mode: TransactionMode,
    pub backup_path: PathBuf,
    pub git: GitSnapshot,
    pub canonical_digest: String,
    pub verification: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncRunResult {
    pub changed: bool,
    pub plan: Plan,
    pub transaction: Option<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoSyncProfile {
    pub target: Target,
    pub enabled: bool,
    pub selection: SyncSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoSyncOutcome {
    pub target: Target,
    pub changed: bool,
    pub transaction_id: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutoSyncUpdateResult {
    pub profile: AutoSyncProfile,
    pub initial_sync: Option<SyncRunResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityMutationResult {
    pub capability: Capability,
    pub auto_sync: Vec<AutoSyncOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityDeleteResult {
    pub id: String,
    pub kind: CapabilityKind,
    pub auto_sync: Vec<AutoSyncOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanImportResult {
    pub imported: Vec<String>,
    pub skipped_duplicates: usize,
    pub auto_sync: Vec<AutoSyncOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PolicySettings {
    #[serde(default)]
    pub strict_authoritative: bool,
    #[serde(default)]
    pub sync_after_reverse_import: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dashboard {
    pub initialized: bool,
    pub inventory: BTreeMap<String, usize>,
    pub enabled_targets: Vec<Target>,
    #[serde(default)]
    pub auto_sync_targets: Vec<Target>,
    pub dirty: bool,
    pub recent_transactions: Vec<Transaction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    #[serde(rename = "schemaVersion", default = "schema_one")]
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}
fn schema_one() -> u32 {
    1
}
