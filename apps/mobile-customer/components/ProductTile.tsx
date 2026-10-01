import { discountPercent, formatPaise, resolveMediaUrl, useApiBaseUrl } from "@vigo/api-client";
import type { CatalogProduct } from "@vigo/types";
import { ProductCard } from "@vigo/ui";
import { router } from "expo-router";
import { useStore } from "../lib/location";
import { AddToCart } from "./CartControls";

/** A catalog product card with its ADD stepper; used by grids and home rails. */
export function ProductTile({ product: p }: { product: CatalogProduct }) {
  const base = useApiBaseUrl();
  const eta = `${useStore().etaMinutes} MINS`;
  const off = discountPercent(p.mrpPaise, p.pricePaise);
  return (
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
      eta={eta}
      action={<AddToCart product={p} />}
      onPress={() => router.push({ pathname: "/product/[id]", params: { id: p.id } })}
    />
  );
}
