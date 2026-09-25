// Browser adapter over the Firebase JS SDK (used by web-admin).
import { initializeApp, type FirebaseOptions } from "firebase/app";
import {
  connectAuthEmulator,
  getAuth,
  onAuthStateChanged,
  RecaptchaVerifier,
  signInWithPhoneNumber,
  signOut,
} from "firebase/auth";
import type { AuthAdapter } from "../auth-context";

export interface PhoneVerification {
  confirm: (code: string) => Promise<void>;
}

export interface WebFirebaseAuth {
  adapter: AuthAdapter;
  /** Sends an SMS OTP; `recaptchaContainer` hosts the invisible reCAPTCHA. */
  sendOtp: (phoneE164: string, recaptchaContainer: HTMLElement) => Promise<PhoneVerification>;
}

export function createWebFirebaseAuth(config: FirebaseOptions, options: { emulatorHost?: string } = {}): WebFirebaseAuth {
  const auth = getAuth(initializeApp(config));
  if (options.emulatorHost) connectAuthEmulator(auth, `http://${options.emulatorHost}`, { disableWarnings: true });
  let verifier: RecaptchaVerifier | null = null;

  return {
    adapter: {
      onAuthStateChanged: (listener) => onAuthStateChanged(auth, (user) => listener(user ? { uid: user.uid } : null)),
      getIdToken: async (forceRefresh = false) => (auth.currentUser ? auth.currentUser.getIdToken(forceRefresh) : null),
      signOut: () => signOut(auth),
    },
    async sendOtp(phoneE164, recaptchaContainer) {
      verifier?.clear();
      verifier = new RecaptchaVerifier(auth, recaptchaContainer, { size: "invisible" });
      try {
        const confirmation = await signInWithPhoneNumber(auth, phoneE164, verifier);
        return {
          async confirm(code) {
            await confirmation.confirm(code);
          },
        };
      } catch (err) {
        verifier.clear();
        verifier = null;
        throw err;
      }
    },
  };
}
