import { discountPercent, formatPaise, resolveMediaUrl, useApiBaseUrl, useCatalogProduct } from "@vigo/api-client";
import { colors, EmptyState } from "@vigo/ui";
import { Stack, useLocalSearchParams } from "expo-router";
import { ActivityIndicator, Image, ScrollView, Text, useWindowDimensions, View } from "react-native";
import { AddToCart } from "../../components/CartControls";
import { DeliveryGate } from "../../components/DeliveryGate";
import { useStore } from "../../lib/location";

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
      <ScrollView contentContainerClassName="gap-4 pb-8">
        {images.length > 0 ? (
          <ScrollView horizontal pagingEnabled showsHorizontalScrollIndicator={false}>
            {images.map((uri) => (
              <Image key={uri} source={{ uri }} style={{ width, height: width }} resizeMode="contain" />
            ))}
          </ScrollView>
        ) : (
          <View className="items-center justify-center bg-surface" style={{ width, height: width * 0.7 }}>
            <Text className="text-6xl">🛒</Text>
          </View>
        )}
        <View className="gap-2 px-5">
          {p.brand ? <Text className="text-sm text-muted">{p.brand}</Text> : null}
          <Text className="text-2xl font-bold text-ink">{p.name}</Text>
          <Text className="text-base text-muted">{p.unitLabel}</Text>
          <View className="flex-row items-baseline gap-3">
            <Text className="text-2xl font-bold text-ink">{formatPaise(p.pricePaise)}</Text>
            {off > 0 ? (
              <>
                <Text className="text-base text-muted line-through">MRP {formatPaise(p.mrpPaise)}</Text>
                <Text className="text-base font-bold text-brand">{off}% OFF</Text>
              </>
            ) : (
              <Text className="text-sm text-muted">MRP (incl. of all taxes)</Text>
            )}
          </View>
          {p.description ? <Text className="mt-2 text-base leading-6 text-ink">{p.description}</Text> : null}
        </View>
      </ScrollView>
      <View className="gap-3 border-t border-line bg-background p-4 pb-8">
        {p.inStock ? (
          <AddToCart product={p} size="md" />
        ) : (
          <Text className="text-center text-base font-semibold text-muted">Out of stock</Text>
        )}
      </View>
    </View>
  );
}
