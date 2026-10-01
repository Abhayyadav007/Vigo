import { describeAuthError, formatIndianPhone, toIndianE164, useAuth } from "@vigo/api-client";
import type { PhoneVerification } from "@vigo/api-client/native";
import { PhoneLoginForm, Screen } from "@vigo/ui";
import { useRef } from "react";
import { Text, View } from "react-native";
import { firebaseAuth } from "../lib/api";

export default function Login() {
  const { state } = useAuth();
  const verification = useRef<PhoneVerification | null>(null);

  return (
    <Screen scroll className="justify-center">
      <View className="items-center gap-3 rounded-lg bg-brand-light px-6 py-8">
        <View className="h-16 w-16 items-center justify-center rounded-lg bg-brand">
          <Text className="text-3xl font-extrabold text-background">v</Text>
        </View>
        <Text className="text-center text-2xl font-extrabold text-ink">Groceries and essentials in minutes</Text>
        <View className="flex-row flex-wrap justify-center gap-2">
          {["10-minute delivery", "Best prices", "Fresh every day"].map((t) => (
            <View key={t} className="rounded-pill bg-surface px-3 py-1">
              <Text className="text-xs font-semibold text-ink">{t}</Text>
            </View>
          ))}
        </View>
      </View>
      <PhoneLoginForm
        title="Log in or sign up"
        subtitle="We'll send a one-time code to your mobile number."
        size="md"
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
