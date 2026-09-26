import {
  ApiError,
  resolveMediaUrl,
  useApiBaseUrl,
  usePack,
  usePickerCancel,
  usePickList,
  useReleaseOrder,
  useScan,
  useSetPicked,
} from "@vigo/api-client";
import type { PickLine } from "@vigo/types";
import { Button, colors, EmptyState } from "@vigo/ui";
import { CameraView, useCameraPermissions } from "expo-camera";
import * as Haptics from "expo-haptics";
import { useKeepAwake } from "expo-keep-awake";
import { Stack, router, useLocalSearchParams } from "expo-router";
import { useRef, useState } from "react";
import {
  ActivityIndicator,
  Alert,
  FlatList,
  Image,
  Modal,
  Pressable,
  Text,
  TextInput,
  View,
} from "react-native";

type Feedback = { ok: boolean; text: string } | null;

/** Ignore the same code seen again within this window (cameras fire repeatedly). */
const DUPLICATE_WINDOW_MS = 1500;

export default function PickScreen() {
  useKeepAwake();
  const { id } = useLocalSearchParams<{ id: string }>();
  const list = usePickList(id);
  const scan = useScan(id);
  const setPicked = useSetPicked(id);
  const release = useReleaseOrder(id);
  const cancel = usePickerCancel(id);
  const base = useApiBaseUrl();
  const [feedback, setFeedback] = useState<Feedback>(null);
  const [cameraOn, setCameraOn] = useState(false);
  const [packing, setPacking] = useState(false);
  const [permission, requestPermission] = useCameraPermissions();
  const wedge = useRef<TextInput>(null);
  const [wedgeText, setWedgeText] = useState("");
  const lastScan = useRef({ code: "", at: 0 });

  if (list.isPending) {
    return (
      <View className="flex-1 items-center justify-center">
        <ActivityIndicator size="large" color={colors.brand} />
      </View>
    );
  }
  if (!list.data) return <EmptyState title="Order not found" />;
  const o = list.data;
  const editable = o.status === "PICKING" && o.isMine;
  const done = o.lines.filter((l) => l.pickedQuantity !== null).length;

  const onScanned = (raw: string) => {
    const code = raw.trim();
    const now = Date.now();
    if (!code || (code === lastScan.current.code && now - lastScan.current.at < DUPLICATE_WINDOW_MS)) return;
    lastScan.current = { code, at: now };
    scan.mutate(code, {
      onSuccess: (res) => {
        void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Success);
        setFeedback({
          ok: true,
          text: `✓ ${res.line.name}  ${res.line.pickedQuantity ?? 0}/${res.line.quantity}${res.lineComplete ? " — done" : ""}`,
        });
      },
      onError: (e) => {
        void Haptics.notificationAsync(Haptics.NotificationFeedbackType.Error);
        setFeedback({ ok: false, text: `✗ ${e instanceof ApiError ? e.message : "Scan failed"}` });
      },
    });
  };

  const markMissing = (line: PickLine) => {
    const found = line.pickedQuantity ?? 0;
    Alert.alert(
      `Mark ${line.quantity - found} × ${line.name} missing?`,
      "The customer won't be charged for them, and the shelf will be marked empty.",
      [
        { text: "Keep looking", style: "cancel" },
        {
          text: "Mark missing",
          style: "destructive",
          onPress: () => setPicked.mutate({ productId: line.productId, pickedQuantity: found }),
        },
      ],
    );
  };

  return (
    <View className="flex-1 bg-background">
      <Stack.Screen
        options={{
          headerShown: true,
          title: `${o.number} · ${done}/${o.lines.length}`,
          headerRight: editable
            ? () => (
                <Pressable
                  accessibilityRole="button"
                  onPress={() =>
                    Alert.alert("Order options", undefined, [
                      { text: "Return to queue", onPress: () => release.mutate(undefined, { onSuccess: () => router.back() }) },
                      {
                        text: "Cancel order (can't fulfil)",
                        style: "destructive",
                        onPress: () =>
                          cancel.mutate("Store couldn't fulfil the order", { onSuccess: () => router.back() }),
                      },
                      { text: "Close", style: "cancel" },
                    ])
                  }
                >
                  <Text className="px-2 text-lg font-semibold text-brand">Options</Text>
                </Pressable>
              )
            : undefined,
        }}
      />

      {editable ? (
        <View className="gap-2 border-b border-line p-4">
          {cameraOn && permission?.granted ? (
            <CameraView
              style={{ height: 180, borderRadius: 12 }}
              facing="back"
              barcodeScannerSettings={{ barcodeTypes: ["ean13", "ean8", "upc_a", "upc_e", "code128"] }}
              onBarcodeScanned={({ data }) => onScanned(data)}
            />
          ) : null}
          <View className="flex-row gap-2">
            {/* Hardware scanners type the code + Enter into this field. */}
            <TextInput
              ref={wedge}
              testID="scan-input"
              autoFocus
              value={wedgeText}
              onChangeText={setWedgeText}
              onSubmitEditing={() => {
                onScanned(wedgeText);
                setWedgeText("");
                wedge.current?.focus();
              }}
              submitBehavior="submit"
              placeholder="Scan or type barcode"
              placeholderTextColor={colors.muted}
              keyboardType="number-pad"
              returnKeyType="done"
              className="h-14 flex-1 rounded-md border-2 border-line px-4 text-xl text-ink"
            />
            <Button
              title={cameraOn ? "Hide camera" : "Camera"}
              variant="secondary"
              size="lg"
              onPress={() => {
                if (!permission?.granted) void requestPermission();
                setCameraOn(!cameraOn);
              }}
            />
          </View>
          {feedback ? (
            <View className={`rounded-md p-3 ${feedback.ok ? "bg-brand-light" : "bg-danger"}`}>
              <Text className={`text-lg font-bold ${feedback.ok ? "text-brand-dark" : "text-white"}`}>{feedback.text}</Text>
            </View>
          ) : null}
        </View>
      ) : (
        <Text className="bg-surface p-4 text-base text-muted">
          {o.status === "PACKED"
            ? `Packed in ${o.bagCount ?? "?"} bag(s)${o.stagingSlot ? ` at ${o.stagingSlot}` : ""}. Waiting for a rider.`
            : "Read-only: this order isn't assigned to you."}
        </Text>
      )}

      <FlatList
        data={o.lines}
        keyExtractor={(l) => l.productId}
        contentContainerClassName="gap-3 p-4 pb-32"
        renderItem={({ item: line }) => (
          <LineCard
            line={line}
            imageUri={resolveMediaUrl(line.imageUrl, base)}
            editable={editable}
            onSet={(n) => setPicked.mutate({ productId: line.productId, pickedQuantity: n })}
            onMissing={() => markMissing(line)}
          />
        )}
      />

      {editable ? (
        <View className="absolute bottom-0 left-0 right-0 border-t border-line bg-background p-4">
          <Button
            testID="pack"
            title={o.canPack ? "Pack order" : `Count every item (${done}/${o.lines.length})`}
            size="lg"
            disabled={!o.canPack}
            onPress={() => setPacking(true)}
          />
        </View>
      ) : null}

      <PackModal id={o.id} visible={packing} onClose={() => setPacking(false)} />
    </View>
  );
}

