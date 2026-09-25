import { execFileSync } from "node:child_process";
import { randomInt } from "node:crypto";
import { expect, type Page } from "@playwright/test";

export const EMULATOR = `http://${process.env.FIREBASE_AUTH_EMULATOR_HOST ?? "localhost:9099"}`;
export const PROJECT = process.env.FIREBASE_PROJECT_ID ?? "demo-vigo";
export const API = process.env.VITE_API_URL ?? "http://localhost:8080";
// The backend CLI (promote-admin, seed-demo), run from the repo root.
const BACKEND = (process.env.BACKEND_CLI ?? "cargo run -q -p backend --").split(" ");

export function backendCli(...args: string[]) {
  execFileSync(BACKEND[0]!, [...BACKEND.slice(1), ...args], { cwd: "../..", stdio: "inherit" });
}

/**
 * A random 10-digit Indian mobile starting with `prefix`. Random, not
 * time-based: tests run in parallel and must never share a number (they'd
 * read each other's OTPs).
 */
export const uniquePhone = (prefix: string) => `${prefix}${randomInt(0, 100_000_000).toString().padStart(8, "0")}`;

/** Newest OTP the emulator "sent" to this number. */
export async function latestOtp(phoneE164: string): Promise<string> {
  const res = await fetch(`${EMULATOR}/emulator/v1/projects/${PROJECT}/verificationCodes`);
  const { verificationCodes } = (await res.json()) as {
    verificationCodes: { phoneNumber: string; code: string }[];
  };
  const code = verificationCodes.filter((c) => c.phoneNumber === phoneE164).at(-1)?.code;
  if (!code) throw new Error(`no OTP for ${phoneE164}`);
  return code;
}

export async function signInViaUi(page: Page, tenDigits: string) {
  await page.goto("/");
  await page.getByLabel("Mobile number").fill(tenDigits);
  await page.getByRole("button", { name: "Send OTP" }).click();
  await expect(page.getByLabel("OTP")).toBeVisible();
  await page.getByLabel("OTP").fill(await latestOtp(`+91${tenDigits}`));
  await page.getByRole("button", { name: "Verify" }).click();
}

/** Signs in through the UI as a fresh ADMIN (bootstrapped via the CLI). */
export async function signInAsNewAdmin(page: Page): Promise<string> {
  const digits = uniquePhone("97");
  const phone = `+91${digits}`;
  await signUpViaApi(phone);
  backendCli("promote-admin", phone);
  await signInViaUi(page, digits);
  await expect(page.getByTestId("signed-in-as")).toBeVisible();
  return digits;
}

/** Signs a user up through the emulator REST API + backend sync (no UI); returns their ID token. */
export async function signUpViaApi(phoneE164: string): Promise<string> {
  const idt = `${EMULATOR}/identitytoolkit.googleapis.com/v1`;
  const post = (url: string, body: unknown) =>
    fetch(url, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) }).then(
      (r) => r.json() as Promise<Record<string, string>>,
    );
  const { sessionInfo } = await post(`${idt}/accounts:sendVerificationCode?key=k`, {
    phoneNumber: phoneE164,
    recaptchaToken: "x",
  });
  const { idToken } = await post(`${idt}/accounts:signInWithPhoneNumber?key=k`, {
    sessionInfo,
    code: await latestOtp(phoneE164),
  });
  const sync = await fetch(`${API}/v1/auth/sync`, { method: "POST", headers: { authorization: `Bearer ${idToken}` } });
  expect(sync.status).toBe(200);
  return idToken!;
}

export async function apiGet<T>(path: string, token?: string): Promise<T> {
  const res = await fetch(`${API}${path}`, token ? { headers: { authorization: `Bearer ${token}` } } : {});
  expect(res.status, `GET ${path}`).toBe(200);
  return (await res.json()) as T;
}
