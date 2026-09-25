// Money is integer paise everywhere; these helpers are the only place rupees
// appear, and never go through floating point.

/** 123456 -> "₹1,234.56"; 5000 -> "₹50" (Indian digit grouping: 12,34,567). */
export function formatPaise(paise: number): string {
  const negative = paise < 0;
  const abs = Math.abs(Math.trunc(paise));
  const rupees = Math.floor(abs / 100).toString();
  const rest = abs % 100;
  const last3 = rupees.slice(-3);
  const head = rupees.slice(0, -3).replace(/\B(?=(\d{2})+(?!\d))/g, ",");
  const grouped = head ? `${head},${last3}` : last3;
  return `${negative ? "-" : ""}₹${grouped}${rest ? `.${rest.toString().padStart(2, "0")}` : ""}`;
}

/** "1,234.5" / "₹99" -> 123450 / 9900; null if not a valid amount. */
export function rupeesToPaise(input: string): number | null {
  const m = /^(\d{1,9})(?:\.(\d{1,2}))?$/.exec(input.replace(/[₹,\s]/g, ""));
  if (!m) return null;
  return Number(m[1]) * 100 + Number((m[2] ?? "").padEnd(2, "0"));
}

/** 123450 -> "1234.50", for editable inputs. */
export function paiseToRupeesInput(paise: number): string {
  const rest = paise % 100;
  return `${Math.floor(paise / 100)}${rest ? `.${rest.toString().padStart(2, "0")}` : ""}`;
}

/** Whole-percent discount of price vs MRP, or 0. */
export function discountPercent(mrpPaise: number, pricePaise: number): number {
  return mrpPaise > pricePaise ? Math.floor(((mrpPaise - pricePaise) * 100) / mrpPaise) : 0;
}
