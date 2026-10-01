import { useCurrentUser, useSetStoreActive, useStores } from "@vigo/api-client";
import type { AdminStore } from "@vigo/types";
import { useDeferredValue, useState } from "react";
import { Link } from "react-router";
import { ErrorText } from "../components/ui";
import { canSee } from "../lib/roles";

export function Stores() {
  const [q, setQ] = useState("");
  const query = useDeferredValue(q.trim());
  const stores = useStores(query ? { q: query } : {});
  const setup = canSee(useCurrentUser().role, "storeSetup");

  return (
    <section className="card">
      <div className="card-head">
        <h2>Dark stores</h2>
        {setup ? (
          <Link className="btn" to="/stores/new">
            New store
          </Link>
        ) : null}
      </div>
      <div className="filters">
        <input placeholder="Search name, code or address" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      <ErrorText error={stores.error} />
      <table>
        <thead>
          <tr>
            <th>Code</th>
            <th>Name</th>
            <th>Area</th>
            <th>Status</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {stores.data?.items.map((s) => (
            <tr key={s.id}>
              <td>{setup ? <Link to={`/stores/${s.id}`}>{s.code}</Link> : s.code}</td>
              <td>{s.name}</td>
              <td className="muted">{s.areaSqKm} km²</td>
              <td>
                <span className={`pill ${s.isActive ? "pill-ok" : ""}`}>{s.isActive ? "open" : "closed"}</span>
              </td>
              <td>
                <OpenSwitch store={s} />
              </td>
            </tr>
          ))}
          {stores.data?.items.length === 0 ? (
            <tr>
              <td colSpan={5} className="muted">
                No stores yet.
              </td>
            </tr>
          ) : null}
        </tbody>
      </table>
    </section>
  );
}

/** Opens or closes a store; a closed store takes no orders and hides its catalog. */
function OpenSwitch({ store }: { store: AdminStore }) {
  const setActive = useSetStoreActive();
  const next = !store.isActive;
  return (
    <>
      <button
        type="button"
        className="btn secondary small-btn"
        disabled={setActive.isPending}
        onClick={() => {
          if (!next && !window.confirm(`Close ${store.code}? Customers in its area can't order until it reopens.`)) return;
          setActive.mutate({ id: store.id, body: { isActive: next } });
        }}
      >
        {next ? "Open store" : "Close store"}
      </button>
      <ErrorText error={setActive.error} />
    </>
  );
}
