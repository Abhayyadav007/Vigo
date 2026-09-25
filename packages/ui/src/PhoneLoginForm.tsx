import { useRef, useState } from "react";
import { Text, View, type TextInput } from "react-native";
import { Button } from "./Button";
import { TextField } from "./TextField";

export interface PhoneLoginFormProps {
  title: string;
  subtitle?: string;
  /**
   * Validates/normalises the 10-digit input; return the E.164 number or null.
   * Kept as a prop so this package stays free of API/Firebase code.
   */
  normalizePhone: (input: string) => string | null;
  formatPhone?: (e164: string) => string;
  /** Sends the OTP. Throw to show an error. */
  onSendCode: (phoneE164: string) => Promise<void>;
  /** Confirms the OTP. Throw to show an error. */
  onConfirmCode: (code: string) => Promise<void>;
  /** Turns thrown errors into user-facing text. */
  describeError: (err: unknown) => string;
  /** Error from outside the form, e.g. "this app is for riders". */
  externalError?: string | undefined;
  size?: "md" | "lg";
}

type Step = { kind: "phone" } | { kind: "code"; phone: string };

export function PhoneLoginForm(props: PhoneLoginFormProps) {
  const { title, subtitle, normalizePhone, formatPhone = (p) => p, describeError, size = "md" } = props;
  const [step, setStep] = useState<Step>({ kind: "phone" });
  const [phoneInput, setPhoneInput] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | undefined>();
  const codeRef = useRef<TextInput>(null);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setError(undefined);
    try {
      await fn();
    } catch (err) {
      setError(describeError(err));
    } finally {
      setBusy(false);
    }
  };

  const sendCode = () =>
    run(async () => {
      const phone = normalizePhone(phoneInput);
      if (!phone) {
        setError("Enter a valid 10-digit Indian mobile number.");
        return;
      }
      await props.onSendCode(phone);
      setCode("");
      setStep({ kind: "code", phone });
      setTimeout(() => codeRef.current?.focus(), 50);
    });

  const confirm = () =>
    run(async () => {
      if (!/^\d{6}$/.test(code)) {
        setError("Enter the 6-digit code.");
        return;
      }
      await props.onConfirmCode(code);
    });

  const shownError = error ?? props.externalError;

  return (
    <View className="gap-6">
      <View className="gap-2">
        <Text className="text-3xl font-bold text-brand">{title}</Text>
        {subtitle ? <Text className="text-base text-muted">{subtitle}</Text> : null}
      </View>

      {step.kind === "phone" ? (
        <>
          <TextField
            testID="phone-input"
            label="Mobile number"
            prefix="+91"
            placeholder="98765 43210"
            keyboardType="phone-pad"
            textContentType="telephoneNumber"
            autoComplete="tel"
            maxLength={14}
            value={phoneInput}
            onChangeText={setPhoneInput}
            onSubmitEditing={sendCode}
            error={shownError}
          />
          <Button testID="send-code" title="Send OTP" onPress={sendCode} loading={busy} size={size} />
        </>
      ) : (
        <>
          <Text className="text-base text-ink">
            Enter the 6-digit code sent to <Text className="font-semibold">{formatPhone(step.phone)}</Text>
          </Text>
          <TextField
            ref={codeRef}
            testID="code-input"
            label="OTP"
            placeholder="123456"
            keyboardType="number-pad"
            textContentType="oneTimeCode"
            autoComplete="sms-otp"
            maxLength={6}
            value={code}
            onChangeText={setCode}
            onSubmitEditing={confirm}
            error={shownError}
          />
          <Button testID="verify-code" title="Verify" onPress={confirm} loading={busy} size={size} />
          <Button
            title="Change number"
            variant="ghost"
            onPress={() => {
              setError(undefined);
              setStep({ kind: "phone" });
            }}
            disabled={busy}
          />
        </>
      )}
    </View>
  );
}
