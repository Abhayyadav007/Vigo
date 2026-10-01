import Ionicons from "@expo/vector-icons/Ionicons";
import { useCatalogCategories, useCatalogProducts } from "@vigo/api-client";
import { EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { useDeferredValue, useState } from "react";
import { Pressable, Text, TextInput, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { CartBar } from "../components/CartControls";
import { DeliveryGate } from "../components/DeliveryGate";
import { ProductGrid } from "../components/ProductGrid";
import { useStore } from "../lib/location";
import { colors } from "../lib/theme";

export default function SearchScreen() {
  return (
    <DeliveryGate>
      <Search />
    </DeliveryGate>
  );
}

function Search() {
  const store = useStore();
  const insets = useSafeAreaInsets();
  const [text, setText] = useState("");
  const q = useDeferredValue(text.trim());
  const products = useCatalogProducts(q.length >= 2 ? store.id : undefined, { q });
  const items = products.data?.pages.flatMap((p) => p.items) ?? [];

  return (
    <View className="flex-1 bg-background" style={{ paddingTop: insets.top + 8 }}>
      <View className="flex-row items-center gap-2 px-4 pb-3">
        <Pressable accessibilityRole="button" accessibilityLabel="Back" onPress={() => router.back()} hitSlop={8}>
          <Ionicons name="arrow-back" size={24} color={colors.ink} />
        </Pressable>
        <View className="h-12 flex-1 flex-row items-center gap-2 rounded-lg border border-line bg-surface px-3">
          <Ionicons name="search" size={18} color={colors.muted} />
          <TextInput
            autoFocus
            testID="search-input"
            placeholder="Search for atta, dal, milk…"
            placeholderTextColor={colors.muted}
            value={text}
            onChangeText={setText}
            returnKeyType="search"
            clearButtonMode="while-editing"
            className="h-full flex-1 text-base text-ink"
          />
        </View>
      </View>
      {q.length < 2 ? (
        <Suggestions />
      ) : (
        <ProductGrid
          products={items}
          loadingMore={products.isFetchingNextPage}
          onEndReached={() => {
            if (products.hasNextPage && !products.isFetchingNextPage) void products.fetchNextPage();
          }}
          empty={
            products.isPending ? undefined : (
              <EmptyState title="No results" message={`Nothing matches "${q}" at ${store.name}.`} />
            )
          }
        />
      )}
      <CartBar />
    </View>
  );
}

/** Before typing: the store's categories as one-tap searches. */
function Suggestions() {
  const store = useStore();
  const categories = useCatalogCategories(store.id);
  const stocked = (categories.data ?? []).filter((c) => c.productCount > 0);
  if (stocked.length === 0) {
    return <EmptyState title="What are you looking for?" message="Type at least 2 letters." />;
  }
  return (
    <View className="gap-3 px-4 pt-2">
      <Text className="text-base font-bold text-ink">Popular at {store.name}</Text>
      <View className="flex-row flex-wrap gap-2">
        {stocked.map((c) => (
          <Pressable
            key={c.id}
            onPress={() => router.push({ pathname: "/category/[id]", params: { id: c.id, name: c.name } })}
            className="flex-row items-center gap-1 rounded-pill border border-line bg-surface px-3 py-2 active:opacity-70"
          >
            <Ionicons name="trending-up" size={14} color={colors.brand} />
            <Text className="text-sm text-ink">{c.name}</Text>
          </Pressable>
        ))}
      </View>
      <Text className="text-xs text-muted">Or type at least 2 letters to search.</Text>
    </View>
  );
}
