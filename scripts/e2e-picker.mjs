#!/usr/bin/env node
// End-to-end picking against a running backend, including the live
// WebSocket feed: customer orders -> picker sees it live -> claims -> scans
// every barcode (plus a wrong one) -> packs -> the feed shows each step.
//
// Needs the Firebase Auth emulator, a backend with FIREBASE_AUTH_EMULATOR_HOST
// set, and the demo seed. PROMOTE_CMD bootstraps an admin (default: cargo run).
// Usage: node scripts/e2e-picker.mjs [backend_url]

import { API, api, check, finish, json, phone, promoteAdmin, signIn } from "./e2e-lib.mjs";

const HOME = { lat: 12.9784, lng: 77.6408 };

// --- people: admin (bootstrapped), picker (at the serving store), customer
const admin = await signIn(phone("97"));
promoteAdmin(admin.phone);

const svc = await json(`${API}/v1/customer/serviceability?lat=${HOME.lat}&lng=${HOME.lng}`);
const storeId = svc.body.store.id;
const picker = await signIn(phone("94"));
const promoted = await api(`/v1/admin/users/${picker.me.id}/role`, admin.token, {
  method: "PATCH",
  body: JSON.stringify({ role: "PICKER", storeId }),
});
check("admin assigns a picker to the store", promoted.body?.role === "PICKER", JSON.stringify(promoted.body));

// --- picker opens the live feed
const events = [];
const ws = new WebSocket(`${API.replace(/^http/, "ws")}/v1/ws/picker`);
const ready = new Promise((resolve, reject) => {
  ws.onopen = () => ws.send(JSON.stringify({ type: "auth", token: picker.token }));
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.type === "ready") resolve();
    else if (msg.type === "order") events.push(msg.event);
    else if (msg.type === "error") reject(new Error(msg.message));
  };
  setTimeout(() => reject(new Error("no ready")), 5000);
});
await ready;
check("picker websocket authenticated", true);
const waitFor = async (pred, what) => {
  for (let i = 0; i < 50; i++) {
    if (events.some(pred)) return true;
    await new Promise((r) => setTimeout(r, 100));
  }
  check(`live event: ${what}`, false, JSON.stringify(events));
  return false;
};

// --- customer orders 2 products
const customer = await signIn(phone("93"));
const address = await api("/v1/customer/addresses", customer.token, {
  method: "POST",
  body: JSON.stringify({ label: "Home", line1: "Flat 7", city: "Bengaluru", pincode: "560038", location: HOME }),
});
const products = await json(`${API}/v1/customer/catalog/products?storeId=${storeId}&limit=50`);
const picks = products.body.items.filter((p) => p.inStock).slice(0, 2);
for (const p of picks) {
  await api(`/v1/customer/cart/items/${p.id}`, customer.token, {
    method: "PUT",
    body: JSON.stringify({ storeId, quantity: 1 }),
  });
}
const checkout = await api("/v1/customer/checkout", customer.token, {
  method: "POST",
  headers: { "idempotency-key": `pk-${Date.now()}` },
  body: JSON.stringify({ storeId, addressId: address.body.id, paymentMethod: "COD" }),
});
const orderId = checkout.body?.order?.id;
check("customer order confirmed", checkout.body?.order?.status === "CONFIRMED", JSON.stringify(checkout.body));
if (await waitFor((e) => e.orderId === orderId && e.status === "CONFIRMED", "new order reaches the picker"))
  check("live event: new order reaches the picker", true);

// --- pick it
const queue = await api("/v1/picker/orders", picker.token);
check("order is in the picker queue", queue.body?.some?.((o) => o.id === orderId));
const started = await api(`/v1/picker/orders/${orderId}/start`, picker.token, { method: "POST" });
check("picker claims the order", started.body?.status === "PICKING", JSON.stringify(started.body));

const wrong = await api(`/v1/picker/orders/${orderId}/scan`, picker.token, {
  method: "POST",
  body: JSON.stringify({ barcode: "8901234567890" }),
});
check("wrong item is rejected", wrong.status === 422 && wrong.body?.error?.code === "SCAN_MISMATCH");

for (const line of started.body.lines) {
  for (let i = 0; i < line.quantity; i++) {
    const r = await api(`/v1/picker/orders/${orderId}/scan`, picker.token, {
      method: "POST",
      body: JSON.stringify({ barcode: line.barcode }),
    });
    if (r.status !== 200) check(`scan ${line.name}`, false, JSON.stringify(r.body));
  }
}
const packed = await api(`/v1/picker/orders/${orderId}/pack`, picker.token, {
  method: "POST",
  body: JSON.stringify({ bagCount: 1, stagingSlot: "S-01" }),
});
check("order packed", packed.body?.status === "PACKED", JSON.stringify(packed.body));
if (await waitFor((e) => e.orderId === orderId && e.status === "PACKED", "PACKED reaches the feed"))
  check("live events: PICKING and PACKED streamed", events.some((e) => e.orderId === orderId && e.status === "PICKING"));

const seen = await api(`/v1/customer/orders/${orderId}`, customer.token);
check("customer sees PACKED", seen.body?.status === "PACKED");

ws.close();
finish("e2e picker passed");
