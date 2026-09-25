import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { PackRequest, PickList, PickerQueueItem, ScanResult } from "@vigo/types";
import { useCallback } from "react";
import { useApiClient } from "../context";
import { useLiveEvents } from "./useLiveEvents";

export const pickerKeys = {
  queue: ["picker", "queue"] as const,
  order: (id: string) => ["picker", "order", id] as const,
};

/** The store queue, kept fresh by the picker WebSocket (with slow polling as a fallback). */
export function usePickerQueue() {
  const client = useApiClient();
  const qc = useQueryClient();
  const query = useQuery({
    queryKey: pickerKeys.queue,
    queryFn: async () => (await client.get<PickerQueueItem[]>("/v1/picker/orders")).data,
    refetchInterval: 60_000,
  });
  const onMessage = useCallback(() => {
    // Any order event (or a resync after reconnecting) means the queue changed.
    void qc.invalidateQueries({ queryKey: ["picker"] });
  }, [qc]);
  const live = useLiveEvents("/v1/ws/picker", onMessage);
  return { ...query, live };
}

export function usePickList(id: string | undefined) {
  const client = useApiClient();
  return useQuery({
    queryKey: pickerKeys.order(id ?? ""),
    queryFn: async () => (await client.get<PickList>(`/v1/picker/orders/${id}`)).data,
    enabled: !!id,
  });
}

/** Shared plumbing: every picker mutation returns the fresh pick list. */
function usePickMutation<V>(id: string, request: (vars: V) => Promise<PickList>) {
  const qc = useQueryClient();
  return useMutation({
    scope: { id: `pick:${id}` },
    mutationFn: request,
    onSuccess: (list) => {
      qc.setQueryData(pickerKeys.order(id), list);
      return qc.invalidateQueries({ queryKey: pickerKeys.queue });
    },
  });
}

export function useStartPicking(id: string) {
  const client = useApiClient();
  return usePickMutation(id, async () => (await client.post<PickList>(`/v1/picker/orders/${id}/start`)).data);
}

export function useScan(id: string) {
  const client = useApiClient();
  const qc = useQueryClient();
  return useMutation({
    scope: { id: `pick:${id}` },
    mutationFn: async (barcode: string) =>
      (await client.post<ScanResult>(`/v1/picker/orders/${id}/scan`, { barcode })).data,
    onSuccess: (res) => qc.setQueryData(pickerKeys.order(id), res.pickList),
  });
}

export function useSetPicked(id: string) {
  const client = useApiClient();
  return usePickMutation(
    id,
    async ({ productId, pickedQuantity }: { productId: string; pickedQuantity: number }) =>
      (await client.put<PickList>(`/v1/picker/orders/${id}/items/${productId}`, { pickedQuantity })).data,
  );
}

export function usePack(id: string) {
  const client = useApiClient();
  return usePickMutation(id, async (body: PackRequest) => (await client.post<PickList>(`/v1/picker/orders/${id}/pack`, body)).data);
}

export function useReleaseOrder(id: string) {
  const client = useApiClient();
  return usePickMutation(id, async () => (await client.post<PickList>(`/v1/picker/orders/${id}/release`)).data);
}

export function usePickerCancel(id: string) {
  const client = useApiClient();
  return usePickMutation(
    id,
    async (reason: string) => (await client.post<PickList>(`/v1/picker/orders/${id}/cancel`, { reason })).data,
  );
}
