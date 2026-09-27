import { useInfiniteQuery, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type {
  AddressDto,
  AddressRequest,
  CartLine,
  CartResponse,
  CatalogProduct,
  CheckoutRequest,
  CheckoutResponse,
  OrderDetail,
  OrderStatus,
  OrderSummary,
  Page,
  WsServerMessage,
} from "@vigo/types";
import { useCallback } from "react";
import { useApiClient } from "../context";
import { useLiveEvents } from "./useLiveEvents";

export const orderKeys = {
  addresses: ["addresses"] as const,
  cart: (storeId: string) => ["cart", storeId] as const,
  orders: ["orders"] as const,
  order: (id: string) => ["orders", id] as const,
};

const TERMINAL: OrderStatus[] = ["DELIVERED", "CANCELLED", "PARTIALLY_FULFILLED"];
export const isOrderActive = (status: OrderStatus) => !TERMINAL.includes(status);

// ---------- addresses ----------

export function useAddresses() {
  const client = useApiClient();
  return useQuery({
    queryKey: orderKeys.addresses,
    queryFn: async () => (await client.get<AddressDto[]>("/v1/customer/addresses")).data,
  });
}

export function useSaveAddress() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, body }: { id?: string | undefined; body: AddressRequest }) =>
      id
        ? (await client.put<AddressDto>(`/v1/customer/addresses/${id}`, body)).data
        : (await client.post<AddressDto>("/v1/customer/addresses", body)).data,
    onSuccess: () => qc.invalidateQueries({ queryKey: orderKeys.addresses }),
  });
}

export function useDeleteAddress() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      await client.delete(`/v1/customer/addresses/${id}`);
    },
    onSuccess: () => qc.invalidateQueries({ queryKey: orderKeys.addresses }),
  });
}

// ---------- cart ----------

export function useCart(storeId: string | undefined) {
  const client = useApiClient();
  return useQuery({
    queryKey: orderKeys.cart(storeId ?? ""),
    queryFn: async () => (await client.get<CartResponse>("/v1/customer/cart", { params: { storeId } })).data,
    enabled: !!storeId,
  });
}

/** Client-side guess of the cart after a quantity change; the server response replaces it. */
function applyQuantity(cart: CartResponse, productId: string, quantity: number, product?: CatalogProduct): CartResponse {
  const existing = cart.items.find((l) => l.productId === productId);
  let items: CartLine[];
  if (quantity === 0) {
    items = cart.items.filter((l) => l.productId !== productId);
  } else if (existing) {
    items = cart.items.map((l) =>
      l.productId === productId ? { ...l, quantity, lineTotalPaise: l.unitPricePaise * quantity } : l,
    );
  } else if (product) {
    items = [
      ...cart.items,
      {
        productId,
        name: product.name,
        brand: product.brand,
        unitLabel: product.unitLabel,
        imageUrl: product.imageUrls[0] ?? null,
        quantity,
        unitPricePaise: product.pricePaise,
        unitMrpPaise: product.mrpPaise,
        lineTotalPaise: product.pricePaise * quantity,
        available: true,
        maxQuantity: product.maxQuantity,
      },
    ];
  } else {
    return cart;
  }
  const available = items.filter((l) => l.available);
  const itemTotal = available.reduce((sum, l) => sum + l.lineTotalPaise, 0);
  return {
    ...cart,
    items,
    itemCount: available.reduce((sum, l) => sum + l.quantity, 0),
    bill: { ...cart.bill, itemTotalPaise: itemTotal, totalPaise: itemTotal + cart.bill.deliveryFeePaise },
  };
}

/**
 * Sets a cart line (0 removes). Optimistic; mutations run one at a time
 * (shared scope) so fast taps can't land out of order.
 */
