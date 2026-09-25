import type { ReactNode } from "react";
import { ScrollView, View } from "react-native";
import { useSafeAreaInsets } from "react-native-safe-area-context";

export interface ScreenProps {
  children: ReactNode;
  /** Wrap content in a ScrollView (forms, long pages). */
  scroll?: boolean;
  className?: string;
}

export function Screen({ children, scroll = false, className = "" }: ScreenProps) {
  const insets = useSafeAreaInsets();
  const padding = { paddingTop: insets.top + 16, paddingBottom: insets.bottom + 16 };
  const body = `gap-6 px-6 ${className}`;

  return scroll ? (
    <ScrollView
      className="flex-1 bg-background"
      contentContainerStyle={padding}
      contentContainerClassName={body}
      keyboardShouldPersistTaps="handled"
    >
      {children}
    </ScrollView>
  ) : (
    <View className={`flex-1 bg-background ${body}`} style={padding}>
      {children}
    </View>
  );
}
