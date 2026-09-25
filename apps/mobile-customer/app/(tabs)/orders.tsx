import { EmptyState, Screen } from "@vigo/ui";

// TODO(phase-4): order history; TODO(phase-7): live tracking.
export default function Orders() {
  return (
    <Screen>
      <EmptyState title="Your orders" message="Your past and current orders will show up here." />
    </Screen>
  );
}
