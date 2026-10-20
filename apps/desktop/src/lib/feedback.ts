import { writable } from "svelte/store";

export type Notice = {
  id: number;
  title: string;
  detail?: string;
  kind: "success" | "info" | "warning" | "error";
  popup: boolean;
  unread: boolean;
  createdAt: number;
  copyText?: string;
  action?: { label: string; run: () => void | Promise<void> };
};
export type Activity = { id: number; title: string; detail?: string; startedAt: number };
type State = { notices: Notice[]; activities: Activity[]; status: Notice | null; panelOpen: boolean };

// Session-only history: never carries paths or results into a different project.
export function createFeedback() {
  let sequence = 0;
  const store = writable<State>({ notices: [], activities: [], status: null, panelOpen: false });
  return {
    subscribe: store.subscribe,
    publish(input: Pick<Notice, "title"> & Partial<Pick<Notice, "detail" | "kind" | "popup" | "copyText" | "action">>) {
      const notice: Notice = { kind: "info", popup: false, ...input, id: ++sequence, unread: true, createdAt: Date.now() };
      store.update(s => ({ ...s, status: notice, notices: [{ ...notice, unread: !s.panelOpen, popup: notice.popup && !s.panelOpen }, ...s.notices].slice(0, 50) }));
      return notice.id;
    },
    start(title: string, detail?: string) {
      const id = ++sequence;
      store.update(s => ({ ...s, status: null, activities: [...s.activities, { id, title, detail, startedAt: Date.now() }] }));
      return id;
    },
    updateActivity(id: number, detail: string) {
      store.update(s => ({ ...s, activities: s.activities.map(a => a.id === id ? { ...a, detail } : a) }));
    },
    finish(id: number) { store.update(s => ({ ...s, activities: s.activities.filter(a => a.id !== id) })); },
    dismiss(id: number) { store.update(s => ({ ...s, notices: s.notices.map(n => n.id === id ? { ...n, popup: false } : n) })); },
    clearStatus(id: number) { store.update(s => s.status?.id === id ? { ...s, status: null } : s); },
    openPanel() { store.update(s => ({ ...s, panelOpen: true, notices: s.notices.map(n => ({ ...n, unread: false, popup: false })) })); },
    closePanel() { store.update(s => ({ ...s, panelOpen: false })); },
    clearHistory() { store.update(s => ({ ...s, notices: [], status: null })); },
    reset() { store.set({ notices: [], activities: [], status: null, panelOpen: false }); },
  };
}

export const feedback = createFeedback();

// Do not expire a message while the user is reading it or using another app.
export function dismissAfterReading(node: HTMLElement, dismiss: () => void, duration = 5000) {
  let remaining = duration;
  let started = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;
  function pause() {
    if (timer === undefined) return;
    clearTimeout(timer); timer = undefined;
    remaining = Math.max(0, remaining - (Date.now() - started));
  }
  function resume() {
    pause();
    if (document.hidden || !document.hasFocus() || node.matches(":hover") || node.contains(document.activeElement)) return;
    started = Date.now(); timer = setTimeout(dismiss, remaining);
  }
  const events: [EventTarget, string, EventListener][] = [
    [node, "pointerenter", pause], [node, "pointerleave", resume],
    [node, "focusin", pause], [node, "focusout", resume],
    [window, "blur", pause], [window, "focus", resume], [document, "visibilitychange", resume],
  ];
  events.forEach(([target, event, handler]) => target.addEventListener(event, handler));
  resume();
  return { destroy() { pause(); events.forEach(([target, event, handler]) => target.removeEventListener(event, handler)); } };
}
