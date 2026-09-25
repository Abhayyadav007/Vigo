/** Vigo is India-only: +91 followed by a 10-digit mobile number starting 6-9. */
const INDIAN_MOBILE = /^[6-9]\d{9}$/;

/** Normalises user input like "98765 43210" or "+91 98765-43210" to E.164, or null. */
export function toIndianE164(input: string): string | null {
  let digits = input.replace(/\D/g, "");
  if (digits.length === 12 && digits.startsWith("91")) digits = digits.slice(2);
  if (digits.length === 11 && digits.startsWith("0")) digits = digits.slice(1);
  return INDIAN_MOBILE.test(digits) ? `+91${digits}` : null;
}

/** "+919876543210" -> "+91 98765 43210" */
export function formatIndianPhone(e164: string): string {
  const m = /^\+91(\d{5})(\d{5})$/.exec(e164);
  return m ? `+91 ${m[1]} ${m[2]}` : e164;
}

/** Maps Firebase Auth error codes (shared by the web and native SDKs) to user-facing text. */
export function describeAuthError(err: unknown): string {
  const code = typeof err === "object" && err !== null && "code" in err ? String(err.code) : "";
  switch (code) {
    case "auth/invalid-phone-number":
      return "That phone number doesn't look right.";
    case "auth/invalid-verification-code":
      return "Incorrect code. Check the SMS and try again.";
    case "auth/code-expired":
    case "auth/session-expired":
      return "The code has expired. Request a new one.";
    case "auth/too-many-requests":
    case "auth/quota-exceeded":
      return "Too many attempts. Please wait a few minutes and try again.";
    case "auth/network-request-failed":
      return "No internet connection.";
    case "auth/captcha-check-failed":
      return "Verification failed. Please try again.";
    default:
      return err instanceof Error && err.message ? err.message : "Something went wrong. Please try again.";
  }
}
