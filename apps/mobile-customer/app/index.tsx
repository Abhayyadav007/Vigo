import { useHealth } from "@vigo/api-client";
import { StatusCard, colors, fontSize, spacing, type StatusTone } from "@vigo/ui";
import { StyleSheet, Text } from "react-native";
import { SafeAreaView } from "react-native-safe-area-context";

// TODO(phase-2): replace with the phone-login flow for the CUSTOMER role.
export default function Home() {
  const health = useHealth();
  const tone = (s: "ok" | "down" | undefined): StatusTone => s ?? (health.isError ? "down" : "pending");
  const label = (s: "ok" | "down" | undefined) => s ?? (health.isError ? "unreachable" : "…");

  return (
    <SafeAreaView style={styles.screen}>
      <Text style={styles.title}>Vigo</Text>
      <StatusCard
        title="Backend"
        rows={[
          { label: "API", tone: tone(health.data?.status), value: label(health.data?.status) },
          { label: "Postgres", tone: tone(health.data?.database), value: label(health.data?.database) },
          { label: "Redis", tone: tone(health.data?.redis), value: label(health.data?.redis) },
        ]}
      />
    </SafeAreaView>
  );
}

const styles = StyleSheet.create({
  screen: { flex: 1, padding: spacing.xl, gap: spacing.xl, backgroundColor: colors.background },
  title: { fontSize: fontSize.xxl, fontWeight: "700", color: colors.brand },
});
