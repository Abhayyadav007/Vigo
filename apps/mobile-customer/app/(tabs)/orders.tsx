import { formatPaise, useOrders } from "@vigo/api-client";
import { colors, EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { ActivityIndicator, FlatList, Pressable, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { StatusChip } from "../../components/OrderStatus";

export default function Orders() {
  const insets = useSafeAreaInsets();
  const orders = useOrders();
  const items = orders.data?.pages.flatMap((p) => p.items) ?? [];

  if (orders.isPending) {
    return (
      <View className="flex-1 items-center justify-center bg-background">
        <ActivityIndicator color={colors.brand} />
      </View>
    );
  }
  return (
    <FlatList
      className="flex-1 bg-background"
      contentContainerClassName="gap-3 px-4 pb-8"
      contentContainerStyle={{ paddingTop: insets.top + 12 }}
      data={items}
      keyExtractor={(o) => o.id}
      onRefresh={() => void orders.refetch()}
      refreshing={orders.isRefetching}
      onEndReached={() => {
        if (orders.hasNextPage && !orders.isFetchingNextPage) void orders.fetchNextPage();
      }}
      ListHeaderComponent={<Text className="text-2xl font-bold text-ink">Your orders</Text>}
      ListEmptyComponent={<EmptyState title="No orders yet" message="Orders you place will show up here." />}
      renderItem={({ item: o }) => (
        <Pressable
          onPress={() => router.push({ pathname: "/order/[id]", params: { id: o.id } })}
          className="gap-2 rounded-lg border border-line p-4 active:bg-surface"
        >
          <View className="flex-row items-center justify-between">
            <Text className="font-semibold text-ink">{o.number}</Text>
            <StatusChip status={o.status} />
          </View>
          <Text className="text-sm text-muted">
            {o.itemCount} {o.itemCount === 1 ? "item" : "items"} · {formatPaise(o.totalPaise)} ·{" "}
            {new Date(o.createdAt).toLocaleString("en-IN", { dateStyle: "medium", timeStyle: "short" })}
          </Text>
        </Pressable>
      )}
    />
  );
}
