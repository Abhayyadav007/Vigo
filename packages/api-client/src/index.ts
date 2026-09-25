export { ApiError, createApiClient, type ApiClientOptions } from "./client";
export { ApiClientProvider, useApiClient } from "./context";
export {
  AuthProvider,
  useAuth,
  useCurrentUser,
  type AuthAdapter,
  type AuthContextValue,
  type AuthProviderProps,
  type AuthState,
} from "./auth-context";
export { describeAuthError, formatIndianPhone, toIndianE164 } from "./phone";
export { healthKeys, useHealth } from "./hooks/useHealth";
export { adminUserKeys, useAdminUsers, useUpdateUserRole } from "./hooks/useAdminUsers";
