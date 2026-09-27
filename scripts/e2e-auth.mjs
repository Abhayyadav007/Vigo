#!/usr/bin/env node
// End-to-end auth check: Firebase Auth emulator phone OTP -> backend.
//
// Needs the emulator (`pnpm emulators`) and a backend started with
//   FIREBASE_AUTH_EMULATOR_HOST=localhost:9099 FIREBASE_PROJECT_ID=demo-vigo
// Usage: node scripts/e2e-auth.mjs [backend_url]

import { api, check, finish, firebaseSignIn, promoteAdmin } from "./e2e-lib.mjs";

const suffix = String(Date.now()).slice(-8);
const customerPhone = `+9198${suffix}`;
const adminPhone = `+9197${suffix}`;

const customerToken = await firebaseSignIn(customerPhone);
check("emulator issued an ID token", typeof customerToken === "string");

const sync = await api("/v1/auth/sync", customerToken, { method: "POST" });
check("sync creates CUSTOMER", sync.status === 200 && sync.body.role === "CUSTOMER", JSON.stringify(sync));
check("sync stores the verified phone", sync.body?.phone === customerPhone, JSON.stringify(sync.body));

const me = await api("/v1/auth/me", customerToken);
check("me returns the same user", me.status === 200 && me.body.id === sync.body.id, JSON.stringify(me));

const forbidden = await api("/v1/admin/users", customerToken);
check("customer is 403 on admin API", forbidden.status === 403, JSON.stringify(forbidden));

const tampered = customerToken.replace(/\.[^.]*\./, `.${Buffer.from(JSON.stringify({ sub: "x" })).toString("base64url")}.`);
const bad = await api("/v1/auth/me", tampered);
check("forged token is 401", bad.status === 401, JSON.stringify(bad));

const adminToken = await firebaseSignIn(adminPhone);
await api("/v1/auth/sync", adminToken, { method: "POST" });
if (process.env.PROMOTE_CMD !== "skip") {
  promoteAdmin(adminPhone);
  const list = await api(`/v1/admin/users?phone=${suffix}`, adminToken);
  check("promoted admin can list users", list.status === 200 && list.body.total === 2, JSON.stringify(list));

  // Riders belong to a store (the demo seed provides some).
  const stores = await api("/v1/admin/stores?limit=1", adminToken);
  const storeId = stores.body.items?.[0]?.id;
  check("a store exists to assign the rider to", !!storeId, "run `backend seed-demo` first");
  const noStore = await api(`/v1/admin/users/${sync.body.id}/role`, adminToken, {
    method: "PATCH",
    body: JSON.stringify({ role: "RIDER" }),
  });
  check("RIDER without a store is rejected", noStore.status === 422, JSON.stringify(noStore));
  const role = await api(`/v1/admin/users/${sync.body.id}/role`, adminToken, {
    method: "PATCH",
    body: JSON.stringify({ role: "RIDER", storeId }),
  });
  check("admin assigns RIDER", role.status === 200 && role.body.role === "RIDER", JSON.stringify(role));
  const after = await api("/v1/auth/me", customerToken);
  check("new role visible immediately", after.body?.role === "RIDER", JSON.stringify(after));
}

finish("e2e auth passed");
