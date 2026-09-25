import { Pressable, Text, View } from "react-native";

export interface QuantityStepperProps {
  quantity: number;
  max: number;
  onChange: (quantity: number) => void;
  disabled?: boolean;
  size?: "sm" | "md";
  testID?: string;
}

/** "ADD" when 0, otherwise − n +. */
export function QuantityStepper({ quantity, max, onChange, disabled, size = "sm", testID }: QuantityStepperProps) {
  const h = size === "sm" ? "h-9" : "h-11";
  if (quantity === 0) {
    return (
      <Pressable
        testID={testID}
        accessibilityRole="button"
        accessibilityLabel="Add to cart"
        disabled={disabled || max === 0}
        onPress={() => onChange(1)}
        className={`${h} items-center justify-center rounded-md border border-brand bg-brand-light px-4 active:opacity-70 ${disabled || max === 0 ? "opacity-40" : ""}`}
      >
        <Text className="font-bold text-brand">ADD</Text>
      </Pressable>
    );
  }
  return (
    <View testID={testID} className={`${h} flex-row items-center justify-between rounded-md bg-brand`}>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Remove one"
        disabled={disabled}
        onPress={() => onChange(quantity - 1)}
        className="h-full w-9 items-center justify-center active:opacity-70"
      >
        <Text className="text-lg font-bold text-white">−</Text>
      </Pressable>
      <Text className="min-w-6 text-center font-bold text-white" accessibilityLabel={`${quantity} in cart`}>
        {quantity}
      </Text>
      <Pressable
        accessibilityRole="button"
        accessibilityLabel="Add one more"
        disabled={disabled || quantity >= max}
        onPress={() => onChange(quantity + 1)}
        className={`h-full w-9 items-center justify-center active:opacity-70 ${quantity >= max ? "opacity-40" : ""}`}
      >
        <Text className="text-lg font-bold text-white">+</Text>
      </Pressable>
    </View>
  );
}
