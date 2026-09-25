import type { ReactNode } from "react";
import { Text, View } from "react-native";

export function EmptyState({ title, message, children }: { title: string; message?: string; children?: ReactNode }) {
  return (
    <View className="flex-1 items-center justify-center gap-3 px-8">
      <Text className="text-center text-xl font-bold text-ink">{title}</Text>
      {message ? <Text className="text-center text-base text-muted">{message}</Text> : null}
      {children ? <View className="mt-2 w-full gap-3">{children}</View> : null}
    </View>
  );
}
