import { formatIndianPhone, useAuth } from "@vigo/api-client";
import { useState } from "react";
import { Login } from "./pages/Login";
import { Overview } from "./pages/Overview";
import { Staff } from "./pages/Staff";

const PAGES = { overview: "Overview", staff: "Staff" } as const;
type Page = keyof typeof PAGES;

// TODO(phase-3): replace the tab state with a router once there are more pages.
export function App() {
  const { state, signOut } = useAuth();
  const [page, setPage] = useState<Page>("overview");

  if (state.status === "loading") return <div className="center muted">Loading…</div>;
  if (state.status === "signedOut") return <Login error={state.error} />;

  return (
    <div className="layout">
      <header className="topbar">
        <strong className="brand">Vigo Admin</strong>
        <nav>
          {(Object.keys(PAGES) as Page[]).map((p) => (
            <button key={p} className={`tab ${p === page ? "active" : ""}`} onClick={() => setPage(p)}>
              {PAGES[p]}
            </button>
          ))}
        </nav>
        <span className="muted" data-testid="signed-in-as">
          {formatIndianPhone(state.user.phone)}
        </span>
        <button className="btn secondary" onClick={() => void signOut()}>
          Sign out
        </button>
      </header>
      <main className="shell">{page === "overview" ? <Overview /> : <Staff currentUserId={state.user.id} />}</main>
    </div>
  );
}
