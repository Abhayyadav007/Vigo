import { discountPercent, formatPaise, resolveMediaUrl, useApiBaseUrl, useCatalogProduct } from "@vigo/api-client";
import { EmptyState } from "@vigo/ui";
import { Stack, useLocalSearchParams } from "expo-router";
import { ActivityIndicator, Image, ScrollView, Text, useWindowDimensions, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { AddToCart } from "../../components/CartControls";
import { DeliveryGate } from "../../components/DeliveryGate";
import { useStore } from "../../lib/location";
import { colors } from "../../lib/theme";

export default function ProductScreen() {
  return (
    <DeliveryGate>
      <ProductDetail />
    </DeliveryGate>
  );
}

function ProductDetail() {
  const { id } = useLocalSearchParams<{ id: string }>();
  const store = useStore();
  const base = useApiBaseUrl();
  const { width } = useWindowDimensions();
  const insets = useSafeAreaInsets();
  const product = useCatalogProduct(store.id, id);

  if (product.isPending) {
    return (
      <View className="flex-1 items-center justify-center bg-background">
        <ActivityIndicator color={colors.brand} />
      </View>
    );
  }
  if (!product.data) return <EmptyState title="Product unavailable" message="It isn't sold at your store right now." />;

  const p = product.data;
  const off = discountPercent(p.mrpPaise, p.pricePaise);
  const images = p.imageUrls.map((u) => resolveMediaUrl(u, base)).filter((u): u is string => !!u);

  return (
    <View className="flex-1 bg-background">
      <Stack.Screen options={{ headerShown: true, title: "" }} />
      <ScrollView contentContainerClassName="gap-3 pb-6">
        {images.length > 0 ? (
          <ScrollView horizontal pagingEnabled showsHorizontalScrollIndicator={false} className="bg-photo">
            {images.map((uri) => (
              <Image key={uri} source={{ uri }} style={{ width, height: width }} resizeMode="contain" />
            ))}
          </ScrollView>
        ) : (
          <View className="items-center justify-center bg-surface" style={{ width, height: width * 0.7 }}>
            <Text className="text-6xl">🛒</Text>
          </View>
        )}
        <View className="mx-4 gap-2 rounded-lg bg-surface p-4">
          <View className="self-start rounded-sm border border-line bg-background px-2 py-0.5">
            <Text className="text-xs font-bold text-ink">⏱ {store.etaMinutes} MINS</Text>
          </View>
          <Text className="text-2xl font-bold text-ink">{p.name}</Text>
          <Text className="text-base text-muted">{p.unitLabel}</Text>
          {p.brand ? (
            <Text className="text-sm text-muted">
              Brand: <Text className="font-semibold text-ink">{p.brand}</Text>
            </Text>
          ) : null}
        </View>
        {p.description ? (
          <View className="mx-4 gap-2 rounded-lg bg-surface p-4">
            <Text className="text-base font-bold text-ink">Product details</Text>
            <Text className="text-sm leading-6 text-ink">{p.description}</Text>
          </View>
        ) : null}
      </ScrollView>
      <View
        className="flex-row items-center gap-3 border-t border-line bg-surface px-4 pt-3"
        style={{ paddingBottom: insets.bottom + 12 }}
      >
        <View className="flex-1">
          <Text className="text-xs text-muted">{p.unitLabel}</Text>
          <View className="flex-row flex-wrap items-baseline gap-x-2">
            <Text className="text-xl font-bold text-ink">{formatPaise(p.pricePaise)}</Text>
            {off > 0 ? (
              <>
                <Text className="text-sm text-muted line-through">MRP {formatPaise(p.mrpPaise)}</Text>
                <Text className="text-sm font-bold text-offer">{off}% OFF</Text>
              </>
            ) : null}
          </View>
          <Text className="text-xs text-muted">Inclusive of all taxes</Text>
        </View>
        <View className="w-32">
          {p.inStock ? (
            <AddToCart product={p} size="md" />
          ) : (
            <Text className="text-center text-base font-semibold text-muted">Out of stock</Text>
          )}
        </View>
      </View>
    </View>
  );
}
