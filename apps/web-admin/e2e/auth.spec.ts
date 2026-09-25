import { execFileSync } from "node:child_process";
import { expect, test, type Page } from "@playwright/test";

const EMULATOR = `http://${process.env.FIREBASE_AUTH_EMULATOR_HOST ?? "localhost:9099"}`;
const PROJECT = process.env.FIREBASE_PROJECT_ID ?? "demo-vigo";
const API = process.env.VITE_API_URL ?? "http://localhost:8080";
// How to promote a user to ADMIN (the bootstrap CLI). Runs from the repo root.
const PROMOTE = (process.env.PROMOTE_CMD ?? "cargo run -q -p backend -- promote-admin").split(" ");

const uniquePhone = (prefix: string) => `${prefix}${String(Date.now()).slice(-8)}`;

/** Newest OTP the emulator "sent" to this number. */
async function latestOtp(phoneE164: string): Promise<string> {
  const res = await fetch(`${EMULATOR}/emulator/v1/projects/${PROJECT}/verificationCodes`);
  const { verificationCodes } = (await res.json()) as {
    verificationCodes: { phoneNumber: string; code: string }[];
  };
  const code = verificationCodes.filter((c) => c.phoneNumber === phoneE164).at(-1)?.code;
  if (!code) throw new Error(`no OTP for ${phoneE164}`);
  return code;
}

async function signInViaUi(page: Page, tenDigits: string) {
  await page.goto("/");
  await page.getByLabel("Mobile number").fill(tenDigits);
  await page.getByRole("button", { name: "Send OTP" }).click();
  await expect(page.getByLabel("OTP")).toBeVisible();
  await page.getByLabel("OTP").fill(await latestOtp(`+91${tenDigits}`));
  await page.getByRole("button", { name: "Verify" }).click();
}

/** Signs a user up through the emulator REST API + backend sync (no UI). */
async function signUpCustomer(phoneE164: string): Promise<string> {
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

test("non-admins are turned away; admins can sign in and assign roles", async ({ page }) => {
  const adminDigits = uniquePhone("97");
  const customerPhone = `+91${uniquePhone("96")}`;
  const customerToken = await signUpCustomer(customerPhone);

  // 1. A brand-new account is a CUSTOMER, so the admin app signs it out.
  await signInViaUi(page, adminDigits);
  await expect(page.getByRole("alert")).toContainText("This app is for admin accounts");
  await expect(page.getByLabel("Mobile number")).toBeVisible();

  // 2. Bootstrap them to ADMIN, then sign in again.
  execFileSync(PROMOTE[0]!, [...PROMOTE.slice(1), `+91${adminDigits}`], { cwd: "../..", stdio: "inherit" });
  await signInViaUi(page, adminDigits);
  await expect(page.getByTestId("signed-in-as")).toContainText(adminDigits.slice(5));
  await expect(page.locator(".pill-ok").first()).toBeVisible();

  // 3. Staff page: find the customer and make them a RIDER.
  await page.getByRole("button", { name: "Staff" }).click();
  await page.getByPlaceholder("Search phone").fill(customerPhone.slice(3));
  const row = page.getByTestId(`user-${customerPhone}`);
  await expect(row).toBeVisible();
  const roleSelect = row.getByLabel(`Role for ${customerPhone}`);
  await expect(roleSelect).toHaveValue("CUSTOMER");
  await roleSelect.selectOption("RIDER");
  await expect(roleSelect).toHaveValue("RIDER");

  // 4. The backend sees the new role immediately (session cache invalidated).
  const me = await fetch(`${API}/v1/auth/me`, { headers: { authorization: `Bearer ${customerToken}` } });
  expect(((await me.json()) as { role: string }).role).toBe("RIDER");

  // 5. Admins can't demote themselves.
  await page.getByPlaceholder("Search phone").fill(adminDigits);
  await expect(page.getByTestId(`user-+91${adminDigits}`).getByRole("combobox")).toBeDisabled();

  // 6. Sign out returns to the login screen.
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByLabel("Mobile number")).toBeVisible();
});
