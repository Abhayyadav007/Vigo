import Ionicons from "@expo/vector-icons/Ionicons";
import { resolveMediaUrl, useApiBaseUrl, useCatalogCategories, useCatalogProducts } from "@vigo/api-client";
import type { CatalogCategory } from "@vigo/types";
import { EmptyState } from "@vigo/ui";
import { Stack, router, useLocalSearchParams } from "expo-router";
import { useState } from "react";
import { FlatList, Image, Pressable, Text, View } from "react-native";
import { CartBar } from "../../components/CartControls";
import { DeliveryGate } from "../../components/DeliveryGate";
import { ProductGrid } from "../../components/ProductGrid";
import { useStore } from "../../lib/location";
import { colors } from "../../lib/theme";

export default function CategoryScreen() {
  return (
    <DeliveryGate>
      <CategoryBrowser />
    </DeliveryGate>
  );
}

/** Category list on the left, the selected category's products on the right. */
function CategoryBrowser() {
  const params = useLocalSearchParams<{ id: string; name?: string }>();
  const store = useStore();
  const categories = useCatalogCategories(store.id);
  const stocked = (categories.data ?? []).filter((c) => c.productCount > 0);
  const [selectedId, setSelectedId] = useState(params.id);
  const selected = stocked.find((c) => c.id === selectedId);
  const products = useCatalogProducts(store.id, { categoryId: selectedId });
  const items = products.data?.pages.flatMap((p) => p.items) ?? [];

  return (
    <View className="flex-1 bg-background">
      <Stack.Screen
        options={{
          headerShown: true,
          title: selected?.name ?? params.name ?? "Category",
          headerRight: () => (
            <Pressable accessibilityRole="button" accessibilityLabel="Search" onPress={() => router.push("/search")} hitSlop={8}>
              <Ionicons name="search" size={22} color={colors.ink} />
            </Pressable>
          ),
        }}
      />
      <View className="flex-1 flex-row">
        {stocked.length > 1 ? (
          <FlatList
            className="w-24 flex-grow-0 border-r border-line bg-surface"
            data={stocked}
            keyExtractor={(c) => c.id}
            showsVerticalScrollIndicator={false}
            contentContainerClassName="pb-24"
            renderItem={({ item }) => (
              <SidebarItem category={item} active={item.id === selectedId} onPress={() => setSelectedId(item.id)} />
            )}
          />
        ) : null}
        <View className="flex-1">
          <ProductGrid
            dense
            products={items}
            loadingMore={products.isFetchingNextPage}
            onEndReached={() => {
              if (products.hasNextPage && !products.isFetchingNextPage) void products.fetchNextPage();
            }}
            empty={products.isPending ? undefined : <EmptyState title="No products in this category yet" />}
          />
        </View>
      </View>
      <CartBar />
    </View>
  );
}

function SidebarItem({ category, active, onPress }: { category: CatalogCategory; active: boolean; onPress: () => void }) {
  const base = useApiBaseUrl();
  const uri = resolveMediaUrl(category.imageUrl, base);
  return (
    <Pressable
      accessibilityRole="tab"
      accessibilityState={{ selected: active }}
      onPress={onPress}
      className={`flex-row items-stretch ${active ? "bg-brand-light" : ""}`}
    >
      <View className="flex-1 items-center gap-1 px-1 py-3">
        <View className="h-12 w-12 items-center justify-center overflow-hidden rounded-pill bg-background">
          {uri ? (
            <Image source={{ uri }} className="h-full w-full" resizeMode="cover" />
          ) : (
            <Text className="text-lg font-bold text-brand">{category.name.charAt(0)}</Text>
          )}
        </View>
        <Text
          className={`text-center text-xs ${active ? "font-bold text-ink" : "text-muted"}`}
          numberOfLines={2}
        >
          {category.name}
        </Text>
      </View>
      <View className={`w-1 rounded-l-sm ${active ? "bg-brand" : ""}`} />
    </Pressable>
  );
}
