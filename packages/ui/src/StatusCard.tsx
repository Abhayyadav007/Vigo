import { Text, View } from "react-native";

export type StatusTone = "ok" | "down" | "pending";

const dot: Record<StatusTone, string> = {
  ok: "bg-success",
  down: "bg-danger",
  pending: "bg-muted",
};

export interface StatusCardProps {
  title: string;
  rows: ReadonlyArray<{ label: string; tone: StatusTone; value: string }>;
}

export function StatusCard({ title, rows }: StatusCardProps) {
  return (
    <View className="gap-2 self-stretch rounded-lg bg-surface p-4">
      <Text className="mb-1 text-base font-semibold text-ink">{title}</Text>
      {rows.map((row) => (
        <View key={row.label} className="flex-row items-center gap-2">
          <View className={`h-2.5 w-2.5 rounded-pill ${dot[row.tone]}`} />
          <Text className="flex-1 text-sm text-ink">{row.label}</Text>
          <Text className="text-sm text-muted">{row.value}</Text>
        </View>
      ))}
    </View>
  );
}
