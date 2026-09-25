#!/usr/bin/env node
// End-to-end purchase against a running backend: emulator phone login ->
// serviceability -> address -> cart -> COD checkout -> cancel -> restock.
//
// Needs the Firebase Auth emulator, a backend with FIREBASE_AUTH_EMULATOR_HOST
// set, and the demo seed (`backend seed-demo`).
// Usage: node scripts/e2e-orders.mjs [backend_url]

const API = process.argv[2] ?? "http://localhost:8080";
const EMU = `http://${process.env.FIREBASE_AUTH_EMULATOR_HOST ?? "localhost:9099"}`;
const PROJECT = process.env.FIREBASE_PROJECT_ID ?? "demo-vigo";
const IDT = `${EMU}/identitytoolkit.googleapis.com/v1`;
// Inside the demo Indiranagar store (seed-demo).
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

async function phoneSignIn(phoneNumber) {
  const sent = await json(`${IDT}/accounts:sendVerificationCode?key=k`, {
    method: "POST",
    body: JSON.stringify({ phoneNumber, recaptchaToken: "x" }),
  });
  const codes = await json(`${EMU}/emulator/v1/projects/${PROJECT}/verificationCodes`);
  const code = codes.body.verificationCodes.find((c) => c.sessionInfo === sent.body.sessionInfo).code;
  const signIn = await json(`${IDT}/accounts:signInWithPhoneNumber?key=k`, {
    method: "POST",
    body: JSON.stringify({ sessionInfo: sent.body.sessionInfo, code }),
  });
  return signIn.body.idToken;
}

const api = (path, token, init = {}) =>
  json(`${API}${path}`, { ...init, headers: { authorization: `Bearer ${token}`, ...init.headers } });

const token = await phoneSignIn(`+9195${String(Date.now()).slice(-8)}`);
await api("/v1/auth/sync", token, { method: "POST" });

const svc = await json(`${API}/v1/customer/serviceability?lat=${HOME.lat}&lng=${HOME.lng}`);
check("home is serviceable", svc.body?.serviceable === true, JSON.stringify(svc.body));
const storeId = svc.body.store.id;

const products = await json(`${API}/v1/customer/catalog/products?storeId=${storeId}&q=atta`);
const atta = products.body.items.find((p) => p.inStock);
check("found an in-stock product", !!atta, JSON.stringify(products.body));

const address = await api("/v1/customer/addresses", token, {
  method: "POST",
  body: JSON.stringify({
    label: "Home", line1: "Flat 2A, Test Residency", city: "Bengaluru", pincode: "560038", location: HOME,
  }),
});
check("address saved and served by the store", address.body?.servingStoreId === storeId, JSON.stringify(address.body));

const cart = await api(`/v1/customer/cart/items/${atta.id}`, token, {
  method: "PUT",
  body: JSON.stringify({ storeId, quantity: 2 }),
});
check("cart has 2 items", cart.body?.itemCount === 2, JSON.stringify(cart.body));

const before = atta.maxQuantity;
const key = `e2e-${Date.now()}`;
const order = await api("/v1/customer/checkout", token, {
  method: "POST",
  headers: { "idempotency-key": key },
  body: JSON.stringify({ storeId, addressId: address.body.id, paymentMethod: "COD" }),
});
check("COD checkout confirms", order.status === 201 && order.body.order.status === "CONFIRMED", JSON.stringify(order.body));
check("order has a 4-digit OTP", /^\d{4}$/.test(order.body?.order?.deliveryOtp ?? ""));
const retry = await api("/v1/customer/checkout", token, {
  method: "POST",
  headers: { "idempotency-key": key },
  body: JSON.stringify({ storeId, addressId: address.body.id, paymentMethod: "COD" }),
});
check("retry with same key returns same order", retry.body?.order?.id === order.body.order.id);

const emptied = await api(`/v1/customer/cart?storeId=${storeId}`, token);
check("cart emptied after order", emptied.body?.items?.length === 0);

const cancelled = await api(`/v1/customer/orders/${order.body.order.id}/cancel`, token, {
  method: "POST",
  body: JSON.stringify({ reason: "e2e" }),
});
check("customer can cancel before packing", cancelled.body?.status === "CANCELLED", JSON.stringify(cancelled.body));

const after = await json(`${API}/v1/customer/catalog/products/${atta.id}?storeId=${storeId}`);
check("stock restored after cancel", after.body?.maxQuantity === before, `${before} -> ${after.body?.maxQuantity}`);

if (failures) {
  console.error(`\n${failures} check(s) failed`);
  process.exit(1);
}
console.log("\ne2e orders passed");