function LineCard({
  line,
  imageUri,
  editable,
  onSet,
  onMissing,
}: {
  line: PickLine;
  imageUri: string | undefined;
  editable: boolean;
  onSet: (n: number) => void;
  onMissing: () => void;
}) {
  const picked = line.pickedQuantity ?? 0;
  const complete = line.pickedQuantity === line.quantity;
  const short = line.pickedQuantity !== null && !complete;
  const border = complete ? "border-success bg-brand-light" : short ? "border-warning" : "border-line";

  return (
    <View className={`flex-row gap-4 rounded-lg border-2 p-4 ${border}`}>
      <View className="w-24 items-center justify-center rounded-md bg-ink py-3">
        <Text className="text-xs text-white">BIN</Text>
        <Text className="text-xl font-bold text-white">{line.binLocation ?? "—"}</Text>
      </View>
      {imageUri ? <Image source={{ uri: imageUri }} className="h-20 w-20 rounded-md" resizeMode="contain" /> : null}
      <View className="flex-1 gap-1">
        <Text className="text-lg font-semibold text-ink">{line.name}</Text>
        <Text className="text-base text-muted">
          {[line.brand, line.unitLabel, line.barcode].filter(Boolean).join(" · ")}
        </Text>
        <Text className={`text-2xl font-bold ${complete ? "text-success" : short ? "text-warning" : "text-ink"}`}>
          {picked} / {line.quantity}
          {short ? "  (short)" : ""}
        </Text>
      </View>
      {editable ? (
        <View className="justify-center gap-2">
          <View className="flex-row gap-2">
            <Pressable
              accessibilityLabel={`One less ${line.name}`}
              disabled={picked === 0}
              onPress={() => onSet(picked - 1)}
              className="h-14 w-14 items-center justify-center rounded-md bg-surface active:bg-line"
            >
              <Text className="text-2xl font-bold text-ink">−</Text>
            </Pressable>
            <Pressable
              accessibilityLabel={`One more ${line.name}`}
              disabled={picked >= line.quantity}
              onPress={() => onSet(picked + 1)}
              className="h-14 w-14 items-center justify-center rounded-md bg-surface active:bg-line"
            >
              <Text className="text-2xl font-bold text-ink">+</Text>
            </Pressable>
          </View>
          {!complete ? (
            <Pressable onPress={onMissing} className="h-10 items-center justify-center rounded-md border border-warning">
              <Text className="font-semibold text-warning">Missing</Text>
            </Pressable>
          ) : null}
        </View>
      ) : null}
    </View>
  );
}

