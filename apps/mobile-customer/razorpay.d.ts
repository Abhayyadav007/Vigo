// react-native-razorpay ships no types; this covers the part the app uses.
declare module "react-native-razorpay" {
  export interface RazorpayOptions {
    key: string;
    order_id: string;
    /** Paise. */
    amount: number;
    currency: string;
    name: string;
    description?: string;
    prefill?: { contact?: string; email?: string; name?: string };
    theme?: { color?: string };
  }
  export interface RazorpaySuccess {
    razorpay_payment_id: string;
    razorpay_order_id: string;
    razorpay_signature: string;
  }
  const RazorpayCheckout: {
    /** Resolves on success; rejects with `{ code, description }` on failure or dismissal. */
    open(options: RazorpayOptions): Promise<RazorpaySuccess>;
  };
  export default RazorpayCheckout;
}
