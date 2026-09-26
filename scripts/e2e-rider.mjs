#!/usr/bin/env node
// End-to-end delivery against a running backend: customer orders, picker
// packs, an online rider near the store gets the offer over the WebSocket,
// accepts, picks up, rides to the customer (auto OUT_FOR_DELIVERY), and
// completes with the customer's OTP and the exact COD amount.
//
// Needs the Firebase Auth emulator, a backend with FIREBASE_AUTH_EMULATOR_HOST
// set, and the demo seed. PROMOTE_CMD bootstraps an admin (default: cargo run).
// Usage: node scripts/e2e-rider.mjs [backend_url]

import { execFileSync } from "node:child_process";
import { randomInt } from "node:crypto";

const API = process.argv[2] ?? "http://localhost:8080";
const EMU = `http://${process.env.FIREBASE_AUTH_EMULATOR_HOST ?? "localhost:9099"}`;
const PROJECT = process.env.FIREBASE_PROJECT_ID ?? "demo-vigo";
const IDT = `${EMU}/identitytoolkit.googleapis.com/v1`;
// Demo Indiranagar store is at 12.9719, 77.6412; the customer ~700 m north.
const STORE = { lat: 12.9719, lng: 77.6412 };
const HOME = { lat: 12.9784, lng: 77.6408 };

