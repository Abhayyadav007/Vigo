import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type {
  ActiveDelivery,
  DeliverRequest,
  DeliveryHistoryItem,
  DeliveryOffer,
  LocationPoint,
  Page,
  RiderMe,
  RiderProfileRequest,
  WsServerMessage,
} from "@vigo/types";
import type { AxiosInstance } from "axios";
import { useCallback } from "react";
import { useApiClient } from "../context";
import { useLiveEvents } from "./useLiveEvents";

export const riderKeys = {
  me: ["rider", "me"] as const,
  offers: ["rider", "offers"] as const,
  active: ["rider", "active"] as const,
  history: ["rider", "history"] as const,
};

/** Upload GPS fixes; usable outside React (background location task). */
export async function postRiderLocation(client: AxiosInstance, points: LocationPoint[]): Promise<void> {
  if (points.length === 0) return;
  await client.post("/v1/rider/location", { points: points.slice(-100) });
}

export function useRiderMe() {
  const client = useApiClient();
  return useQuery({
    queryKey: riderKeys.me,
    queryFn: async () => (await client.get<RiderMe>("/v1/rider/me")).data,
  });
}

export function useSetOnline() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (online: boolean) => (await client.put<RiderMe>("/v1/rider/status", { online })).data,
    onSuccess: (me) => {
      qc.setQueryData(riderKeys.me, me);
      return qc.invalidateQueries({ queryKey: riderKeys.offers });
    },
  });
}

export function useUpdateRiderProfile() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (body: RiderProfileRequest) => (await client.put<RiderMe>("/v1/rider/profile", body)).data,
    onSuccess: (me) => qc.setQueryData(riderKeys.me, me),
  });
}

/**
 * Pending offers, pushed live over `/v1/ws/rider` (new offers appear
 * instantly; taken/expired ones vanish). `onOffer` fires for each new offer
 * (e.g. to buzz the phone).
 */
export function useRiderOffers(onOffer?: (offer: DeliveryOffer) => void) {
  const client = useApiClient();
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: riderKeys.offers,
    queryFn: async () => (await client.get<DeliveryOffer[]>("/v1/rider/offers")).data,
    refetchInterval: 30_000,
    // Drop offers whose countdown ran out.
    select: (offers) => offers.filter((o) => new Date(o.expiresAt).getTime() > Date.now()),
  });
  const onMessage = useCallback(
    (msg: WsServerMessage) => {
      switch (msg.type) {
        case "offer":
          qc.setQueryData<DeliveryOffer[]>(riderKeys.offers, (old = []) => [
            ...old.filter((o) => o.orderId !== msg.offer.orderId),
            msg.offer,
          ]);
          onOffer?.(msg.offer);
          break;
        case "offerRevoked":
          qc.setQueryData<DeliveryOffer[]>(riderKeys.offers, (old = []) =>
            old.filter((o) => o.orderId !== msg.orderId),
          );
          break;
        case "order":
        case "resync":
          void qc.invalidateQueries({ queryKey: ["rider"] });
          break;
        default:
          break;
      }
    },
    [qc, onOffer],
  );
  const live = useLiveEvents("/v1/ws/rider", onMessage);
  return { ...query, live };
}

export function useAcceptOffer() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (orderId: string) =>
      (await client.post<ActiveDelivery>(`/v1/rider/offers/${orderId}/accept`)).data,
    onSuccess: (active) => {
      qc.setQueryData(riderKeys.active, active);
      return qc.invalidateQueries({ queryKey: ["rider"] });
    },
    onError: () => qc.invalidateQueries({ queryKey: riderKeys.offers }),
  });
}

export function useDeclineOffer() {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (orderId: string) => {
      await client.post(`/v1/rider/offers/${orderId}/decline`);
      return orderId;
    },
    onSuccess: (orderId) =>
      qc.setQueryData<DeliveryOffer[]>(riderKeys.offers, (old = []) => old.filter((o) => o.orderId !== orderId)),
  });
}

/** The delivery in progress (null when none). */
export function useActiveDelivery() {
  const client = useApiClient();
  return useQuery({
    queryKey: riderKeys.active,
    queryFn: async () => {
      const res = await client.get<ActiveDelivery | "">("/v1/rider/delivery");
      return res.status === 204 || !res.data ? null : res.data;
    },
  });
}

function useDeliveryStep<V>(request: (client: AxiosInstance, vars: V) => Promise<unknown>) {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (vars: V) => request(client, vars),
    onSettled: () => qc.invalidateQueries({ queryKey: ["rider"] }),
  });
}

export const usePickup = () =>
  useDeliveryStep((c, v: { orderId: string; bagCount: number }) =>
    c.post(`/v1/rider/deliveries/${v.orderId}/pickup`, { bagCount: v.bagCount }),
  );

export const useDepart = () =>
  useDeliveryStep((c, orderId: string) => c.post(`/v1/rider/deliveries/${orderId}/depart`));

export const useDeliver = () =>
  useDeliveryStep((c, v: { orderId: string } & DeliverRequest) =>
    c.post(`/v1/rider/deliveries/${v.orderId}/deliver`, {
      otp: v.otp,
      ...(v.codCollectedPaise !== undefined ? { codCollectedPaise: v.codCollectedPaise } : {}),
    }),
  );

export const useUnassign = () =>
  useDeliveryStep((c, orderId: string) => c.post(`/v1/rider/deliveries/${orderId}/unassign`));

export function useDeliveryHistory() {
  const client = useApiClient();
  return useQuery({
    queryKey: riderKeys.history,
    queryFn: async () =>
      (await client.get<Page<DeliveryHistoryItem>>("/v1/rider/deliveries", { params: { limit: 30 } })).data,
  });
}
