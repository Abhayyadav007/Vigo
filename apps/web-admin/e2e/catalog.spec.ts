import { expect, test } from "@playwright/test";
import type { CatalogProduct, Page as ApiPage, ServiceabilityResponse } from "@vigo/types";
import { apiGet, signInAsNewAdmin } from "./helpers";

// A 1x1 PNG, uploaded through the product image picker.
const PNG = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
  "base64",
);

test("admin sets up a store, category, product and stock; customers there see it", async ({ page }) => {
  await signInAsNewAdmin(page);
  const tag = String(Date.now()).slice(-6);
  // Somewhere in Pune, away from the Bengaluru demo stores.
  const lat = 18.52 + Number(tag.slice(-3)) / 100_000;
  const lng = 73.8567;

  // Store: coordinates + generated service area.
  await page.getByRole("link", { name: "Stores" }).click();
  await page.getByRole("link", { name: "New store" }).click();
  await page.getByLabel("Code").fill(`PNQ-E2E-${tag}`);
  await page.getByLabel("Name").fill(`E2E Store ${tag}`);
  await page.getByLabel("Address").fill("FC Road, Pune");
  await page.getByLabel("Latitude").fill(String(lat));
  await page.getByLabel("Longitude").fill(String(lng));
  await page.getByLabel("Quick area (km radius)").fill("2");
  await page.getByRole("button", { name: "Generate area" }).click();
  await expect(page.locator(".leaflet-interactive").first()).toBeVisible();
  await page.getByRole("button", { name: "Save store" }).click();
  await expect(page.getByRole("link", { name: `PNQ-E2E-${tag}` })).toBeVisible();

  // Category.
  await page.getByRole("link", { name: "Categories" }).click();
  await page.getByLabel("Name").fill(`E2E Dairy ${tag}`);
  await page.getByRole("button", { name: "Add category" }).click();
  await expect(page.getByRole("button", { name: `E2E Dairy ${tag}` })).toBeVisible();

  // Product with an image; price above MRP is caught client-side first.
  await page.getByRole("link", { name: "Products" }).click();
  await page.getByRole("link", { name: "New product" }).click();
  await page.getByLabel("Name").fill(`E2E Paneer ${tag}`);
  await page.getByLabel("Brand").fill("Amul");
  await page.getByLabel("Category").selectOption({ label: `E2E Dairy ${tag}` });
  await page.getByLabel("Pack size").fill("200 g");
  await page.getByLabel("MRP").fill("95");
  await page.getByLabel("Selling price").fill("99");
  await page.getByRole("button", { name: "Save product" }).click();
  await expect(page.getByText("Selling price can't be above MRP.")).toBeVisible();
  await page.getByLabel("Selling price").fill("89.50");
  await page.getByLabel("Upload image").setInputFiles({ name: "paneer.png", mimeType: "image/png", buffer: PNG });
  await expect(page.getByAltText("Product image 1")).toBeVisible();
  await page.getByRole("button", { name: "Save product" }).click();
  await expect(page.getByRole("link", { name: `E2E Paneer ${tag}` })).toBeVisible();

  // Stock it at the new store with a store price.
  await page.getByRole("link", { name: "Inventory" }).click();
  await page.getByLabel("Store").selectOption({ label: `PNQ-E2E-${tag} · E2E Store ${tag}` });
  await page.getByPlaceholder("Search product, barcode or bin").fill(`E2E Paneer ${tag}`);
  const row = page.getByTestId(`inv-E2E Paneer ${tag}`);
  await row.getByLabel("Bin").fill("C-01-2");
  await row.getByLabel("Quantity").fill("25");
  await row.getByLabel("Store price").fill("85");
  await row.getByRole("button", { name: "Stock" }).click();
  await expect(row.getByRole("button", { name: "Save" })).toBeDisabled();

  // A customer at that location is served by the new store and sees the product.
  const svc = await apiGet<ServiceabilityResponse>(`/v1/customer/serviceability?lat=${lat}&lng=${lng}`);
  expect(svc.store?.code).toBe(`PNQ-E2E-${tag}`);
  const page1 = await apiGet<ApiPage<CatalogProduct>>(
    `/v1/customer/catalog/products?storeId=${svc.store!.id}&q=paneer`,
  );
  const product = page1.items.find((p) => p.name === `E2E Paneer ${tag}`);
  expect(product).toMatchObject({ pricePaise: 8500, mrpPaise: 9500, inStock: true, maxQuantity: 10 });
  expect(product!.imageUrls[0]).toMatch(/^\/media\/.+\.png$/);

  // The uploaded image is actually served.
  const img = await fetch(`${process.env.VITE_API_URL ?? "http://localhost:8080"}${product!.imageUrls[0]}`);
  expect(img.headers.get("content-type")).toBe("image/png");
});
