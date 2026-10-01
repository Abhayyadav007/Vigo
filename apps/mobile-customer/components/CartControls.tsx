import Ionicons from "@expo/vector-icons/Ionicons";
import { formatPaise, useCart, useSetCartItem } from "@vigo/api-client";
import type { CatalogProduct } from "@vigo/types";
import { QuantityStepper } from "@vigo/ui";
import { router } from "expo-router";
import { Alert, Pressable, Text, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";
import { useStore } from "../lib/location";
import { colors } from "../lib/theme";

function useCartQuantity() {
  const store = useStore();
  const cart = useCart(store.id);
  const set = useSetCartItem(store.id);
  const quantityOf = (productId: string) => cart.data?.items.find((l) => l.productId === productId)?.quantity ?? 0;
  const setQuantity = (productId: string, quantity: number, product?: CatalogProduct) =>
    set.mutate(
      { productId, quantity, ...(product ? { product } : {}) },
      { onError: (e) => Alert.alert("Couldn't update cart", e.message) },
    );
  return { quantityOf, setQuantity };
}

/** Add-to-cart stepper for a catalog product. */
export function AddToCart({ product, size }: { product: CatalogProduct; size?: "sm" | "md" }) {
  const { quantityOf, setQuantity } = useCartQuantity();
  return (
    <QuantityStepper
      testID={`add-${product.id}`}
      size={size}
      quantity={quantityOf(product.id)}
      max={product.maxQuantity}
      disabled={!product.inStock}
      onChange={(q) => setQuantity(product.id, q, product)}
    />
  );
}

/** Stepper for an existing cart line. */
export function CartLineStepper({ productId, max }: { productId: string; max: number }) {
  const { quantityOf, setQuantity } = useCartQuantity();
  return (
    <QuantityStepper
      quantity={quantityOf(productId)}
      max={Math.max(max, 0)}
      onChange={(q) => setQuantity(productId, Math.min(q, Math.max(max, 0)))}
    />
  );
}

/** Floating "View cart" bar shown while the cart has items. */
export function CartBar({ overTabs = false }: { overTabs?: boolean }) {
  const store = useStore();
  const cart = useCart(store.id);
  const insets = useSafeAreaInsets();
  const count = cart.data?.itemCount ?? 0;
  if (count === 0) return null;
  // Above a tab bar the safe area is already taken care of.
  const bottom = 12 + (overTabs ? 0 : insets.bottom);
  return (
    <View className="absolute left-4 right-4" style={{ bottom }}>
      <Pressable
        testID="cart-bar"
        accessibilityRole="button"
        onPress={() => router.navigate("/cart")}
        className="flex-row items-center gap-3 rounded-lg bg-brand px-4 py-3 shadow-lg active:bg-brand-dark"
      >
        <View className="h-9 w-9 items-center justify-center rounded-md bg-brand-dark">
          <Ionicons name="bag-handle" size={20} color={colors.surface} />
        </View>
        <View className="flex-1">
          <Text className="text-sm font-bold text-background">
            {count} {count === 1 ? "item" : "items"}
          </Text>
          <Text className="text-sm font-semibold text-background">
            {formatPaise(cart.data?.bill.itemTotalPaise ?? 0)}
          </Text>
        </View>
        <Text className="text-base font-bold text-background">View cart ›</Text>
      </Pressable>
    </View>
  );
}
