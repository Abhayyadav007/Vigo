import { useAuth, usePickerQueue, useStartPicking, type LiveStatus } from "@vigo/api-client";
import type { PickerQueueItem } from "@vigo/types";
import { Button, colors, EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { useMemo } from "react";
import { ActivityIndicator, Alert, SectionList, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

const minutesAgo = (iso: string) => Math.max(0, Math.round((Date.now() - new Date(iso).getTime()) / 60_000));

export default function Queue() {
  const insets = useSafeAreaInsets();
  const { state, signOut } = useAuth();
  const queue = usePickerQueue();
  const { width } = useWindowDimensions();
  const columns = width >= 768 ? 2 : 1;

  const sections = useMemo(() => {
    const all = queue.data ?? [];
    const pick = (f: (o: PickerQueueItem) => boolean) => all.filter(f);
    return [
      { title: "My orders", data: pick((o) => o.status === "PICKING" && o.isMine) },
      { title: "Waiting to be picked", data: pick((o) => o.status === "CONFIRMED") },
      { title: "Packed — waiting for rider", data: pick((o) => o.status === "PACKED") },
      { title: "Being picked by others", data: pick((o) => o.status === "PICKING" && !o.isMine) },
    ].filter((s) => s.data.length > 0);
  }, [queue.data]);

  if (state.status === "signedIn" && !state.user.storeId) {
    return (
      <EmptyState title="No store assigned" message="Ask an admin to assign you to a dark store.">
        <Button title="Sign out" variant="secondary" size="lg" onPress={() => void signOut()} />
      </EmptyState>
    );
  }

  return (
    <View className="flex-1 bg-background" style={{ paddingTop: insets.top }}>
      <View className="flex-row items-center justify-between px-5 py-3">
        <Text className="text-3xl font-bold text-brand">Orders</Text>
        <View className="flex-row items-center gap-4">
          <LiveBadge status={queue.live} />
          <Button title="Sign out" variant="ghost" onPress={() => void signOut()} />
        </View>
      </View>
      {queue.isPending ? (
        <ActivityIndicator size="large" color={colors.brand} />
      ) : (
        <SectionList
          sections={sections.map((s) => ({ ...s, data: chunk(s.data, columns) }))}
          keyExtractor={(row) => row.map((o) => o.id).join()}
          contentContainerClassName="gap-3 px-5 pb-10"
          stickySectionHeadersEnabled={false}
          onRefresh={() => void queue.refetch()}
          refreshing={queue.isRefetching}
          ListEmptyComponent={<EmptyState title="All caught up" message="New orders appear here instantly." />}
          renderSectionHeader={({ section }) => (
            <Text className="mt-4 text-lg font-bold text-muted">
              {section.title} ({section.data.flat().length})
            </Text>
          )}
          renderItem={({ item: row }) => (
            <View className="flex-row gap-3">
              {row.map((o) => (
                <OrderCard key={o.id} order={o} />
              ))}
              {row.length < columns ? <View className="flex-1" /> : null}
            </View>
          )}
        />
      )}
    </View>
  );
}

function chunk<T>(items: T[], size: number): T[][] {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) out.push(items.slice(i, i + size));
  return out;
}

function OrderCard({ order: o }: { order: PickerQueueItem }) {
  const start = useStartPicking(o.id);
  const open = () => router.push({ pathname: "/order/[id]", params: { id: o.id } });
  const age = minutesAgo(o.createdAt);
  const urgent = o.status === "CONFIRMED" && age >= 3;

  return (
    <View className={`flex-1 gap-3 rounded-lg border-2 p-4 ${urgent ? "border-warning" : "border-line"}`}>
      <View className="flex-row items-center justify-between">
        <Text className="text-2xl font-bold text-ink">{o.number}</Text>
        <Text className={`text-base font-semibold ${urgent ? "text-warning" : "text-muted"}`}>{age} min</Text>
      </View>
      <Text className="text-base text-ink">
        {o.itemCount} items · {o.lineCount} lines
        {o.status === "PICKING" ? ` · ${o.linesDone}/${o.lineCount} done` : ""}
        {o.stagingSlot ? ` · slot ${o.stagingSlot}` : ""}
      </Text>
      {o.status === "CONFIRMED" ? (
        <Button
          title="Start picking"
          size="lg"
          loading={start.isPending}
          onPress={() =>
            start.mutate(undefined, {
              onSuccess: open,
              onError: (e) => Alert.alert("Couldn't start", e.message),
            })
          }
        />
      ) : o.status === "PICKING" && o.isMine ? (
        <Button title="Continue" size="lg" onPress={open} />
      ) : (
        <Button title="View" size="lg" variant="secondary" onPress={open} />
      )}
    </View>
  );
}

function LiveBadge({ status }: { status: LiveStatus }) {
  const live = status === "open";
  return (
    <View className="flex-row items-center gap-2">
      <View className={`h-3 w-3 rounded-pill ${live ? "bg-success" : "bg-warning"}`} />
      <Text className="text-sm text-muted">{live ? "Live" : status === "stopped" ? "Offline" : "Reconnecting…"}</Text>
    </View>
  );
}
