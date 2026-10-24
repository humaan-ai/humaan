<script lang="ts">
  import CircleCheck from "@lucide/svelte/icons/circle-check";
  import CircleX from "@lucide/svelte/icons/circle-x";
  import Clock from "@lucide/svelte/icons/clock";
  import CircleStop from "@lucide/svelte/icons/circle-stop";
  import CircleQuestionMark from "@lucide/svelte/icons/circle-question-mark";
  import Hand from "@lucide/svelte/icons/hand";
  let { kind, value = "unknown" }: { kind: "outcome" | "assistance"; value?: string } = $props();
  const outcomes = {
    success: { label: "Success", icon: CircleCheck, color: "text-success" },
    failure: { label: "Failure", icon: CircleX, color: "text-error" },
    timeout: { label: "Timeout", icon: Clock, color: "text-warning" },
    aborted: { label: "Aborted", icon: CircleStop, color: "text-muted" },
    unknown: { label: "Unknown", icon: CircleQuestionMark, color: "text-muted" },
  };
  const status = $derived(kind === "outcome"
    ? outcomes[value as keyof typeof outcomes] ?? outcomes.unknown
    : value === "yes" ? { label: "Human assisted", icon: Hand, color: "text-link" }
    : value === "no" ? { label: "No assistance", icon: Hand, color: "text-muted" }
    : { label: "Assistance unknown", icon: CircleQuestionMark, color: "text-muted" });
</script>

<span class="inline-flex items-center gap-1 {status.color}">
  <span class="relative inline-flex shrink-0" aria-hidden="true">
    <status.icon size={13} />
    {#if kind === "assistance" && value === "no"}
      <svg class="absolute inset-0" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M3 3 21 21" /></svg>
    {/if}
  </span>
  {status.label}
</span>
