import { resolveMediaUrl, useApiBaseUrl, useCatalogCategories } from "@vigo/api-client";
import { CategoryTile, EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { ActivityIndicator, FlatList, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { CartBar } from "../../components/CartControls";
import { useStore } from "../../lib/location";
import { colors } from "../../lib/theme";

export default function Categories() {
  const store = useStore();
  const base = useApiBaseUrl();
  const insets = useSafeAreaInsets();
  // Three columns with 16px side padding and 12px gaps; fixed width keeps a short last row aligned.
  const tileWidth = (useWindowDimensions().width - 32 - 24) / 3;
  const categories = useCatalogCategories(store.id);
  const stocked = (categories.data ?? []).filter((c) => c.productCount > 0);

  return (
    <View className="flex-1 bg-background">
      <FlatList
        data={stocked}
        keyExtractor={(c) => c.id}
        numColumns={3}
        columnWrapperClassName="gap-3"
        contentContainerClassName="gap-4 px-4 pb-24"
        contentContainerStyle={{ paddingTop: insets.top + 12 }}
        ListHeaderComponent={<Text className="text-2xl font-bold text-ink">All categories</Text>}
        ListEmptyComponent={
          categories.isPending ? (
            <ActivityIndicator className="mt-10" color={colors.brand} />
          ) : (
            <EmptyState title="No categories yet" />
          )
        }
        renderItem={({ item: c }) => (
          <View style={{ width: tileWidth }}>
            <CategoryTile
              name={c.name}
              imageUri={resolveMediaUrl(c.imageUrl, base)}
              onPress={() => router.push({ pathname: "/category/[id]", params: { id: c.id, name: c.name } })}
            />
          </View>
        )}
      />
      <CartBar overTabs />
    </View>
  );
}
