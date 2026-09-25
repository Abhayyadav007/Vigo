import { ActivityIndicator, Pressable, Text } from "react-native";
import { colors } from "./theme";

export type ButtonVariant = "primary" | "secondary" | "ghost";

export interface ButtonProps {
  title: string;
  onPress: () => void;
  variant?: ButtonVariant;
  disabled?: boolean;
  loading?: boolean;
  /** Tall targets for gloved/rugged-device use (picker, rider). */
  size?: "md" | "lg";
  testID?: string;
}

const container: Record<ButtonVariant, string> = {
  primary: "bg-brand active:bg-brand-dark",
  secondary: "bg-surface border border-line active:bg-line",
  ghost: "bg-transparent active:bg-surface",
};
const label: Record<ButtonVariant, string> = {
  primary: "text-white",
  secondary: "text-ink",
  ghost: "text-brand",
};

export function Button({ title, onPress, variant = "primary", disabled, loading, size = "md", testID }: ButtonProps) {
  const inactive = disabled || loading;
  return (
    <Pressable
      testID={testID}
      accessibilityRole="button"
      accessibilityState={{ disabled: !!inactive, busy: !!loading }}
      disabled={inactive}
      onPress={onPress}
      className={`flex-row items-center justify-center rounded-md px-5 ${size === "lg" ? "h-16" : "h-12"} ${container[variant]} ${inactive ? "opacity-50" : ""}`}
    >
      {loading ? (
        <ActivityIndicator color={variant === "primary" ? "#fff" : colors.brand} />
      ) : (
        <Text className={`font-semibold ${size === "lg" ? "text-lg" : "text-base"} ${label[variant]}`}>{title}</Text>
      )}
    </Pressable>
  );
}
