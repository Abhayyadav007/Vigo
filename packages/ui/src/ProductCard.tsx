import { Image, Pressable, Text, View } from "react-native";

export interface ProductCardProps {
  name: string;
  unitLabel: string;
  brand?: string | null;
  imageUri?: string | undefined;
  /** Pre-formatted, e.g. "₹26". */
  price: string;
  /** Pre-formatted MRP; shown struck through when different from price. */
  mrp?: string | undefined;
  /** e.g. "7% OFF" */
  badge?: string | undefined;
  inStock: boolean;
  onPress: () => void;
  testID?: string;
}

export function ProductCard(p: ProductCardProps) {
  return (
    <Pressable
      testID={p.testID}
      accessibilityRole="button"
      accessibilityLabel={`${p.name}, ${p.unitLabel}, ${p.price}${p.inStock ? "" : ", out of stock"}`}
      onPress={p.onPress}
      className="flex-1 gap-1 rounded-lg border border-line bg-background p-3 active:bg-surface"
    >
      <View className="aspect-square items-center justify-center overflow-hidden rounded-md bg-surface">
        {p.imageUri ? (
          <Image source={{ uri: p.imageUri }} className="h-full w-full" resizeMode="contain" />
        ) : (
          <Text className="text-3xl text-muted">🛒</Text>
        )}
        {p.badge ? (
          <View className="absolute left-0 top-0 rounded-br-md bg-brand px-2 py-0.5">
            <Text className="text-xs font-bold text-white">{p.badge}</Text>
          </View>
        ) : null}
        {!p.inStock ? (
          <View className="absolute inset-0 items-center justify-center bg-white/70">
            <Text className="text-sm font-semibold text-muted">Out of stock</Text>
          </View>
        ) : null}
      </View>
      {p.brand ? <Text className="text-xs text-muted" numberOfLines={1}>{p.brand}</Text> : null}
      <Text className="text-sm font-medium text-ink" numberOfLines={2}>
        {p.name}
      </Text>
      <Text className="text-xs text-muted">{p.unitLabel}</Text>
      <View className="flex-row items-baseline gap-2">
        <Text className="text-base font-bold text-ink">{p.price}</Text>
        {p.mrp && p.mrp !== p.price ? <Text className="text-xs text-muted line-through">{p.mrp}</Text> : null}
      </View>
    </Pressable>
  );
}
