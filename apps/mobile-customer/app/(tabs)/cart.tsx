import { EmptyState, Screen } from "@vigo/ui";

// TODO(phase-4): cart with optimistic updates and checkout.
export default function Cart() {
  return (
    <Screen>
      <EmptyState title="Your cart" message="Adding to cart and checkout arrive in the next update." />
    </Screen>
  );
}
