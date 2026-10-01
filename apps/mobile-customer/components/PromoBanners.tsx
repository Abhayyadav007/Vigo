import type { CatalogCategory } from "@vigo/types";
import { router } from "expo-router";
import { Pressable, ScrollView, Text, View } from "react-native";

type Banner = { title: string; subtitle: string; tone: string };

// Copy only: no artwork, so nothing to license. Each card opens the matching
// category from the store's list (by position), or the Categories tab.
const BANNERS: Banner[] = [
  { title: "Fresh picks, every morning", subtitle: "Fruits, vegetables and dairy from your nearest store", tone: "bg-offer" },
  { title: "Stock up on essentials", subtitle: "Atta, rice, dal and oil at everyday prices", tone: "bg-brand" },
  { title: "Snack o'clock", subtitle: "Chips, biscuits and cold drinks in minutes", tone: "bg-ink" },
];

export function PromoBanners({ categories }: { categories: CatalogCategory[] }) {
  return (
    <ScrollView horizontal showsHorizontalScrollIndicator={false} contentContainerClassName="gap-3 px-4">
      {BANNERS.map((b, i) => {
        const category = categories[i];
        return (
          <Pressable
            key={b.title}
            accessibilityRole="button"
            onPress={() =>
              category
                ? router.push({ pathname: "/category/[id]", params: { id: category.id, name: category.name } })
                : router.navigate("/categories")
            }
            className={`w-72 justify-between gap-3 rounded-lg p-4 active:opacity-90 ${b.tone}`}
          >
            <View className="gap-1">
              <Text className="text-lg font-extrabold text-background">{b.title}</Text>
              <Text className="text-sm text-background">{b.subtitle}</Text>
            </View>
            <View className="self-start rounded-pill bg-surface px-3 py-1">
              <Text className="text-xs font-bold text-ink">Shop now ›</Text>
            </View>
          </Pressable>
        );
      })}
    </ScrollView>
  );
}