function PackModal({ id, visible, onClose }: { id: string; visible: boolean; onClose: () => void }) {
  const pack = usePack(id);
  const [bags, setBags] = useState(1);
  const [slot, setSlot] = useState("");
  return (
    <Modal visible={visible} animationType="slide" transparent onRequestClose={onClose}>
      <View className="flex-1 justify-end bg-black/40">
        <View className="gap-5 rounded-t-lg bg-background p-6 pb-10">
          <Text className="text-2xl font-bold text-ink">Pack order</Text>
          <View className="flex-row items-center justify-between">
            <Text className="text-lg text-ink">Bags</Text>
            <View className="flex-row items-center gap-4">
              <Button title="−" variant="secondary" size="lg" disabled={bags <= 1} onPress={() => setBags(bags - 1)} />
              <Text className="w-10 text-center text-2xl font-bold text-ink">{bags}</Text>
              <Button title="+" variant="secondary" size="lg" disabled={bags >= 20} onPress={() => setBags(bags + 1)} />
            </View>
          </View>
          <TextInput
            value={slot}
            onChangeText={(t) => setSlot(t.toUpperCase())}
            placeholder="Staging slot (e.g. S-03)"
            placeholderTextColor={colors.muted}
            autoCapitalize="characters"
            maxLength={16}
            className="h-14 rounded-md border-2 border-line px-4 text-xl text-ink"
          />
          {pack.error ? <Text className="text-danger">{pack.error.message}</Text> : null}
          <Button
            title="Mark packed"
            size="lg"
            loading={pack.isPending}
            onPress={() =>
              pack.mutate(
                { bagCount: bags, ...(slot.trim() ? { stagingSlot: slot.trim() } : {}) },
                {
                  onSuccess: () => {
                    onClose();
                    router.back();
                  },
                },
              )
            }
          />
          <Button title="Back" variant="ghost" onPress={onClose} />
        </View>
      </View>
    </Modal>
  );
}
