import type { ReactNode } from "react";
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
  /** Rendered next to the price, e.g. an add-to-cart stepper. */
  action?: ReactNode;
  /** Delivery-time chip, e.g. "10 MINS". */
  eta?: string | undefined;
  testID?: string;
}

export function ProductCard(p: ProductCardProps) {
  return (
    <Pressable
      testID={p.testID}
      accessibilityRole="button"
      accessibilityLabel={`${p.name}, ${p.unitLabel}, ${p.price}${p.inStock ? "" : ", out of stock"}`}
      onPress={p.onPress}
      className="flex-1 gap-1 rounded-lg border border-line bg-surface p-2 active:opacity-80"
    >
      {/* Product photos are shot on white, so they sit on a white tile. */}
      <View className="aspect-square items-center justify-center overflow-hidden rounded-md bg-photo">
        {p.imageUri ? (
          <Image source={{ uri: p.imageUri }} className="h-full w-full" resizeMode="contain" />
        ) : (
          <Text className="text-3xl">🛒</Text>
        )}
        {p.badge ? (
          <View className="absolute left-1.5 top-0 rounded-b-md bg-offer px-1.5 py-1">
            <Text className="text-xs font-extrabold text-white">{p.badge}</Text>
          </View>
        ) : null}
        {!p.inStock ? (
          <View className="absolute inset-0 items-center justify-center bg-background/70">
            <Text className="text-sm font-semibold text-ink">Out of stock</Text>
          </View>
        ) : null}
      </View>
      {p.eta ? (
        <View className="mt-1 self-start rounded-sm border border-line bg-background px-1.5 py-0.5">
          <Text className="text-[10px] font-bold text-ink">⏱ {p.eta}</Text>
        </View>
      ) : null}
      <Text className="min-h-10 text-sm font-semibold leading-5 text-ink" numberOfLines={2}>
        {p.name}
      </Text>
      <Text className="text-xs text-muted" numberOfLines={1}>
        {p.brand ? `${p.brand} · ` : ""}
        {p.unitLabel}
      </Text>
      <View className="mt-1 flex-row items-center justify-between gap-2">
        <View>
          <Text className="text-sm font-bold text-ink">{p.price}</Text>
          {p.mrp && p.mrp !== p.price ? <Text className="text-xs text-muted line-through">{p.mrp}</Text> : null}
        </View>
        {p.action}
      </View>
    </Pressable>
  );
}
