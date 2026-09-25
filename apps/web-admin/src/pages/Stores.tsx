import { useStores } from "@vigo/api-client";
import { useDeferredValue, useState } from "react";
import { Link } from "react-router";
import { ErrorText } from "../components/ui";

export function Stores() {
  const [q, setQ] = useState("");
  const query = useDeferredValue(q.trim());
  const stores = useStores(query ? { q: query } : {});

  return (
    <section className="card">
      <div className="card-head">
        <h2>Dark stores</h2>
        <Link className="btn" to="/stores/new">
          New store
        </Link>
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
          </tr>
        </thead>
        <tbody>
          {stores.data?.items.map((s) => (
            <tr key={s.id}>
              <td>
                <Link to={`/stores/${s.id}`}>{s.code}</Link>
              </td>
              <td>{s.name}</td>
              <td className="muted">{s.areaSqKm} km²</td>
              <td>
                <span className={`pill ${s.isActive ? "pill-ok" : ""}`}>{s.isActive ? "active" : "inactive"}</span>
              </td>
            </tr>
          ))}
          {stores.data?.items.length === 0 ? (
            <tr>
              <td colSpan={4} className="muted">
                No stores yet.
              </td>
            </tr>
          ) : null}
        </tbody>
      </table>
    </section>
  );
}
