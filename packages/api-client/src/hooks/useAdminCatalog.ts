import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type {
  AdminCategory,
  AdminProduct,
  AdminStore,
  CategoryRequest,
  InventoryItem,
  InventoryListQuery,
  InventoryRequest,
  Page,
  ProductListQuery,
  ProductRequest,
  StoreListQuery,
  StoreRequest,
  UploadResponse,
} from "@vigo/types";
import { useApiClient } from "../context";

interface Paging {
  limit?: number;
  offset?: number;
}

export const adminCatalogKeys = {
  stores: ["admin", "stores"] as const,
  store: (id: string) => ["admin", "stores", id] as const,
  categories: ["admin", "categories"] as const,
  products: ["admin", "products"] as const,
  product: (id: string) => ["admin", "products", id] as const,
  inventory: (storeId: string) => ["admin", "inventory", storeId] as const,
};

// ---------- stores ----------

export function useStores(query: StoreListQuery = {}, { limit = 50, offset = 0 }: Paging = {}) {
  const client = useApiClient();
  return useQuery({
    queryKey: [...adminCatalogKeys.stores, query, limit, offset],
    queryFn: async () =>
      (await client.get<Page<AdminStore>>("/v1/admin/stores", { params: { ...query, limit, offset } })).data,
    placeholderData: keepPreviousData,
  });
}

export function useStore(id: string | undefined) {
  const client = useApiClient();
  return useQuery({
    queryKey: adminCatalogKeys.store(id ?? ""),
    queryFn: async () => (await client.get<AdminStore>(`/v1/admin/stores/${id}`)).data,
    enabled: !!id,
  });
}

export function useSaveStore() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, body }: { id?: string | undefined; body: StoreRequest }) =>
      id
        ? (await client.put<AdminStore>(`/v1/admin/stores/${id}`, body)).data
        : (await client.post<AdminStore>("/v1/admin/stores", body)).data,
    onSuccess: () => qc.invalidateQueries({ queryKey: adminCatalogKeys.stores }),
  });
}

// ---------- categories ----------

export function useCategories() {
  const client = useApiClient();
  return useQuery({
    queryKey: adminCatalogKeys.categories,
    queryFn: async () => (await client.get<AdminCategory[]>("/v1/admin/categories")).data,
  });
}

export function useSaveCategory() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, body }: { id?: string | undefined; body: CategoryRequest }) =>
      id
        ? (await client.put<AdminCategory>(`/v1/admin/categories/${id}`, body)).data
        : (await client.post<AdminCategory>("/v1/admin/categories", body)).data,
    onSuccess: () => qc.invalidateQueries({ queryKey: adminCatalogKeys.categories }),
  });
}

// ---------- products ----------

export function useProducts(query: ProductListQuery = {}, { limit = 20, offset = 0 }: Paging = {}) {
  const client = useApiClient();
  return useQuery({
    queryKey: [...adminCatalogKeys.products, query, limit, offset],
    queryFn: async () =>
      (await client.get<Page<AdminProduct>>("/v1/admin/products", { params: { ...query, limit, offset } })).data,
    placeholderData: keepPreviousData,
  });
}

export function useProduct(id: string | undefined) {
  const client = useApiClient();
  return useQuery({
    queryKey: adminCatalogKeys.product(id ?? ""),
    queryFn: async () => (await client.get<AdminProduct>(`/v1/admin/products/${id}`)).data,
    enabled: !!id,
  });
}

export function useSaveProduct() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, body }: { id?: string | undefined; body: ProductRequest }) =>
      id
        ? (await client.put<AdminProduct>(`/v1/admin/products/${id}`, body)).data
        : (await client.post<AdminProduct>("/v1/admin/products", body)).data,
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: adminCatalogKeys.products });
      await qc.invalidateQueries({ queryKey: adminCatalogKeys.categories });
    },
  });
}

// ---------- inventory ----------

export function useInventory(
  storeId: string | undefined,
  query: InventoryListQuery = {},
  { limit = 50, offset = 0 }: Paging = {},
) {
  const client = useApiClient();
  return useQuery({
    queryKey: [...adminCatalogKeys.inventory(storeId ?? ""), query, limit, offset],
    queryFn: async () =>
      (
        await client.get<Page<InventoryItem>>(`/v1/admin/stores/${storeId}/inventory`, {
          params: { ...query, limit, offset },
        })
      ).data,
    enabled: !!storeId,
    placeholderData: keepPreviousData,
  });
}

export function useSetInventory(storeId: string) {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ productId, body }: { productId: string; body: InventoryRequest }) =>
      (await client.put<InventoryItem>(`/v1/admin/stores/${storeId}/inventory/${productId}`, body)).data,
    onSuccess: () => qc.invalidateQueries({ queryKey: adminCatalogKeys.inventory(storeId) }),
  });
}

// ---------- uploads ----------

/** Uploads one image (web `File`/`Blob`); resolves to its URL for `imageUrls`. */
export function useUploadImage() {
  const client = useApiClient();
  return useMutation({
    mutationFn: async (file: Blob) => {
      const form = new FormData();
      form.append("file", file);
      return (await client.post<UploadResponse>("/v1/admin/uploads", form)).data.url;
    },
  });
}
