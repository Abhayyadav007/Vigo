import type { Role } from "@vigo/types";

/** Everyone who may sign in to the admin panel. Module-level so the array is stable. */
export const BACK_OFFICE_ROLES: readonly Role[] = ["ADMIN", "STORE_MANAGER", "CATALOG_MANAGER", "SUPPORT_AGENT"];

/** Which back-office roles see each page (the backend enforces the same split). */
export const PAGE_ROLES = {
  overview: ["ADMIN", "SUPPORT_AGENT", "STORE_MANAGER"],
  stores: ["ADMIN", "STORE_MANAGER"],
  /** Creating stores and editing their area. */
  storeSetup: ["ADMIN"],
  catalog: ["ADMIN", "CATALOG_MANAGER"],
  inventory: ["ADMIN", "STORE_MANAGER"],
  staff: ["ADMIN"],
} as const satisfies Record<string, readonly Role[]>;

export type Page = keyof typeof PAGE_ROLES;

export const canSee = (role: Role, page: Page) => (PAGE_ROLES[page] as readonly Role[]).includes(role);

/** Roles tied to one store (mirrors `Role::needs_store` in the backend). */
export const needsStore = (role: Role) => role === "PICKER" || role === "RIDER" || role === "STORE_MANAGER";
