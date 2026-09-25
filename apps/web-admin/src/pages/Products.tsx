import { resolveMediaUrl, useApiBaseUrl, useCategories, useProducts } from "@vigo/api-client";
import { useDeferredValue, useState } from "react";
import { Link } from "react-router";
import { ErrorText, money, Pager } from "../components/ui";

const LIMIT = 20;

export function Products() {
  const [q, setQ] = useState("");
  const [categoryId, setCategoryId] = useState("");
  const [offset, setOffset] = useState(0);
  const search = useDeferredValue(q.trim());
  const categories = useCategories();
  const products = useProducts(
    { ...(search ? { q: search } : {}), ...(categoryId ? { categoryId } : {}) },
    { limit: LIMIT, offset },
  );
  const base = useApiBaseUrl();

  return (
    <section className="card">
      <div className="card-head">
        <h2>Products</h2>
        <Link className="btn" to="/products/new">
          New product
        </Link>
      </div>
      <div className="filters">
        <input
          placeholder="Search name, brand or barcode"
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setOffset(0);
          }}
        />
        <select
          value={categoryId}
          onChange={(e) => {
            setCategoryId(e.target.value);
            setOffset(0);
          }}
        >
          <option value="">All categories</option>
          {categories.data?.map((c) => (
            <option key={c.id} value={c.id}>
              {c.name}
            </option>
          ))}
        </select>
      </div>
      <ErrorText error={products.error} />
      <table>
        <thead>
          <tr>
            <th />
            <th>Product</th>
            <th>Category</th>
            <th>MRP</th>
            <th>Price</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody>
          {products.data?.items.map((p) => {
            const img = resolveMediaUrl(p.imageUrls[0], base);
            return (
              <tr key={p.id}>
                <td>{img ? <img className="thumb" src={img} alt="" /> : <div className="thumb" />}</td>
                <td>
                  <Link to={`/products/${p.id}`}>{p.name}</Link>
                  <div className="muted small">
                    {[p.brand, p.unitLabel, p.barcode].filter(Boolean).join(" · ")}
                  </div>
                </td>
                <td className="muted">{p.categoryName}</td>
                <td className="muted">{money(p.mrpPaise)}</td>
                <td>{money(p.pricePaise)}</td>
                <td>
                  <span className={`pill ${p.isActive ? "pill-ok" : ""}`}>{p.isActive ? "active" : "hidden"}</span>
                </td>
              </tr>
            );
          })}
          {products.data?.items.length === 0 ? (
            <tr>
              <td colSpan={6} className="muted">
                No products match.
              </td>
            </tr>
          ) : null}
        </tbody>
      </table>
      <Pager total={products.data?.total ?? 0} limit={LIMIT} offset={offset} onChange={setOffset} />
    </section>
  );
}
