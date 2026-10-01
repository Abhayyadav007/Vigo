import { useCatalogProducts } from "@vigo/api-client";
import type { CatalogCategory } from "@vigo/types";
import { router } from "expo-router";
import { FlatList, Pressable, Text, View } from "react-native";
import { useStore } from "../lib/location";
import { ProductTile } from "./ProductTile";

const RAIL_SIZE = 10;

/** One horizontal "shelf" of a category's first products, with a link to the full list. */
export function CategoryRail({ category }: { category: CatalogCategory }) {
  const store = useStore();
  const products = useCatalogProducts(store.id, { categoryId: category.id });
  const items = (products.data?.pages[0]?.items ?? []).slice(0, RAIL_SIZE);
  if (!products.isPending && items.length === 0) return null;

  const open = () => router.push({ pathname: "/category/[id]", params: { id: category.id, name: category.name } });
  return (
    <View className="gap-3 py-3">
      <View className="flex-row items-center justify-between px-4">
        <Text className="text-lg font-bold text-ink">{category.name}</Text>
        <Pressable accessibilityRole="link" onPress={open} hitSlop={8}>
          <Text className="font-semibold text-brand">See all</Text>
        </Pressable>
      </View>
      <FlatList
        horizontal
        data={items}
        keyExtractor={(p) => p.id}
        showsHorizontalScrollIndicator={false}
        contentContainerClassName="gap-3 px-4"
        renderItem={({ item }) => (
          <View className="w-40">
            <ProductTile product={item} />
          </View>
        )}
      />
    </View>
  );
}
