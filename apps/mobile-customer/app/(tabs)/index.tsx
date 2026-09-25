import { resolveMediaUrl, useApiBaseUrl, useCatalogCategories, useCatalogProducts } from "@vigo/api-client";
import { CategoryTile, EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { Pressable, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { CartBar } from "../../components/CartControls";
import { ProductGrid } from "../../components/ProductGrid";
import { useStore } from "../../lib/location";

export default function Home() {
  const store = useStore();
  const insets = useSafeAreaInsets();
  const base = useApiBaseUrl();
  const categories = useCatalogCategories(store.id);
  const products = useCatalogProducts(store.id);
  const items = products.data?.pages.flatMap((p) => p.items) ?? [];

  const header = (
    <View className="gap-5 pb-2" style={{ paddingTop: insets.top + 12 }}>
      <View className="gap-1">
        <Text className="text-2xl font-bold text-ink" testID="eta">
          Delivery in {store.etaMinutes} minutes
        </Text>
        <Text className="text-sm text-muted">from {store.name}</Text>
      </View>
      <Pressable
        accessibilityRole="search"
        onPress={() => router.push("/search")}
        className="h-12 justify-center rounded-md border border-line bg-surface px-4"
      >
        <Text className="text-base text-muted">Search for atta, dal, milk…</Text>
      </Pressable>
      {categories.data && categories.data.length > 0 ? (
        <View className="gap-3">
          <Text className="text-lg font-bold text-ink">Shop by category</Text>
          <View className="flex-row flex-wrap gap-y-4">
            {categories.data.map((c) => (
              <View key={c.id} className="w-1/4 px-1">
                <CategoryTile
                  name={c.name}
                  imageUri={resolveMediaUrl(c.imageUrl, base)}
                  onPress={() => router.push({ pathname: "/category/[id]", params: { id: c.id, name: c.name } })}
                />
              </View>
            ))}
          </View>
        </View>
      ) : null}
      <Text className="text-lg font-bold text-ink">All products</Text>
    </View>
  );

  return (
    <View className="flex-1 bg-background">
      <ProductGrid
        products={items}
        header={header}
        loadingMore={products.isFetchingNextPage}
        onEndReached={() => {
          if (products.hasNextPage && !products.isFetchingNextPage) void products.fetchNextPage();
        }}
        empty={
          products.isPending ? undefined : (
            <EmptyState title="Nothing here yet" message="This store hasn't stocked any products yet." />
          )
        }
      />
      <CartBar />
    </View>
  );
}
