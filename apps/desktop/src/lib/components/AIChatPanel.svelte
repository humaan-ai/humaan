<script lang="ts">
  import IconButton from "$lib/components/ui/icon-button.svelte";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Textarea } from "$lib/components/ui/textarea/index.js";
  import PanelResizeHandle from "./PanelResizeHandle.svelte";
  import Plus from "@lucide/svelte/icons/plus";
  import MessageSquare from "@lucide/svelte/icons/message-square";
  import Sparkles from "@lucide/svelte/icons/sparkles";
  import Maximize from "@lucide/svelte/icons/maximize";
  import Minimize from "@lucide/svelte/icons/minimize";
  import X from "@lucide/svelte/icons/x";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import Folder from "@lucide/svelte/icons/folder";
  import { studio } from "$lib/studio.svelte";
  import { onMount, tick } from "svelte";

  type ChatSession = { id: string; project: string; draft: string; updatedAt: number };
  let sessions = $state<ChatSession[]>([]);
  let selected = $state<Record<string, string>>({});
  let historyReady = $state(false);
  let storageError = $state("");
  const storageKey = "humaan.chat-sessions.v1";
  const projectSessions = $derived(sessions.filter(session => session.project === projectKey).toSorted((a, b) => b.updatedAt - a.updatedAt));
  const activeSession = $derived(projectSessions.find(session => session.id === selected[projectKey]) ?? projectSessions[0]);
  onMount(() => {
    try {
      const saved: unknown = JSON.parse(localStorage.getItem(storageKey) ?? "[]");
      if (!Array.isArray(saved) || !saved.every(session => session && typeof session.id === "string" && typeof session.project === "string" && typeof session.draft === "string" && typeof session.updatedAt === "number" && Number.isFinite(session.updatedAt))) throw new Error("Invalid chat history");
      sessions = saved;
      historyReady = true;
    } catch { storageError = "Saved sessions could not be loaded. New drafts are kept in this window only."; }
  });
  $effect(() => {
    const saved = JSON.stringify(sessions);
    if (!historyReady) return;
    try { localStorage.setItem(storageKey, saved); storageError = ""; }
    catch { storageError = "Sessions could not be saved. Drafts are kept in this window only."; }
  });
  function newSession() {
    const session = { id: crypto.randomUUID(), project: projectKey, draft: "", updatedAt: Date.now() };
    sessions.push(session);
    selected[projectKey] = session.id;
    return sessions[sessions.length - 1];
  }
  function updateDraft(draft: string) {
    const session = activeSession ?? newSession();
    session.draft = draft;
    session.updatedAt = Date.now();
  }
  async function startSession() {
    newSession();
    await tick();
    composer?.focus();
  }
  function sessionTitle(session: ChatSession) { return session.draft.trim().split("\n")[0] || "New chat"; }

  const projectKey = $derived(studio.project?.root ?? "welcome");
  let composer = $state<HTMLTextAreaElement | null>(null);
  let panel = $state<HTMLElement>();
  let expandButton = $state<HTMLButtonElement | null>(null);
  const suggestions = [
    { label: "Explore this project", prompt: "Give me an overview of this project and its experiences." },
    { label: "Review experiences", prompt: "Help me identify experiences that need review." },
    { label: "Plan a learning set", prompt: "Help me plan a learning set for this project." },
  ];
  async function useSuggestion(prompt: string) {
    updateDraft(prompt);
    await tick();
    composer?.focus();
  }
  function close() {
    studio.chatExpanded = false;
    studio.chatOpen = false;
    document.getElementById("chat-toggle")?.focus();
  }
  function handleKeydown(event: KeyboardEvent) {
    if (!studio.chatOpen || !studio.chatExpanded) return;
    if (event.key === "Escape" && event.target instanceof Node && panel?.contains(event.target)) {
      event.preventDefault();
      studio.chatExpanded = false;
      expandButton?.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#if studio.chatOpen}
  <aside bind:this={panel} class:immersive={studio.chatExpanded} id="ai-chat-panel" style:--chat-width="{studio.chatWidth}px" class="workbench-panel chat-panel relative flex min-h-0 shrink-0 flex-col text-sm" aria-label="Chat">
    {#if !studio.chatExpanded}<div class="chat-resizer"><PanelResizeHandle label="Resize AI chat" width={studio.chatWidth} min={260} max={420} edge="left" onresize={(width) => studio.chatWidth = width} /></div>{/if}
    <div class="flex min-h-0 flex-1 flex-col overflow-hidden rounded-[inherit]">
    <div class="flex min-h-0 flex-1">
    <div class="flex min-h-0 min-w-0 flex-1 flex-col">
    <header class="flex h-[35px] shrink-0 items-center justify-between px-4">
      <h2 class="text-xs font-medium">Chat</h2>
      <div class="flex items-center gap-1">
        <IconButton bind:ref={expandButton} class="text-muted" aria-label={studio.chatExpanded ? "Restore chat panel" : "Expand chat to workspace"} title={studio.chatExpanded ? "Restore chat panel (Esc)" : "Expand chat to workspace"} aria-pressed={studio.chatExpanded} onclick={() => studio.chatExpanded = !studio.chatExpanded}>
          {#if studio.chatExpanded}<Minimize size={15} aria-hidden="true" />{:else}<Maximize size={15} aria-hidden="true" />{/if}
        </IconButton>
      <IconButton class="text-muted" aria-label="Close chat" title="Close chat" onclick={close}><X size={15} aria-hidden="true" /></IconButton>
      </div>
    </header>
    <div class="chat-content flex min-h-0 flex-1 flex-col overflow-y-auto px-5 py-6">
      <div class="my-auto py-6">
        <div class="mb-4 flex h-10 w-10 items-center justify-center rounded-xl border border-border-strong bg-raised text-title"><Sparkles size={21} strokeWidth={1.5} aria-hidden="true" /></div>
        <h3 class="text-lg font-medium tracking-tight">A little help, right here.</h3>
        <p class="mt-2 text-xs leading-5 text-muted">Explore your experiences, review recordings, and plan what your robot learns next.</p>
        <div class="mt-6 flex flex-col gap-2">
          {#each suggestions as suggestion}
            <Button class="justify-start text-left" onclick={() => useSuggestion(suggestion.prompt)}>{suggestion.label}</Button>
          {/each}
        </div>
      </div>
    </div>
    <div class="chat-composer shrink-0 p-3 pt-0">
      <div class="mb-2 flex min-w-0 items-center gap-1.5 px-1 text-xs text-muted" title={studio.project?.project.name}>
        <Folder size={13} class="shrink-0" aria-hidden="true" />
        <span class="truncate">{studio.project?.project.name ?? "No project open"}</span>
      </div>
      <div class="rounded-xl border border-border-strong bg-raised p-3 focus-within:border-accent">
        <Textarea bind:ref={composer} aria-label="Message AI assistant" aria-describedby="chat-connection-status" placeholder="Ask about your project…" rows={3} value={activeSession?.draft ?? ""} oninput={(event) => updateDraft(event.currentTarget.value)} class="max-h-48 border-0 bg-transparent p-0 leading-5 focus-visible:ring-0"></Textarea>
        <div class="mt-2 flex items-center justify-between gap-2">
          <span class="text-[11px] text-muted">AI assistant</span>
          <IconButton disabled aria-label="Send message (AI provider not connected)" title="Connect an AI provider to send messages" class="flex   items-center justify-center bg-selection text-muted opacity-50"><ArrowUp size={16} aria-hidden="true" /></IconButton>
        </div>
      </div>
      <p id="chat-connection-status" class="mt-2 px-1 text-[11px] leading-4 text-muted">AI provider not connected. You can draft a message here; sending isn’t available yet.</p>
    </div>
    </div>
    {#if studio.chatExpanded}
      <aside class="session-history flex min-h-0 shrink-0 flex-col border-l border-border bg-surface/40" aria-label="Chat session history">
        <header class="flex h-[35px] shrink-0 items-center justify-between gap-2 px-3">
          <h3 class="text-xs font-medium">Sessions</h3>
          <IconButton class="text-muted" aria-label="New chat session" title="New chat session" onclick={startSession}><Plus size={15} aria-hidden="true" /></IconButton>
        </header>
        <div class="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
          {#each projectSessions as session (session.id)}
            <Button variant="ghost" class="mb-1 h-auto whitespace-normal flex w-full items-start gap-2 px-2 py-2.5 text-left {activeSession?.id === session.id ? 'bg-selection' : 'hover:bg-hover'}" aria-current={activeSession?.id === session.id ? "true" : undefined} title={sessionTitle(session)} onclick={() => selected[projectKey] = session.id}>
              <MessageSquare size={14} class="mt-0.5 shrink-0 text-muted" aria-hidden="true" />
              <span class="min-w-0"><span class="block truncate text-xs">{sessionTitle(session)}</span><span class="mt-1 block text-[11px] text-muted">Draft · {new Date(session.updatedAt).toLocaleDateString(undefined, { month: "short", day: "numeric" })}</span></span>
            </Button>
          {:else}
            <div class="px-2 py-5 text-xs leading-5 text-muted"><p>No sessions yet.</p><p class="mt-1">Start a chat or draft a message to keep it here.</p><Button variant="ghost" class="mt-3 text-foreground" onclick={startSession}>New chat</Button></div>
          {/each}
        </div>
        <p class="px-3 pb-3 text-[11px] leading-4 text-muted" role="status">{storageError || "Saved on this device · Current project"}</p>
      </aside>
    {/if}
    </div>
    </div>
  </aside>
{/if}

<style>
  .session-history { width: clamp(180px, 22%, 240px); }
  @media (max-width: 600px) { .session-history { width: 140px; } }
  .chat-panel { width: var(--chat-width); }
  @media (max-width: 900px) {
    .chat-resizer { display: none; }
    .chat-panel { position: absolute; z-index: 20; right: 4px; top: 0; bottom: 4px; width: min(300px, calc(100% - 8px)); box-shadow: -8px 0 28px #0005; }
  }
  .chat-panel.immersive { position: relative; inset: auto; z-index: auto; width: auto; min-width: 0; flex: 1; box-shadow: none; }
  .immersive .chat-content, .immersive .chat-composer { width: 100%; max-width: 760px; margin-inline: auto; }
  .immersive .chat-composer { padding-bottom: 24px; }
</style>
