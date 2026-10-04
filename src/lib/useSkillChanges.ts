import { useCallback, useEffect, useRef, useState } from "react";
import { api, type SkillChangeReport } from "./api";

const CHECK_INTERVAL = 120_000;

/** Shared reminder state; checking never imports or changes tool files. */
export function useSkillChanges(enabled: boolean, page: string) {
  const [report, setReport] = useState<SkillChangeReport>({ changes: [], errors: [] });
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef(false);
  const checked = useRef<number | null>(null);
  const generation = useRef(0);
  const check = useCallback(async (refresh = false) => {
    if (!enabled || pending.current || (!refresh && (document.hidden || (checked.current !== null && Date.now() - checked.current < CHECK_INTERVAL)))) return;
    pending.current = true;
    checked.current = Date.now();
    const current = generation.current;
    setChecking(true);
    try {
      const result = await api.checkSkillChanges(refresh);
      if (current === generation.current) { setReport(result); setError(""); }
    } catch (value) {
      if (current === generation.current) setError(String(value));
    } finally {
      pending.current = false;
      if (current === generation.current) setChecking(false);
    }
  }, [enabled]);
  useEffect(() => {
    if (!enabled) { generation.current++; checked.current = null; setReport({ changes: [], errors: [] }); setError(""); setChecking(false); return; }
    void check();
    const resume = () => { void check(); };
    const timer = window.setInterval(resume, CHECK_INTERVAL);
    window.addEventListener("focus", resume);
    document.addEventListener("visibilitychange", resume);
    return () => { window.clearInterval(timer); window.removeEventListener("focus", resume); document.removeEventListener("visibilitychange", resume); };
  }, [enabled, check]);
  useEffect(() => { void check(); }, [page, check]);
  return { ...report, checking, error, check };
}
