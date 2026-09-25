import { strict as assert } from "node:assert";
import { test } from "node:test";
import { discountPercent, formatPaise, paiseToRupeesInput, rupeesToPaise } from "./money.ts";
import { resolveMediaUrl } from "./media.ts";
import { toIndianE164 } from "./phone.ts";

void test("formatPaise uses Indian grouping and drops .00", () => {
  assert.equal(formatPaise(0), "₹0");
  assert.equal(formatPaise(5_000), "₹50");
  assert.equal(formatPaise(12_345), "₹123.45");
  assert.equal(formatPaise(100_000), "₹1,000");
  assert.equal(formatPaise(123_456_700), "₹12,34,567");
  assert.equal(formatPaise(10_000_000_05), "₹1,00,00,000.05");
  assert.equal(formatPaise(-2_550), "-₹25.50");
});

void test("rupeesToPaise parses without floats", () => {
  assert.equal(rupeesToPaise("0.29"), 29);
  assert.equal(rupeesToPaise("1,234.5"), 123_450);
  assert.equal(rupeesToPaise("₹ 99"), 9_900);
  assert.equal(rupeesToPaise("19.99"), 1_999);
  for (const bad of ["", "abc", "1.234", "-5", "1e3"]) assert.equal(rupeesToPaise(bad), null, bad);
  assert.equal(paiseToRupeesInput(123_450), "1234.50");
  assert.equal(paiseToRupeesInput(9_900), "99");
});

void test("discountPercent floors", () => {
  assert.equal(discountPercent(2_800, 2_600), 7);
  assert.equal(discountPercent(1_000, 1_000), 0);
  assert.equal(discountPercent(1_000, 1_200), 0);
});

void test("resolveMediaUrl", () => {
  assert.equal(resolveMediaUrl("/media/a.png", "http://192.168.1.5:8080/"), "http://192.168.1.5:8080/media/a.png");
  assert.equal(resolveMediaUrl("https://cdn.x/a.png", "http://h"), "https://cdn.x/a.png");
  assert.equal(resolveMediaUrl(null, "http://h"), undefined);
});

void test("toIndianE164", () => {
  assert.equal(toIndianE164("98765 43210"), "+919876543210");
  assert.equal(toIndianE164("+91-98765-43210"), "+919876543210");
  assert.equal(toIndianE164("098765 43210"), "+919876543210");
  assert.equal(toIndianE164("5876543210"), null);
  assert.equal(toIndianE164("12345"), null);
});
