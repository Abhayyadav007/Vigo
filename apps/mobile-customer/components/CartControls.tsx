import { formatPaise, useCart, useSetCartItem } from "@vigo/api-client";
import type { CatalogProduct } from "@vigo/types";
import { QuantityStepper } from "@vigo/ui";
import { router } from "expo-router";
import { Alert, Pressable, Text, View } from "react-native";
import { useStore } from "../lib/location";

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
export function CartBar() {
  const store = useStore();
  const cart = useCart(store.id);
  const count = cart.data?.itemCount ?? 0;
  if (count === 0) return null;
  return (
    <View className="absolute bottom-3 left-4 right-4">
      <Pressable
        testID="cart-bar"
        accessibilityRole="button"
        onPress={() => router.navigate("/cart")}
        className="flex-row items-center justify-between rounded-lg bg-brand px-4 py-3 shadow-lg active:bg-brand-dark"
      >
        <Text className="font-semibold text-white">
          {count} {count === 1 ? "item" : "items"} · {formatPaise(cart.data?.bill.itemTotalPaise ?? 0)}
        </Text>
        <Text className="font-bold text-white">View cart ›</Text>
      </Pressable>
    </View>
  );
}
