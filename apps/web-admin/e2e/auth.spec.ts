import { expect, test } from "@playwright/test";
import { apiGet, backendCli, signInViaUi, signUpViaApi, uniquePhone } from "./helpers";

test("non-admins are turned away; admins can sign in and assign roles", async ({ page }) => {
  const adminDigits = uniquePhone("97");
  const customerPhone = `+91${uniquePhone("96")}`;
  const customerToken = await signUpViaApi(customerPhone);

  // 1. A brand-new account is a CUSTOMER, so the admin app signs it out.
  await signInViaUi(page, adminDigits);
  await expect(page.getByRole("alert")).toContainText("This app is for admin accounts");
  await expect(page.getByLabel("Mobile number")).toBeVisible();

  // 2. Bootstrap them to ADMIN, then sign in again.
  backendCli("promote-admin", `+91${adminDigits}`);
  await signInViaUi(page, adminDigits);
  await expect(page.getByTestId("signed-in-as")).toContainText(adminDigits.slice(5));
  await expect(page.locator(".pill-ok").first()).toBeVisible();

  // 3. Staff page: find the customer and make them a RIDER at a store.
  await page.getByRole("link", { name: "Staff" }).click();
  await page.getByPlaceholder("Search phone").fill(customerPhone.slice(3));
  const row = page.getByTestId(`user-${customerPhone}`);
  await expect(row).toBeVisible();
  const roleSelect = row.getByLabel(`Role for ${customerPhone}`);
  await expect(roleSelect).toHaveValue("CUSTOMER");
  await roleSelect.selectOption("RIDER");
  const save = row.getByRole("button", { name: "Save" });
  await expect(save).toBeDisabled(); // riders need a store
  await row.getByLabel(`Store for ${customerPhone}`).selectOption({ label: "BLR-IND-01" });
  await save.click();
  await expect(save).toBeHidden();

  // 4. The backend sees the new role immediately (session cache invalidated).
  const me = await apiGet<{ role: string; storeId: string | null }>("/v1/auth/me", customerToken);
  expect(me.role).toBe("RIDER");
  expect(me.storeId).not.toBeNull();

  // 5. Admins can't demote themselves.
  await page.getByPlaceholder("Search phone").fill(adminDigits);
  await expect(page.getByTestId(`user-+91${adminDigits}`).getByRole("combobox").first()).toBeDisabled();

  // 6. Sign out returns to the login screen.
  await page.getByRole("button", { name: "Sign out" }).click();
  await expect(page.getByLabel("Mobile number")).toBeVisible();
});
