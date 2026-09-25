import { describeAuthError, formatIndianPhone, toIndianE164 } from "@vigo/api-client";
import type { PhoneVerification } from "@vigo/api-client/web";
import { useRef, useState, type FormEvent } from "react";
import { firebaseAuth } from "../lib/api";

export function Login({ error: externalError }: { error?: string | undefined }) {
  const [phoneInput, setPhoneInput] = useState("");
  const [phone, setPhone] = useState<string | null>(null);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | undefined>();
  const verification = useRef<PhoneVerification | null>(null);
  const recaptcha = useRef<HTMLDivElement>(null);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(undefined);
    try {
      await fn();
    } catch (err) {
      setError(describeAuthError(err));
    } finally {
      setBusy(false);
    }
  };

  const sendCode = (e: FormEvent) => {
    e.preventDefault();
    void run(async () => {
      const e164 = toIndianE164(phoneInput);
      if (!e164) return setError("Enter a valid 10-digit Indian mobile number.");
      if (!recaptcha.current) return;
      verification.current = await firebaseAuth.sendOtp(e164, recaptcha.current);
      setPhone(e164);
    });
  };

  const verify = (e: FormEvent) => {
    e.preventDefault();
    void run(async () => {
      if (!/^\d{6}$/.test(code)) return setError("Enter the 6-digit code.");
      await verification.current?.confirm(code);
    });
  };

  const shownError = error ?? externalError;

  return (
    <div className="center">
      <div className="card login">
        <h1>Vigo Admin</h1>
        {phone === null ? (
          <form onSubmit={sendCode}>
            <label htmlFor="phone">Mobile number</label>
            <div className="input-group">
              <span>+91</span>
              <input
                id="phone"
                autoFocus
                inputMode="tel"
                autoComplete="tel"
                placeholder="98765 43210"
                value={phoneInput}
                onChange={(e) => setPhoneInput(e.target.value)}
              />
            </div>
            <button className="btn" disabled={busy}>
              {busy ? "Sending…" : "Send OTP"}
            </button>
          </form>
        ) : (
          <form onSubmit={verify}>
            <p>
              Code sent to <strong>{formatIndianPhone(phone)}</strong>
            </p>
            <label htmlFor="code">OTP</label>
            <input
              id="code"
              autoFocus
              inputMode="numeric"
              autoComplete="one-time-code"
              maxLength={6}
              placeholder="123456"
              value={code}
              onChange={(e) => setCode(e.target.value)}
            />
            <button className="btn" disabled={busy}>
              {busy ? "Verifying…" : "Verify"}
            </button>
            <button type="button" className="btn ghost" onClick={() => setPhone(null)} disabled={busy}>
              Change number
            </button>
          </form>
        )}
        {shownError ? (
          <p className="error" role="alert">
            {shownError}
          </p>
        ) : null}
        <div ref={recaptcha} />
      </div>
    </div>
  );
}
