import { Button, colors, EmptyState } from "@vigo/ui";
import type { ReactNode } from "react";
import { ActivityIndicator, Linking, View } from "react-native";
import { useDelivery } from "../lib/location";

/** Renders children only once we know which store delivers to the customer. */
export function DeliveryGate({ children }: { children: ReactNode }) {
  const { state, retry } = useDelivery();
  switch (state.status) {
    case "ready":
      return <>{children}</>;
    case "locating":
      return (
        <View className="flex-1 items-center justify-center bg-background">
          <ActivityIndicator size="large" color={colors.brand} />
        </View>
      );
    case "denied":
      return (
        <EmptyState
          title="Location needed"
          message="Vigo uses your location to find the store that can deliver to you in minutes."
        >
          <Button title="Open settings" onPress={() => void Linking.openSettings()} />
          <Button title="Try again" variant="secondary" onPress={retry} />
        </EmptyState>
      );
    case "unserviceable":
      return (
        <EmptyState
          title="We're not here yet"
          message="Vigo doesn't deliver to your location yet. We're opening new stores across India soon."
        >
          <Button title="Check again" variant="secondary" onPress={retry} />
        </EmptyState>
      );
    case "error":
      return (
        <EmptyState title="Something went wrong" message={state.message}>
          <Button title="Try again" onPress={retry} />
        </EmptyState>
      );
  }
}
