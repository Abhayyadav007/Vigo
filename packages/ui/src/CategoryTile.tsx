import { Image, Pressable, Text, View } from "react-native";

export interface CategoryTileProps {
  name: string;
  imageUri?: string | undefined;
  onPress: () => void;
}

export function CategoryTile({ name, imageUri, onPress }: CategoryTileProps) {
  return (
    <Pressable accessibilityRole="button" onPress={onPress} className="flex-1 items-center gap-2 active:opacity-70">
      <View className="aspect-square w-full items-center justify-center overflow-hidden rounded-lg bg-brand-light">
        {imageUri ? (
          <Image source={{ uri: imageUri }} className="h-full w-full" resizeMode="cover" />
        ) : (
          <Text className="text-2xl font-bold text-brand">{name.charAt(0)}</Text>
        )}
      </View>
      <Text className="text-center text-xs font-medium text-ink" numberOfLines={2}>
        {name}
      </Text>
    </Pressable>
  );
}
