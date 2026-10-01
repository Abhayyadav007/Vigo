import Ionicons from "@expo/vector-icons/Ionicons";
import { Tabs } from "expo-router/js-tabs";
import type { ComponentProps } from "react";
import type { ColorValue } from "react-native";
import { DeliveryGate } from "../../components/DeliveryGate";
import { colors } from "../../lib/theme";

type IconName = ComponentProps<typeof Ionicons>["name"];

const tab = (title: string, icon: IconName, activeIcon: IconName) => ({
  title,
  tabBarIcon: ({ color, size, focused }: { color: ColorValue; size: number; focused: boolean }) => (
    <Ionicons name={focused ? activeIcon : icon} color={color} size={size} />
  ),
});

// Search and the cart are stack screens (no tab bar), reached from the search
// pill and the floating cart bar.
export default function TabsLayout() {
  return (
    <DeliveryGate>
      <Tabs
        screenOptions={{
          headerShown: false,
          tabBarActiveTintColor: colors.brand,
          tabBarInactiveTintColor: colors.muted,
          tabBarLabelStyle: { fontWeight: "600" },
        }}
      >
        <Tabs.Screen name="index" options={tab("Home", "home-outline", "home")} />
        <Tabs.Screen name="categories" options={tab("Categories", "grid-outline", "grid")} />
        <Tabs.Screen name="orders" options={tab("Orders", "receipt-outline", "receipt")} />
        <Tabs.Screen name="account" options={tab("Account", "person-outline", "person")} />
      </Tabs>
    </DeliveryGate>
  );
}
