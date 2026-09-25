import { useCatalogProducts } from "@vigo/api-client";
import { colors, EmptyState } from "@vigo/ui";
import { useDeferredValue, useState } from "react";
import { TextInput, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { ProductGrid } from "../../components/ProductGrid";
import { useStore } from "../../lib/location";

export default function Search() {
  const store = useStore();
  const insets = useSafeAreaInsets();
  const [text, setText] = useState("");
  const q = useDeferredValue(text.trim());
  const products = useCatalogProducts(q.length >= 2 ? store.id : undefined, { q });
  const items = products.data?.pages.flatMap((p) => p.items) ?? [];

  return (
    <View className="flex-1 bg-background" style={{ paddingTop: insets.top + 12 }}>
      <View className="px-4 pb-3">
        <TextInput
          autoFocus
          testID="search-input"
          placeholder="Search for atta, dal, milk…"
          placeholderTextColor={colors.muted}
          value={text}
          onChangeText={setText}
          returnKeyType="search"
          clearButtonMode="while-editing"
          className="h-12 rounded-md border border-line bg-surface px-4 text-base text-ink"
        />
      </View>
      <ProductGrid
        products={items}
        loadingMore={products.isFetchingNextPage}
        onEndReached={() => {
          if (products.hasNextPage && !products.isFetchingNextPage) void products.fetchNextPage();
        }}
        empty={
          q.length < 2 ? (
            <EmptyState title="What are you looking for?" message="Type at least 2 letters." />
          ) : products.isPending ? undefined : (
            <EmptyState title="No results" message={`Nothing matches "${q}" at ${store.name}.`} />
          )
        }
      />
    </View>
  );
}