export function useSetCartItem(storeId: string) {
  const client = useApiClient();
  const qc = useQueryClient();
  const key = orderKeys.cart(storeId);
  return useMutation({
    scope: { id: `cart:${storeId}` },
    mutationFn: async ({ productId, quantity }: { productId: string; quantity: number; product?: CatalogProduct }) =>
      (await client.put<CartResponse>(`/v1/customer/cart/items/${productId}`, { storeId, quantity })).data,
    onMutate: async ({ productId, quantity, product }) => {
      await qc.cancelQueries({ queryKey: key });
      const previous = qc.getQueryData<CartResponse>(key);
      if (previous) qc.setQueryData(key, applyQuantity(previous, productId, quantity, product));
      return { previous };
    },
    onError: (_err, _vars, ctx) => {
      if (ctx?.previous) qc.setQueryData(key, ctx.previous);
    },
    onSuccess: (cart) => qc.setQueryData(key, cart),
  });
}

// ---------- checkout & orders ----------

export function useCheckout() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ body, idempotencyKey }: { body: CheckoutRequest; idempotencyKey: string }) =>
      (
        await client.post<CheckoutResponse>("/v1/customer/checkout", body, {
          headers: { "Idempotency-Key": idempotencyKey },
        })
      ).data,
    onSuccess: async (res, { body }) => {
      qc.setQueryData(orderKeys.order(res.order.id), res.order);
      await qc.invalidateQueries({ queryKey: orderKeys.cart(body.storeId) });
      await qc.invalidateQueries({ queryKey: orderKeys.orders, exact: true });
    },
    onError: (_err, { body }) => qc.invalidateQueries({ queryKey: orderKeys.cart(body.storeId) }),
  });
}

/** A fresh key per checkout attempt; reuse it when retrying the same attempt. */
export function newIdempotencyKey(): string {
  const rand = () => Math.random().toString(36).slice(2, 10);
  return `ck-${Date.now().toString(36)}-${rand()}${rand()}`;
}

export function useOrders() {
  const client = useApiClient();
  return useInfiniteQuery({
    queryKey: orderKeys.orders,
    queryFn: async ({ pageParam }) =>
      (await client.get<Page<OrderSummary>>("/v1/customer/orders", { params: { limit: 20, offset: pageParam } })).data,
    initialPageParam: 0,
    getNextPageParam: (last) => (last.offset + last.limit < last.total ? last.offset + last.limit : undefined),
  });
}

/** One order, live over `/v1/ws/orders/{id}` while it's in progress (status + rider position). */
export function useOrder(id: string | undefined) {
  const client = useApiClient();
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: orderKeys.order(id ?? ""),
    queryFn: async () => (await client.get<OrderDetail>(`/v1/customer/orders/${id}`)).data,
    enabled: !!id,
    // Slow safety net in case the socket is down.
    refetchInterval: (q) => (q.state.data && isOrderActive(q.state.data.status) ? 60_000 : false),
  });
  const active = !!id && !!query.data && isOrderActive(query.data.status);
  const onMessage = useCallback(
    (msg: WsServerMessage) => {
      const k = orderKeys.order(id ?? "");
      if (msg.type === "riderLocation") {
        const { lat, lng } = msg.location;
        qc.setQueryData<OrderDetail>(k, (o) => (o?.rider ? { ...o, rider: { ...o.rider, location: { lat, lng } } } : o));
      } else if (msg.type === "order" || msg.type === "resync") {
        void qc.invalidateQueries({ queryKey: k });
      }
    },
    [qc, id],
  );
  useLiveEvents(active ? `/v1/ws/orders/${id}` : null, onMessage);
  return query;
}

export function useCancelOrder() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, reason }: { id: string; reason?: string }) =>
      (await client.post<OrderDetail>(`/v1/customer/orders/${id}/cancel`, reason ? { reason } : {})).data,
    onSuccess: (order) => {
      qc.setQueryData(orderKeys.order(order.id), order);
      return qc.invalidateQueries({ queryKey: orderKeys.orders, exact: true });
    },
  });
}
