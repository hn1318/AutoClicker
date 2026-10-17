import { ref } from "vue";

export type ToastKind = "info" | "success" | "error";

export interface ToastItem {
  id: number;
  kind: ToastKind;
  message: string;
}

const items = ref<ToastItem[]>([]);
const timers = new Map<number, number>();
let sequence = 0;

function dismiss(id: number): void {
  const timer = timers.get(id);
  if (timer !== undefined) {
    window.clearTimeout(timer);
    timers.delete(id);
  }
  items.value = items.value.filter((item) => item.id !== id);
}

function push(kind: ToastKind, message: string, ttl = 3200): number {
  sequence += 1;
  const id = sequence;
  items.value = [...items.value, { id, kind, message }];
  // Keep the stack shallow; old toasts are the least useful.
  if (items.value.length > 3) {
    const oldest = items.value[0];
    if (oldest) dismiss(oldest.id);
  }
  timers.set(
    id,
    window.setTimeout(() => dismiss(id), ttl),
  );
  return id;
}

export function useToast() {
  return {
    items,
    dismiss,
    info: (message: string) => push("info", message),
    success: (message: string) => push("success", message),
    error: (message: string) => push("error", message, 4800),
  };
}
