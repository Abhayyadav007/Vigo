import { resolveMediaUrl, useApiBaseUrl, useCatalogCategories } from "@vigo/api-client";
import { CategoryTile, EmptyState } from "@vigo/ui";
import { router } from "expo-router";
import { ActivityIndicator, FlatList, Pressable, Text, View } from "react-native";
import { CartBar } from "../../components/CartControls";
import { CategoryRail } from "../../components/CategoryRail";
import { HomeHeader } from "../../components/HomeHeader";
import { PromoBanners } from "../../components/PromoBanners";
import { useStore } from "../../lib/location";
import { colors } from "../../lib/theme";

const GRID_SIZE = 8;

export default function Home() {
  const store = useStore();
  const base = useApiBaseUrl();
  const categories = useCatalogCategories(store.id);
  const stocked = (categories.data ?? []).filter((c) => c.productCount > 0);

  const feedHeader = (
    <View className="gap-4 pt-4">
      <PromoBanners categories={stocked} />
      {stocked.length > 0 ? (
        <View className="gap-3 px-4">
          <View className="flex-row items-center justify-between">
            <Text className="text-lg font-bold text-ink">Shop by category</Text>
            {stocked.length > GRID_SIZE ? (
              <Pressable accessibilityRole="link" onPress={() => router.navigate("/categories")} hitSlop={8}>
                <Text className="font-semibold text-brand">See all</Text>
              </Pressable>
            ) : null}
          </View>
          <View className="flex-row flex-wrap gap-y-4">
            {stocked.slice(0, GRID_SIZE).map((c) => (
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
    </View>
  );

  return (
    <View className="flex-1 bg-background">
      <HomeHeader />
      <FlatList
        data={stocked}
        keyExtractor={(c) => c.id}
        ListHeaderComponent={feedHeader}
        contentContainerClassName="pb-24"
        renderItem={({ item }) => <CategoryRail category={item} />}
        onRefresh={() => void categories.refetch()}
        refreshing={categories.isRefetching}
        ListEmptyComponent={
          categories.isPending ? (
            <ActivityIndicator className="mt-10" color={colors.brand} />
          ) : (
            <EmptyState title="Nothing here yet" message="This store hasn't stocked any products yet." />
          )
        }
      />
      <CartBar overTabs />
    </View>
  );
}
