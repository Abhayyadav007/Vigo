import { useHealth } from "@vigo/api-client";
import type { ComponentStatus } from "@vigo/types";

// TODO(phase-2): Firebase login + ADMIN role gate; TODO(phase-3): store/catalog screens.
export function App() {
  const health = useHealth();

  const cell = (s: ComponentStatus | undefined) => {
    const value = s ?? (health.isError ? "unreachable" : "…");
    return <span className={`pill pill-${s ?? (health.isError ? "down" : "pending")}`}>{value}</span>;
  };

  return (
    <main className="shell">
      <h1>Vigo Admin</h1>
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
    </main>
  );
}
