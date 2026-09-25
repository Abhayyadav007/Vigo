import Ionicons from "@expo/vector-icons/Ionicons";
import { colors } from "@vigo/ui";
import { Tabs } from "expo-router/js-tabs";
import type { ComponentProps } from "react";
import type { ColorValue } from "react-native";
import { DeliveryGate } from "../../components/DeliveryGate";

type IconName = ComponentProps<typeof Ionicons>["name"];

const tab = (title: string, icon: IconName) => ({
  title,
  tabBarIcon: ({ color, size }: { color: ColorValue; size: number }) => <Ionicons name={icon} color={color} size={size} />,
});

export default function TabsLayout() {
  return (
    <DeliveryGate>
      <Tabs screenOptions={{ headerShown: false, tabBarActiveTintColor: colors.brand }}>
        <Tabs.Screen name="index" options={tab("Home", "home-outline")} />
        <Tabs.Screen name="search" options={tab("Search", "search-outline")} />
        <Tabs.Screen name="cart" options={tab("Cart", "bag-outline")} />
        <Tabs.Screen name="orders" options={tab("Orders", "receipt-outline")} />
        <Tabs.Screen name="account" options={tab("Account", "person-outline")} />
      </Tabs>
    </DeliveryGate>
  );
}
