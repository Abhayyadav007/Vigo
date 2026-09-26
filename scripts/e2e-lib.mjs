// Shared helpers for the e2e-*.mjs scripts (Firebase Auth emulator + backend).

import { execFileSync } from "node:child_process";
import { randomInt } from "node:crypto";

export const API = process.argv[2] ?? "http://localhost:8080";
const EMU = `http://${process.env.FIREBASE_AUTH_EMULATOR_HOST ?? "localhost:9099"}`;
const PROJECT = process.env.FIREBASE_PROJECT_ID ?? "demo-vigo";
const IDT = `${EMU}/identitytoolkit.googleapis.com/v1`;

let failures = 0;
export function check(name, ok, detail = "") {
  console.log(`${ok ? "ok  " : "FAIL"} ${name}${ok ? "" : `  ${detail}`}`);
  if (!ok) failures++;
}

/** Exits non-zero if any check failed. */
export function finish(passedMessage) {
  if (failures) {
    console.error(`\n${failures} check(s) failed`);
    process.exit(1);
  }
  console.log(`\n${passedMessage}`);
}

export async function json(url, init = {}) {
  const res = await fetch(url, { ...init, headers: { "content-type": "application/json", ...init.headers } });
  const text = await res.text();
  return { status: res.status, body: text ? JSON.parse(text) : null };
}

export const api = (path, token, init = {}) =>
  json(`${API}${path}`, { ...init, headers: { authorization: `Bearer ${token}`, ...init.headers } });

/** A random Indian mobile, `+91<prefix><8 digits>`. */
export const phone = (prefix) => `+91${prefix}${String(randomInt(0, 1e8)).padStart(8, "0")}`;

/** Phone OTP sign-in exactly as the client SDKs do it, reading the OTP from the emulator. */
export async function firebaseSignIn(phoneNumber) {
  const sent = await json(`${IDT}/accounts:sendVerificationCode?key=k`, {
    method: "POST",
    body: JSON.stringify({ phoneNumber, recaptchaToken: "ignored-by-emulator" }),
  });
  if (sent.status !== 200) throw new Error(`sendVerificationCode: ${JSON.stringify(sent.body)}`);
  const codes = await json(`${EMU}/emulator/v1/projects/${PROJECT}/verificationCodes`);
  const entry = codes.body.verificationCodes.find((c) => c.sessionInfo === sent.body.sessionInfo);
  if (!entry) throw new Error("OTP not found in emulator");
  const res = await json(`${IDT}/accounts:signInWithPhoneNumber?key=k`, {
    method: "POST",
    body: JSON.stringify({ sessionInfo: sent.body.sessionInfo, code: entry.code }),
  });
  if (res.status !== 200) throw new Error(`signInWithPhoneNumber: ${JSON.stringify(res.body)}`);
  return res.body.idToken;
}

/** Firebase sign-in + backend sync; returns `{ token, me, phone }`. */
export async function signIn(phoneNumber) {
  const token = await firebaseSignIn(phoneNumber);
  const me = await api("/v1/auth/sync", token, { method: "POST" });
  return { token, me: me.body, phone: phoneNumber };
}

/** Makes a signed-in user ADMIN via the backend CLI (PROMOTE_CMD). */
export function promoteAdmin(phoneNumber) {
  const cmd = (process.env.PROMOTE_CMD ?? "cargo run -q -p backend -- promote-admin").split(" ");
  execFileSync(cmd[0], [...cmd.slice(1), phoneNumber], { stdio: "ignore" });
}
