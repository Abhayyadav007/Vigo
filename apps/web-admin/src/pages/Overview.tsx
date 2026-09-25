import { useHealth } from "@vigo/api-client";
import type { ComponentStatus } from "@vigo/types";

export function Overview() {
  const health = useHealth();

  const cell = (s: ComponentStatus | undefined) => {
    const value = s ?? (health.isError ? "unreachable" : "…");
    return <span className={`pill pill-${s ?? (health.isError ? "down" : "pending")}`}>{value}</span>;
  };

  return (
    <section className="card">
      <h2>Backend</h2>
      <dl>
        <dt>API</dt>
        <dd>{cell(health.data?.status)}</dd>
        <dt>Postgres</dt>
        <dd>{cell(health.data?.database)}</dd>
        <dt>Redis</dt>
        <dd>{cell(health.data?.redis)}</dd>
        <dt>Version</dt>
        <dd>{health.data?.version ?? "—"}</dd>
      </dl>
    </section>
  );
}
