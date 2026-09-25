export { ApiError, createApiClient, type ApiClientOptions } from "./client";
export { ApiClientProvider, useApiBaseUrl, useApiClient } from "./context";
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
export { discountPercent, formatPaise, paiseToRupeesInput, rupeesToPaise } from "./money";
export { resolveMediaUrl } from "./media";
export { healthKeys, useHealth } from "./hooks/useHealth";
export { adminUserKeys, useAdminUsers, useUpdateUserRole } from "./hooks/useAdminUsers";
export {
  adminCatalogKeys,
  useCategories,
  useInventory,
  useProduct,
  useProducts,
  useSaveCategory,
  useSaveProduct,
  useSaveStore,
  useSetInventory,
  useStore,
  useStores,
  useUploadImage,
} from "./hooks/useAdminCatalog";
export {
  catalogKeys,
  useCatalogCategories,
  useCatalogProduct,
  useCatalogProducts,
  useServiceability,
} from "./hooks/useCatalog";
