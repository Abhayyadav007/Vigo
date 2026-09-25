import { discountPercent, formatPaise, resolveMediaUrl, useApiBaseUrl } from "@vigo/api-client";
import type { CatalogProduct } from "@vigo/types";
import { colors, ProductCard } from "@vigo/ui";
import { router } from "expo-router";
import type { ReactElement } from "react";
import { ActivityIndicator, FlatList, View } from "react-native";
import { AddToCart } from "./CartControls";

export interface ProductGridProps {
  products: CatalogProduct[];
  onEndReached?: () => void;
  loadingMore?: boolean;
  header?: ReactElement;
  empty?: ReactElement;
}

export function ProductGrid({ products, onEndReached, loadingMore, header, empty }: ProductGridProps) {
  const base = useApiBaseUrl();
  return (
    <FlatList
      data={products}
      keyExtractor={(p) => p.id}
      numColumns={2}
      columnWrapperClassName="gap-3"
      contentContainerClassName="gap-3 px-4 pb-24"
      ListHeaderComponent={header}
      ListEmptyComponent={empty}
      onEndReached={onEndReached}
      onEndReachedThreshold={0.5}
      keyboardShouldPersistTaps="handled"
      ListFooterComponent={loadingMore ? <ActivityIndicator color={colors.brand} /> : null}
      renderItem={({ item: p }) => {
        const off = discountPercent(p.mrpPaise, p.pricePaise);
        return (
          <View className="flex-1">
            <ProductCard
              testID={`product-${p.id}`}
              name={p.name}
              brand={p.brand}
              unitLabel={p.unitLabel}
              imageUri={resolveMediaUrl(p.imageUrls[0], base)}
              price={formatPaise(p.pricePaise)}
              mrp={formatPaise(p.mrpPaise)}
              badge={off > 0 ? `${off}% OFF` : undefined}
              inStock={p.inStock}
              action={<AddToCart product={p} />}
              onPress={() => router.push({ pathname: "/product/[id]", params: { id: p.id } })}
            />
          </View>
        );
      }}
    />
  );
}