let failures = 0;
function check(name, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : `  ${detail}`}`);
  if (!ok) failures++;
}

async function json(url, init = {}) {
  const res = await fetch(url, { ...init, headers: { "content-type": "application/json", ...init.headers } });
  const text = await res.text();
  return { status: res.status, body: text ? JSON.parse(text) : null };
}

const phone = (prefix) => `+91${prefix}${String(randomInt(0, 1e8)).padStart(8, "0")}`;

async function signIn(phoneNumber) {
  const sent = await json(`${IDT}/accounts:sendVerificationCode?key=k`, {
    method: "POST",
    body: JSON.stringify({ phoneNumber, recaptchaToken: "x" }),
  });
  const codes = await json(`${EMU}/emulator/v1/projects/${PROJECT}/verificationCodes`);
  const code = codes.body.verificationCodes.find((c) => c.sessionInfo === sent.body.sessionInfo).code;
  const res = await json(`${IDT}/accounts:signInWithPhoneNumber?key=k`, {
    method: "POST",
    body: JSON.stringify({ sessionInfo: sent.body.sessionInfo, code }),
  });
  const token = res.body.idToken;
  const me = await json(`${API}/v1/auth/sync`, { method: "POST", headers: { authorization: `Bearer ${token}` } });
  return { token, me: me.body, phone: phoneNumber };
}

const api = (path, token, init = {}) =>
  json(`${API}${path}`, { ...init, headers: { authorization: `Bearer ${token}`, ...init.headers } });
const locate = (token, at) =>
  api("/v1/rider/location", token, {
    method: "POST",
    body: JSON.stringify({ points: [{ ...at, recordedAt: Date.now() }] }),
  });

// --- staff
const admin = await signIn(phone("97"));
const cmd = (process.env.PROMOTE_CMD ?? "cargo run -q -p backend -- promote-admin").split(" ");
execFileSync(cmd[0], [...cmd.slice(1), admin.phone], { stdio: "ignore" });
const storeId = (await json(`${API}/v1/customer/serviceability?lat=${HOME.lat}&lng=${HOME.lng}`)).body.store.id;
const assign = async (user, role) =>
  api(`/v1/admin/users/${user.me.id}/role`, admin.token, { method: "PATCH", body: JSON.stringify({ role, storeId }) });
const picker = await signIn(phone("94"));
await assign(picker, "PICKER");
const rider = await signIn(phone("92"));
await assign(rider, "RIDER");

const online = await api("/v1/rider/status", rider.token, { method: "PUT", body: JSON.stringify({ online: true }) });
check("rider goes online", online.body?.isOnline === true, JSON.stringify(online.body));
check("rider location accepted", (await locate(rider.token, { lat: STORE.lat + 0.0003, lng: STORE.lng })).status === 204);

// --- rider's live feed
const feed = [];
const ws = new WebSocket(`${API.replace(/^http/, "ws")}/v1/ws/rider`);
await new Promise((resolve, reject) => {
  ws.onopen = () => ws.send(JSON.stringify({ type: "auth", token: rider.token }));
  ws.onmessage = (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.type === "ready") resolve();
    else if (msg.type === "error") reject(new Error(msg.message));
    else feed.push(msg);
  };
  setTimeout(() => reject(new Error("rider socket not ready")), 5000);
});
check("rider websocket ready", true);

// --- customer order, picked and packed
const customer = await signIn(phone("93"));
const address = await api("/v1/customer/addresses", customer.token, {
  method: "POST",
  body: JSON.stringify({ label: "Home", line1: "Flat 3C", city: "Bengaluru", pincode: "560038", location: HOME }),
});
const products = await json(`${API}/v1/customer/catalog/products?storeId=${storeId}&limit=50`);
const item = products.body.items.find((p) => p.inStock);
await api(`/v1/customer/cart/items/${item.id}`, customer.token, {
  method: "PUT",
  body: JSON.stringify({ storeId, quantity: 1 }),
});
const checkout = await api("/v1/customer/checkout", customer.token, {
  method: "POST",
  headers: { "idempotency-key": `rd-${Date.now()}` },
  body: JSON.stringify({ storeId, addressId: address.body.id, paymentMethod: "COD" }),
});
const orderId = checkout.body.order.id;
const started = await api(`/v1/picker/orders/${orderId}/start`, picker.token, { method: "POST" });
for (const line of started.body.lines) {
  for (let i = 0; i < line.quantity; i++) {
    await api(`/v1/picker/orders/${orderId}/scan`, picker.token, {
      method: "POST",
      body: JSON.stringify({ barcode: line.barcode }),
    });
  }
}
const packed = await api(`/v1/picker/orders/${orderId}/pack`, picker.token, {
  method: "POST",
  body: JSON.stringify({ bagCount: 1, stagingSlot: "S-07" }),
});
check("order packed", packed.body?.status === "PACKED", JSON.stringify(packed.body));

// --- offer arrives live
let offer;
for (let i = 0; i < 50 && !offer; i++) {
  offer = feed.find((m) => m.type === "offer" && m.offer.orderId === orderId);
  if (!offer) await new Promise((r) => setTimeout(r, 100));
}
check("offer pushed to the rider", !!offer, JSON.stringify(feed));
check("offer shows cash to collect", offer?.offer.collectPaise === checkout.body.order.bill.totalPaise);

const accepted = await api(`/v1/rider/offers/${orderId}/accept`, rider.token, { method: "POST" });
check("rider accepts", accepted.body?.status === "RIDER_ASSIGNED", JSON.stringify(accepted.body));
check("rider sees staging slot", accepted.body?.stagingSlot === "S-07");
const seenByCustomer = await api(`/v1/customer/orders/${orderId}`, customer.token);
check("customer sees the rider", seenByCustomer.body?.rider?.phone === rider.phone);

const picked = await api(`/v1/rider/deliveries/${orderId}/pickup`, rider.token, {
  method: "POST",
  body: JSON.stringify({ bagCount: 1 }),
});
check("pickup confirmed", picked.body?.status === "PICKED_UP", JSON.stringify(picked.body));

// --- ride to the customer
await locate(rider.token, { lat: 12.975, lng: 77.641 });
const moving = await api(`/v1/customer/orders/${orderId}`, customer.token);
check("leaving the store -> OUT_FOR_DELIVERY", moving.body?.status === "OUT_FOR_DELIVERY", moving.body?.status);
await locate(rider.token, { lat: HOME.lat - 0.0002, lng: HOME.lng });

const otp = moving.body.deliveryOtp;
const wrong = await api(`/v1/rider/deliveries/${orderId}/deliver`, rider.token, {
  method: "POST",
  body: JSON.stringify({ otp: otp === "0000" ? "1111" : "0000", codCollectedPaise: picked.body.collectPaise }),
});
check("wrong OTP rejected", wrong.status === 422 && wrong.body?.error?.code === "WRONG_OTP", JSON.stringify(wrong.body));
const done = await api(`/v1/rider/deliveries/${orderId}/deliver`, rider.token, {
  method: "POST",
  body: JSON.stringify({ otp, codCollectedPaise: picked.body.collectPaise }),
});
check("delivered with the right OTP + cash", done.body?.status === "DELIVERED", JSON.stringify(done.body));

const final = await api(`/v1/customer/orders/${orderId}`, customer.token);
check("customer sees DELIVERED, paid", final.body?.status === "DELIVERED" && final.body?.paymentStatus === "PAID");
check(
  "rider feed carried the status changes",
  ["PICKED_UP", "OUT_FOR_DELIVERY", "DELIVERED"].every((s) => feed.some((m) => m.type === "order" && m.event.status === s)),
  JSON.stringify(feed.map((m) => m.event?.status ?? m.type)),
);

await api("/v1/rider/status", rider.token, { method: "PUT", body: JSON.stringify({ online: false }) });
ws.close();
if (failures) {
  console.error(`\n${failures} check(s) failed`);
  process.exit(1);
}
console.log("\ne2e rider passed");
