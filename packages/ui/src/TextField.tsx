import { forwardRef, type ReactNode } from "react";
import { Text, TextInput, View, type TextInputProps } from "react-native";
import { colors } from "./theme";

export interface TextFieldProps extends Omit<TextInputProps, "className"> {
  label: string;
  error?: string | undefined;
  /** Fixed text before the input, e.g. "+91". */
  prefix?: ReactNode;
}

export const TextField = forwardRef<TextInput, TextFieldProps>(function TextField(
  { label, error, prefix, ...input },
  ref,
) {
  return (
    <View className="gap-2">
      <Text className="text-sm font-medium text-ink">{label}</Text>
      <View
        className={`h-14 flex-row items-center rounded-md border bg-background px-4 ${error ? "border-danger" : "border-line"}`}
      >
        {prefix ? <Text className="mr-2 text-lg text-muted">{prefix}</Text> : null}
        <TextInput
          ref={ref}
          placeholderTextColor={colors.muted}
          className="flex-1 text-lg text-ink"
          accessibilityLabel={label}
          {...input}
        />
      </View>
      {error ? <Text className="text-sm text-danger">{error}</Text> : null}
    </View>
  );
});
