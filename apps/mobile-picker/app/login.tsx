import { describeAuthError, formatIndianPhone, toIndianE164, useAuth } from "@vigo/api-client";
import type { PhoneVerification } from "@vigo/api-client/native";
import { PhoneLoginForm, Screen } from "@vigo/ui";
import { useRef } from "react";
import { firebaseAuth } from "../lib/api";

export default function Login() {
  const { state } = useAuth();
  const verification = useRef<PhoneVerification | null>(null);

  return (
    <Screen scroll className="justify-center">
      <PhoneLoginForm
        title="Vigo Picker"
        subtitle="Dark store staff sign-in."
        size="lg"
        normalizePhone={toIndianE164}
        formatPhone={formatIndianPhone}
        describeError={describeAuthError}
        externalError={state.status === "signedOut" ? state.error : undefined}
        onSendCode={async (phone) => {
          verification.current = await firebaseAuth.sendOtp(phone);
        }}
        onConfirmCode={async (code) => {
          if (!verification.current) throw new Error("Request a new code first.");
          // On success Firebase signs in; AuthProvider syncs with the backend
          // and Stack.Protected swaps this screen for the app.
          await verification.current.confirm(code);
        }}
      />
    </Screen>
  );
}
