import { StyleSheet, Text, View } from "react-native";
import { colors, fontSize, radius, spacing } from "./theme";

export type StatusTone = "ok" | "down" | "pending";

const toneColor: Record<StatusTone, string> = {
  ok: colors.success,
  down: colors.danger,
  pending: colors.textMuted,
};

export interface StatusCardProps {
  title: string;
  rows: ReadonlyArray<{ label: string; tone: StatusTone; value: string }>;
}

export function StatusCard({ title, rows }: StatusCardProps) {
  return (
    <View style={styles.card}>
      <Text style={styles.title}>{title}</Text>
      {rows.map((row) => (
        <View key={row.label} style={styles.row}>
          <View style={[styles.dot, { backgroundColor: toneColor[row.tone] }]} />
          <Text style={styles.label}>{row.label}</Text>
          <Text style={styles.value}>{row.value}</Text>
        </View>
      ))}
    </View>
  );
}

const styles = StyleSheet.create({
  card: {
    backgroundColor: colors.surface,
    borderRadius: radius.lg,
    padding: spacing.lg,
    gap: spacing.sm,
    alignSelf: "stretch",
  },
  title: { fontSize: fontSize.md, fontWeight: "600", color: colors.text, marginBottom: spacing.xs },
  row: { flexDirection: "row", alignItems: "center", gap: spacing.sm },
  dot: { width: 10, height: 10, borderRadius: radius.pill },
  label: { flex: 1, fontSize: fontSize.sm, color: colors.text },
  value: { fontSize: fontSize.sm, color: colors.textMuted },
});
