import { keepPreviousData, useInfiniteQuery, useQuery } from "@tanstack/react-query";
import type { CatalogCategory, CatalogProduct, Page, ServiceabilityResponse } from "@vigo/types";
import { useApiClient } from "../context";

const PAGE_SIZE = 20;

export const catalogKeys = {
  serviceability: (lat: number, lng: number) => ["serviceability", lat, lng] as const,
  categories: (storeId: string) => ["catalog", storeId, "categories"] as const,
  products: (storeId: string, categoryId?: string, q?: string) =>
    ["catalog", storeId, "products", categoryId ?? null, q ?? null] as const,
  product: (storeId: string, id: string) => ["catalog", storeId, "product", id] as const,
};

/** Which dark store (if any) delivers to this point. Rounded so tiny GPS jitter reuses the cache. */
export function useServiceability(coords: { lat: number; lng: number } | null) {
  const client = useApiClient();
  const lat = coords ? Math.round(coords.lat * 10_000) / 10_000 : 0;
  const lng = coords ? Math.round(coords.lng * 10_000) / 10_000 : 0;
  return useQuery({
    queryKey: catalogKeys.serviceability(lat, lng),
    queryFn: async () =>
      (await client.get<ServiceabilityResponse>("/v1/customer/serviceability", { params: { lat, lng } })).data,
    enabled: coords !== null,
    staleTime: 5 * 60_000,
  });
}

export function useCatalogCategories(storeId: string | undefined) {
  const client = useApiClient();
  return useQuery({
    queryKey: catalogKeys.categories(storeId ?? ""),
    queryFn: async () =>
      (await client.get<CatalogCategory[]>("/v1/customer/catalog/categories", { params: { storeId } })).data,
    enabled: !!storeId,
    staleTime: 60_000,
  });
}

/** Infinite product list for a store, optionally by category and/or search text. */
export function useCatalogProducts(storeId: string | undefined, filter: { categoryId?: string; q?: string } = {}) {
  const client = useApiClient();
  const q = filter.q?.trim() || undefined;
  return useInfiniteQuery({
    queryKey: catalogKeys.products(storeId ?? "", filter.categoryId, q),
    queryFn: async ({ pageParam }) =>
      (
        await client.get<Page<CatalogProduct>>("/v1/customer/catalog/products", {
          params: { storeId, categoryId: filter.categoryId, q, limit: PAGE_SIZE, offset: pageParam },
        })
      ).data,
    initialPageParam: 0,
    getNextPageParam: (last) => (last.offset + last.limit < last.total ? last.offset + last.limit : undefined),
    enabled: !!storeId,
    placeholderData: keepPreviousData,
    staleTime: 30_000,
  });
}

export function useCatalogProduct(storeId: string | undefined, productId: string | undefined) {
  const client = useApiClient();
  return useQuery({
    queryKey: catalogKeys.product(storeId ?? "", productId ?? ""),
    queryFn: async () =>
      (await client.get<CatalogProduct>(`/v1/customer/catalog/products/${productId}`, { params: { storeId } })).data,
    enabled: !!storeId && !!productId,
  });
}
