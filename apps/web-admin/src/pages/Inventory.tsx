import { formatPaise, rupeesToPaise, useInventory, useSetInventory, useStores } from "@vigo/api-client";
import type { InventoryItem } from "@vigo/types";
import { useDeferredValue, useState } from "react";
import { useSearchParams } from "react-router";
import { errorText, ErrorText, Pager } from "../components/ui";

const LIMIT = 50;

export function Inventory() {
  const [params, setParams] = useSearchParams();
  const stores = useStores();
  const storeId = params.get("store") ?? stores.data?.items[0]?.id;
  const [q, setQ] = useState("");
  const [stocked, setStocked] = useState<"" | "true" | "false">("");
  const [offset, setOffset] = useState(0);
  const search = useDeferredValue(q.trim());
  const inventory = useInventory(
    storeId,
    { ...(search ? { q: search } : {}), ...(stocked ? { stocked: stocked === "true" } : {}) },
    { limit: LIMIT, offset },
  );

  return (
    <section className="card">
      <div className="card-head">
        <h2>Inventory</h2>
        <select
          aria-label="Store"
          value={storeId ?? ""}
          onChange={(e) => {
            setParams({ store: e.target.value });
            setOffset(0);
          }}
        >
          {stores.data?.items.map((s) => (
            <option key={s.id} value={s.id}>
              {s.code} · {s.name}
            </option>
          ))}
        </select>
      </div>
      {stores.data?.items.length === 0 ? <p className="muted">Create a store first.</p> : null}
      <div className="filters">
        <input
          placeholder="Search product, barcode or bin"
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setOffset(0);
          }}
        />
        <select value={stocked} onChange={(e) => setStocked(e.target.value as typeof stocked)}>
          <option value="">All products</option>
          <option value="true">Carried by this store</option>
          <option value="false">Not carried yet</option>
        </select>
      </div>
      <ErrorText error={inventory.error} />
      <table>
        <thead>
          <tr>
            <th>Product</th>
            <th>Bin</th>
            <th>Qty</th>
            <th>Store price</th>
            <th>Available</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {storeId
            ? inventory.data?.items.map((item) => <Row key={item.productId} storeId={storeId} item={item} />)
            : null}
        </tbody>
      </table>
      <Pager total={inventory.data?.total ?? 0} limit={LIMIT} offset={offset} onChange={setOffset} />
    </section>
  );
}

/** Inline editor for one product at one store; Save upserts the inventory row. */
function Row({ storeId, item }: { storeId: string; item: InventoryItem }) {
  const set = useSetInventory(storeId);
  const [qty, setQty] = useState(String(item.quantity));
  const [bin, setBin] = useState(item.binLocation ?? "");
  const [price, setPrice] = useState(item.priceOverridePaise === null ? "" : String(item.priceOverridePaise / 100));
  const [available, setAvailable] = useState(item.stocked ? item.isAvailable : true);

  const quantity = Number(qty);
  const override = price.trim() ? rupeesToPaise(price) : null;
  const valid = Number.isInteger(quantity) && quantity >= 0 && (price.trim() === "" || override !== null);
  const dirty =
    !item.stocked ||
    quantity !== item.quantity ||
    bin !== (item.binLocation ?? "") ||
    override !== item.priceOverridePaise ||
    available !== item.isAvailable;

  return (
    <tr data-testid={`inv-${item.productName}`} className={item.stocked ? "" : "unstocked"}>
      <td>
        {item.productName}
        <div className="muted small">
          {[item.brand, item.unitLabel, `MRP ${formatPaise(item.mrpPaise)}`].filter(Boolean).join(" · ")}
        </div>
      </td>
      <td>
        <input className="narrow" aria-label="Bin" placeholder="A-01-1" value={bin} onChange={(e) => setBin(e.target.value)} />
      </td>
      <td>
        <input className="narrow" aria-label="Quantity" inputMode="numeric" value={qty} onChange={(e) => setQty(e.target.value)} />
      </td>
      <td>
        <input
          className="narrow"
          aria-label="Store price"
          inputMode="decimal"
          placeholder={formatPaise(item.basePricePaise)}
          value={price}
          onChange={(e) => setPrice(e.target.value)}
        />
      </td>
      <td>
        <input aria-label="Available" type="checkbox" checked={available} onChange={(e) => setAvailable(e.target.checked)} />
      </td>
      <td>
        <button
          type="button"
          className="btn secondary"
          disabled={!valid || !dirty || set.isPending}
          onClick={() =>
            set.mutate({
              productId: item.productId,
              body: {
                quantity,
                isAvailable: available,
                ...(bin.trim() ? { binLocation: bin.trim() } : {}),
                ...(override !== null ? { priceOverridePaise: override } : {}),
              },
            })
          }
        >
          {item.stocked ? "Save" : "Stock"}
        </button>
        {set.error ? <div className="error small">{errorText(set.error)}</div> : null}
      </td>
    </tr>
  );
}
