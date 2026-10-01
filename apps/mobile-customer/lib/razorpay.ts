import type { RazorpayCheckout as Checkout } from "@vigo/types";
import RazorpayCheckout from "react-native-razorpay";
import { colors } from "./theme";

export type PaymentOutcome =
  | {
      ok: true;
      razorpayOrderId: string;
      razorpayPaymentId: string;
      razorpaySignature: string;
    }
  | { ok: false; message: string };

/** Opens Razorpay's native checkout (UPI apps, cards, netbanking) for a gateway order. */
export async function payWithRazorpay(checkout: Checkout, phone: string): Promise<PaymentOutcome> {
  try {
    const res = await RazorpayCheckout.open({
      key: checkout.keyId,
      order_id: checkout.gatewayOrderId,
      amount: checkout.amountPaise,
      currency: checkout.currency,
      name: "Vigo",
      description: "Groceries delivered in minutes",
      prefill: { contact: phone },
      theme: { color: colors.brand },
    });
    return {
      ok: true,
      razorpayOrderId: res.razorpay_order_id,
      razorpayPaymentId: res.razorpay_payment_id,
      razorpaySignature: res.razorpay_signature,
    };
  } catch (e: unknown) {
    const description =
      typeof e === "object" && e !== null && "description" in e && typeof e.description === "string"
        ? e.description
        : undefined;
    return { ok: false, message: description ?? "Payment was not completed." };
  }
}
