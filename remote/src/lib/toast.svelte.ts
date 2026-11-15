export type ToastTone = "neutral" | "bad";

export const toast = $state({
  message: "",
  tone: "neutral" as ToastTone,
  visible: false,
});

let timer: ReturnType<typeof setTimeout> | undefined;

export function say(message: string, bad = false): void {
  clearTimeout(timer);
  if (!message) {
    toast.visible = false;
    return;
  }
  toast.message = message;
  toast.tone = bad ? "bad" : "neutral";
  toast.visible = true;
  timer = setTimeout(() => {
    toast.visible = false;
  }, 3400);
}

export function clearToast(): void {
  clearTimeout(timer);
  toast.visible = false;
}
