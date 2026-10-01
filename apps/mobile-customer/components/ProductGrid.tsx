import type { CatalogProduct } from "@vigo/types";
import type { ReactElement } from "react";
import { ActivityIndicator, FlatList, View } from "react-native";
import { colors } from "../lib/theme";
import { ProductTile } from "./ProductTile";

export interface ProductGridProps {
  products: CatalogProduct[];
  onEndReached?: () => void;
  loadingMore?: boolean;
  header?: ReactElement;
  empty?: ReactElement;
  /** Tighter gutters for the category screen's right-hand pane. */
  dense?: boolean;
}

export function ProductGrid({ products, onEndReached, loadingMore, header, empty, dense }: ProductGridProps) {
  return (
    <FlatList
      data={products}
      keyExtractor={(p) => p.id}
      numColumns={2}
      columnWrapperClassName={dense ? "gap-2" : "gap-3"}
      contentContainerClassName={dense ? "gap-2 px-2 pt-2 pb-24" : "gap-3 px-4 pb-24"}
      ListHeaderComponent={header}
      ListEmptyComponent={empty}
      onEndReached={onEndReached}
      onEndReachedThreshold={0.5}
      keyboardShouldPersistTaps="handled"
      ListFooterComponent={loadingMore ? <ActivityIndicator color={colors.brand} /> : null}
      renderItem={({ item }) => (
        <View className="flex-1">
          <ProductTile product={item} />
        </View>
      )}
    />
  );
}
