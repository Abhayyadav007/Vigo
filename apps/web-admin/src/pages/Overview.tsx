import { formatPaise, useHealth, useLiveBoard, useStores } from "@vigo/api-client";
import type { BoardOrder, OrderStatus } from "@vigo/types";
import { useState } from "react";

const COLUMNS: [string, OrderStatus[]][] = [
  ["New", ["PLACED", "CONFIRMED"]],
  ["Picking", ["PICKING"]],
  ["Packed", ["PACKED"]],
  ["With rider", ["RIDER_ASSIGNED", "PICKED_UP", "OUT_FOR_DELIVERY"]],
];

const minutesSince = (iso: string) => Math.max(0, Math.round((Date.now() - new Date(iso).getTime()) / 60_000));

export function Overview() {
  const [storeId, setStoreId] = useState("");
  const stores = useStores();
  const { board, metrics, live } = useLiveBoard(storeId || undefined);
  const health = useHealth();
  const m = metrics.data;

  return (
    <>
      <section className="card-head">
        <h2>
          Live orders <span className={`pill ${live === "open" ? "pill-ok" : ""}`}>{live === "open" ? "live" : "connecting"}</span>
        </h2>
        <select aria-label="Store" value={storeId} onChange={(e) => setStoreId(e.target.value)}>
          <option value="">All stores</option>
          {stores.data?.items.map((s) => (
            <option key={s.id} value={s.id}>
              {s.code}
            </option>
          ))}
        </select>
      </section>

      <section className="metrics">
        <Metric label="Orders today" value={m?.ordersToday} />
        <Metric label="Delivered" value={m?.deliveredToday} />
        <Metric label="Cancelled" value={m?.cancelledToday} />
        <Metric label="GMV today" value={m ? formatPaise(m.gmvTodayPaise) : undefined} />
        <Metric label="Avg delivery" value={m?.avgDeliveryMinutes != null ? `${m.avgDeliveryMinutes} min` : "—"} />
        <Metric label="Riders online" value={m?.ridersOnline} />
      </section>

      <section className="board" data-testid="live-board">
        {COLUMNS.map(([title, statuses]) => {
          const orders = (board.data ?? []).filter((o) => statuses.includes(o.status));
          return (
            <div key={title} className="card column">
              <h3>
                {title} <span className="muted">{orders.length}</span>
              </h3>
              {orders.map((o) => (
                <OrderCard key={o.id} order={o} />
              ))}
            </div>
          );
        })}
      </section>

      <p className="muted small">
        API {health.data?.status ?? "…"} · Postgres {health.data?.database ?? "…"} · Redis {health.data?.redis ?? "…"} · v
        {health.data?.version ?? "?"}
      </p>
    </>
  );
}

function Metric({ label, value }: { label: string; value: string | number | undefined }) {
  return (
    <div className="card metric">
      <span className="muted small">{label}</span>
      <strong>{value ?? "…"}</strong>
    </div>
  );
}

function OrderCard({ order: o }: { order: BoardOrder }) {
  const age = minutesSince(o.createdAt);
  return (
    <div className={`order-card ${age >= 10 ? "late" : ""}`} data-testid={`board-${o.number}`}>
      <div className="row">
        <strong>{o.number}</strong>
        <span className="muted small">{age} min</span>
      </div>
      <div className="muted small">
        {o.storeCode} · {o.itemCount} items · {formatPaise(o.totalPaise)} {o.paymentMethod}
      </div>
      <div className="small">{o.status.replaceAll("_", " ").toLowerCase()}</div>
      {o.riderPhone ? <div className="muted small">Rider {o.riderPhone}</div> : null}
    </div>
  );
}
