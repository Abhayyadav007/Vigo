import { useCatalogProducts } from "@vigo/api-client";
import { EmptyState } from "@vigo/ui";
import { Stack, useLocalSearchParams } from "expo-router";
import { View } from "react-native";
import { DeliveryGate } from "../../components/DeliveryGate";
import { ProductGrid } from "../../components/ProductGrid";
import { useStore } from "../../lib/location";

export default function CategoryScreen() {
  return (
    <DeliveryGate>
      <CategoryProducts />
    </DeliveryGate>
  );
}

function CategoryProducts() {
  const { id, name } = useLocalSearchParams<{ id: string; name?: string }>();
  const store = useStore();
  const products = useCatalogProducts(store.id, { categoryId: id });
  const items = products.data?.pages.flatMap((p) => p.items) ?? [];

  return (
    <View className="flex-1 bg-background pt-3">
      <Stack.Screen options={{ headerShown: true, title: name ?? "Category" }} />
      <ProductGrid
        products={items}
        loadingMore={products.isFetchingNextPage}
        onEndReached={() => {
          if (products.hasNextPage && !products.isFetchingNextPage) void products.fetchNextPage();
        }}
        empty={products.isPending ? undefined : <EmptyState title="No products in this category yet" />}
      />
    </View>
  );
}
