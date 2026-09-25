import { formatIndianPhone, useAuth } from "@vigo/api-client";
import { lazy, Suspense } from "react";
import { createBrowserRouter, NavLink, Outlet, RouterProvider } from "react-router";
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

const NAV = [
  ["/", "Overview"],
  ["/stores", "Stores"],
  ["/categories", "Categories"],
  ["/products", "Products"],
  ["/inventory", "Inventory"],
  ["/staff", "Staff"],
] as const;

const router = createBrowserRouter([
  {
    element: <Layout />,
    children: [
      { index: true, element: <Overview /> },
      { path: "stores", element: <Stores /> },
      { path: "stores/new", element: <StoreEditor /> },
      { path: "stores/:id", element: <StoreEditor /> },
      { path: "categories", element: <Categories /> },
      { path: "products", element: <Products /> },
      { path: "products/new", element: <ProductEditor /> },
      { path: "products/:id", element: <ProductEditor /> },
      { path: "inventory", element: <Inventory /> },
      { path: "staff", element: <Staff /> },
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
  return (
    <div className="layout">
      <header className="topbar">
        <strong className="brand">Vigo Admin</strong>
        <nav>
          {NAV.map(([to, label]) => (
            <NavLink key={to} to={to} end={to === "/"} className={({ isActive }) => `tab ${isActive ? "active" : ""}`}>
              {label}
            </NavLink>
          ))}
        </nav>
        {state.status === "signedIn" ? (
          <span className="muted" data-testid="signed-in-as">
            {formatIndianPhone(state.user.phone)}
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
