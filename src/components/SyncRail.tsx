import { Box, Braces, Check, Circle, Code2, FileText, PlugZap, Radio, Sparkles } from "lucide-react";
import type { CSSProperties } from "react";
import type { Dashboard, Kind, Target } from "../lib/api";
import { t } from "../lib/i18n";
import agentHubLogo from "../assets/agenthub-logo.png";

const targets: Array<{ id: Target; mark: string }> = [
  { id: "cursor", mark: "CU" },
  { id: "codex", mark: "CX" },
  { id: "claude", mark: "CL" },
];

const domains: Array<{ id: Kind; icon: typeof Sparkles }> = [
  { id: "skill", icon: Sparkles },
  { id: "mcp", icon: PlugZap },
  { id: "plugin", icon: Box },
  { id: "rule", icon: FileText },
];

export function SyncRail({ dashboard }: { dashboard: Dashboard }) {
  const total = domains.reduce((sum, domain) => sum + (dashboard.inventory[domain.id] ?? 0), 0);
  const maximum = Math.max(0, ...domains.map((domain) => dashboard.inventory[domain.id] ?? 0));
  return (
    <section className="flow-card material" aria-label={t("overview.title")}>
      <header className="flow-card__header">
        <div>
          <p className="eyebrow">CANONICAL FLOW</p>
          <h2>{t("overview.flowTitle")}</h2>
        </div>
        <span className="flow-card__summary"><Radio size={13} /> {dashboard.auto_sync_targets.length} {t("overview.autoChannels")} · {dashboard.enabled_targets.length} / 3 {t("overview.channels")}</span>
      </header>

      <div className="flow-topology">
        <div className="source-node">
          <div className="source-node__orb">
            <img src={agentHubLogo} alt="" />
          </div>
          <strong>AgentHub</strong>
          <code>~/.agenthub</code>
          <span>{total} {t("overview.resources")}</span>
        </div>

        <div className="flow-streams" aria-label={t("overview.capabilities")}>
          {domains.map(({ id, icon: Icon }) => {
            const count = dashboard.inventory[id] ?? 0;
            const fill = maximum > 0 ? (count / maximum) * 100 : 0;
            return <div className={`flow-stream flow-stream--${id} ${count === 0 ? "is-empty" : ""}`} style={{ "--stream-fill": `${fill}%` } as CSSProperties} key={id}>
              <span className="flow-stream__label"><Icon size={15} /> {t(`kinds.${id}`)}</span>
              <i aria-hidden="true"><b /></i>
              <strong>{count}</strong>
            </div>;
          })}
        </div>

        <div className="target-stack">
          {targets.map(({ id, mark }) => {
            const active = dashboard.enabled_targets.includes(id);
            const automatic = dashboard.auto_sync_targets.includes(id);
            return (
              <div className={`target-node ${active ? "is-active" : ""}`} key={id}>
                <span className="target-node__mark">{mark}</span>
                <span className="target-node__copy">
                  <strong>{t(`targets.${id}`)}</strong>
                  <small>{automatic ? t("overview.autoReady") : active ? t("overview.ready") : t("overview.notEnabled")}</small>
                </span>
                {automatic ? <Radio size={17} /> : active ? <Check size={17} /> : <Circle size={15} />}
              </div>
            );
          })}
        </div>
      </div>

      <footer className="flow-card__footer">
        <span><Code2 size={15} /> Git {t("overview.historyLayer")}</span>
        <span><Braces size={15} /> SQLite {t("overview.stateLayer")}</span>
      </footer>
    </section>
  );
}
