import { formatIndianPhone, ROLE_LABEL, useAuth, useCurrentUser } from "@vigo/api-client";
import { lazy, Suspense, type ReactNode } from "react";
import { createBrowserRouter, Navigate, NavLink, Outlet, RouterProvider } from "react-router";
import { canSee, type Page } from "./lib/roles";
import { Categories } from "./pages/Categories";
import { Inventory } from "./pages/Inventory";
import { Login } from "./pages/Login";
import { Overview } from "./pages/Overview";
import { ProductEditor } from "./pages/ProductEditor";
import { Products } from "./pages/Products";
import { Staff } from "./pages/Staff";
import { Stores } from "./pages/Stores";

// Leaflet is heavy and only needed here; keep it out of the main bundle.
const StoreEditor = lazy(() => import("./pages/StoreEditor").then((m) => ({ default: m.StoreEditor })));

const NAV: readonly (readonly [string, string, Page])[] = [
  ["/", "Overview", "overview"],
  ["/stores", "Stores", "stores"],
  ["/categories", "Categories", "catalog"],
  ["/products", "Products", "catalog"],
  ["/inventory", "Inventory", "inventory"],
  ["/staff", "Staff", "staff"],
];

/** Renders a page only for roles allowed to see it; others land on their first page. */
function Gate({ page, children }: { page: Page; children: ReactNode }) {
  const { role } = useCurrentUser();
  if (canSee(role, page)) return <>{children}</>;
  const home = NAV.find(([, , p]) => canSee(role, p));
  return home && home[0] !== "/" ? <Navigate to={home[0]} replace /> : <p className="muted">Not available for your role.</p>;
}

const router = createBrowserRouter([
  {
    element: <Layout />,
    children: [
      { index: true, element: <Gate page="overview"><Overview /></Gate> },
      { path: "stores", element: <Gate page="stores"><Stores /></Gate> },
      { path: "stores/new", element: <Gate page="storeSetup"><StoreEditor /></Gate> },
      { path: "stores/:id", element: <Gate page="storeSetup"><StoreEditor /></Gate> },
      { path: "categories", element: <Gate page="catalog"><Categories /></Gate> },
      { path: "products", element: <Gate page="catalog"><Products /></Gate> },
      { path: "products/new", element: <Gate page="catalog"><ProductEditor /></Gate> },
      { path: "products/:id", element: <Gate page="catalog"><ProductEditor /></Gate> },
      { path: "inventory", element: <Gate page="inventory"><Inventory /></Gate> },
      { path: "staff", element: <Gate page="staff"><Staff /></Gate> },
      { path: "*", element: <p className="muted">Page not found.</p> },
    ],
  },
]);

export function App() {
  const { state } = useAuth();
  if (state.status === "loading") return <div className="center muted">Loading…</div>;
  if (state.status === "signedOut") return <Login error={state.error} />;
  return <RouterProvider router={router} />;
}

function Layout() {
  const { state, signOut } = useAuth();
  const { role } = useCurrentUser();
  return (
    <div className="layout">
      <header className="topbar">
        <strong className="brand">Vigo Admin</strong>
        <nav>
          {NAV.filter(([, , page]) => canSee(role, page)).map(([to, label]) => (
            <NavLink key={to} to={to} end={to === "/"} className={({ isActive }) => `tab ${isActive ? "active" : ""}`}>
              {label}
            </NavLink>
          ))}
        </nav>
        {state.status === "signedIn" ? (
          <span className="muted" data-testid="signed-in-as">
            {formatIndianPhone(state.user.phone)} · {ROLE_LABEL[role]}
          </span>
        ) : null}
        <button className="btn secondary" onClick={() => void signOut()}>
          Sign out
        </button>
      </header>
      <main className="shell">
        <Suspense fallback={<p className="muted">Loading…</p>}>
          <Outlet />
        </Suspense>
      </main>
    </div>
  );
}
