// React Native adapter over @react-native-firebase/auth (needs a dev build).
import {
  connectAuthEmulator,
  getAuth,
  getIdToken,
  onAuthStateChanged,
  signInWithPhoneNumber,
  signOut,
} from "@react-native-firebase/auth";
import type { AuthAdapter } from "../auth-context";

export interface PhoneVerification {
  confirm: (code: string) => Promise<void>;
}

export interface NativeFirebaseAuth {
  adapter: AuthAdapter;
  /** Sends an SMS OTP to an E.164 number. */
  sendOtp: (phoneE164: string) => Promise<PhoneVerification>;
}

export function createNativeFirebaseAuth(options: { emulatorHost?: string } = {}): NativeFirebaseAuth {
  const auth = getAuth();
  if (options.emulatorHost) connectAuthEmulator(auth, `http://${options.emulatorHost}`);

  return {
    adapter: {
      onAuthStateChanged: (listener) => onAuthStateChanged(auth, (user) => listener(user ? { uid: user.uid } : null)),
      getIdToken: async (forceRefresh = false) => (auth.currentUser ? getIdToken(auth.currentUser, forceRefresh) : null),
      signOut: () => signOut(auth),
    },
    async sendOtp(phoneE164) {
      const confirmation = await signInWithPhoneNumber(auth, phoneE164);
      return {
        async confirm(code) {
          await confirmation.confirm(code);
        },
      };
    },
  };
}
