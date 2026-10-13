<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { statusRank } from "$lib/record-sorting";
  import { createTable, createSortedRowModel, rowSortingFeature, tableFeatures, sortFn_alphanumeric, sortFn_basic, type ColumnDef } from "@tanstack/svelte-table";
  import VerificationStatus from "./VerificationStatus.svelte";
  import type { VerificationReport } from "$lib/verification";
  import ExperienceStatus from "./ExperienceStatus.svelte";
  import Eye from "@lucide/svelte/icons/eye";
  import EyeOff from "@lucide/svelte/icons/eye-off";
  import ArrowUp from "@lucide/svelte/icons/arrow-up";
  import ArrowDown from "@lucide/svelte/icons/arrow-down";
  type RecordItem = { verification?: VerificationReport; id: string; task: string; robot: string; duration_s?: number | null; source_error?: string | null };
  type Revision = { saved_at_unix_ms: number; review: { outcome: string; review_state: string; assistance: string } };
  let { records, reviews, mixedRobots, selectedId, busy, dirty, loaded, onopen }: {
    records: RecordItem[]; reviews: Record<string, { revisions: Revision[] }>;
    mixedRobots: boolean; selectedId?: string; busy: boolean; dirty: boolean; loaded: boolean;
    onopen: (id: string) => void;
  } = $props();
  const features = tableFeatures({ rowSortingFeature, sortedRowModel: createSortedRowModel(), sortFns: { alphanumeric: sortFn_alphanumeric, basic: sortFn_basic } });
  const data = $derived(records.map(record => ({ ...record, revision: latest(record.id) })));
  type TableRecord = RecordItem & { revision: Revision | undefined };
  const columns = $derived<ColumnDef<typeof features, TableRecord>[]>([
    { id: "task", accessorKey: "task", header: "Name", sortFn: "alphanumeric" },
    ...(mixedRobots ? [{ id: "robot", accessorKey: "robot", header: "Robot", sortFn: "alphanumeric" as const }] : []),
    ...([ ["outcome", "Final outcome"], ["review_state", "Review status"], ["assistance", "Human assistance"] ] as const).map(([key, header]) => ({
      id: key, header, accessorFn: (record: TableRecord) => statusRank(key, record.revision?.review[key] ?? (key === "review_state" ? "unreviewed" : "unknown")) ?? undefined,
    })),
    { id: "verification", header: "Verification", accessorFn: record => (record.verification ? { failed: 0, needs_recheck: 1, verified: 2 }[record.verification.status] : undefined), sortDescFirst: false },
    { id: "duration_s", header: "Duration", accessorFn: record => record.duration_s ?? undefined },
    { id: "reviewed_at", header: "Last reviewed", accessorFn: record => record.revision?.saved_at_unix_ms, sortDescFirst: true },
  ]);
  const table = createTable({
    features,
    get data() { return data; },
    get columns() { return columns; },
    getRowId: record => record.id,
    defaultColumn: { sortFn: "basic", sortUndefined: "last", sortDescFirst: true },
    initialState: { sorting: [] },
    enableSortingRemoval: true,
    enableMultiSort: false,
  });
  const shortIds = $derived.by(() => {
    const ids = Object.keys(reviews);
    const allIds = [...new Set([...ids, ...records.map(record => record.id)])];
    const result: Record<string, string> = {};
    for (const id of allIds) {
      let length = Math.min(8, id.length);
      while (length < id.length && allIds.some(other => other !== id && other.slice(-length) === id.slice(-length))) length++;
      result[id] = `${length < id.length ? "…" : ""}${id.slice(-length)}`;
    }
    return result;
  });
  function latest(id: string) { return reviews[id]?.revisions.at(-1); }
  function disabled(id: string) { return busy || (dirty && selectedId !== id); }
  function open(id: string) { if (!disabled(id)) onopen(id); }
  function duration(seconds: number) { return seconds < 60 ? `${seconds.toFixed(1)}s` : `${Math.floor(seconds / 60)}m ${Math.floor(seconds % 60)}s`; }
</script>

<div class="mb-2 flex h-7 items-center justify-end">
  {#if table.getAllLeafColumns().some(column => column.getIsSorted())}
    <Button variant="ghost" class="text-muted" onclick={() => table.resetSorting()}>Reset sorting</Button>
  {/if}
</div>
<div class="overflow-x-auto rounded border border-border-strong">
  <table class="w-full text-left text-xs" aria-label="Experience records">
    <thead class="bg-surface text-muted"><tr>
      {#each table.getAllLeafColumns() as column (column.id)}<th scope="col" class="whitespace-nowrap border-b border-border-strong" aria-sort={column.getIsSorted() === "desc" ? "descending" : column.getIsSorted() === "asc" ? "ascending" : "none"}>
        <Button variant="ghost" class="w-full justify-start gap-1.5 rounded-none px-4 text-left" onclick={column.getToggleSortingHandler()}>{column.columnDef.header}
          {#if column.getIsSorted()}{#if column.getIsSorted() === "desc"}<ArrowDown size={12} aria-hidden="true" />{:else}<ArrowUp size={12} aria-hidden="true" />{/if}{/if}
        </Button>
      </th>{/each}
    </tr></thead>
    <tbody>
      {#each table.getRowModel().rows as row (row.id)}
        {@const record = row.original}
        {@const revision = record.revision}
        <!-- Rows retain table semantics and support Enter/Space as well as pointer activation. -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex a11y_no_noninteractive_element_interactions -->
        <tr tabindex={disabled(record.id) ? -1 : 0} aria-disabled={disabled(record.id)} aria-current={selectedId === record.id ? "true" : undefined}
          class="border-b border-border last:border-0 focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-title/40 {disabled(record.id) ? 'opacity-45' : 'cursor-pointer hover:bg-hover'} {selectedId === record.id ? 'bg-selection' : ''}"
          onclick={(event) => { if (!(event.target instanceof Element) || !event.target.closest("button, dialog")) open(record.id); }} onkeydown={(event) => { if (event.target !== event.currentTarget) return; if (event.key === "Enter" || event.key === " ") { event.preventDefault(); open(record.id); } }}>
          <td class="min-w-64 px-4 py-3"><span class="block text-sm font-medium text-title">{record.task}</span><span class="mt-1 block max-w-sm break-all font-mono text-[10px] leading-relaxed text-muted" title={record.id}>{shortIds[record.id]}</span></td>
          {#if mixedRobots}<td class="px-4 py-3">{record.robot}</td>{/if}
          <td class="whitespace-nowrap px-4 py-3"><ExperienceStatus kind="outcome" value={revision?.review.outcome} /></td>
          <td class="whitespace-nowrap px-4 py-3"><span class="inline-flex items-center gap-1 {revision?.review.review_state === 'reviewed' ? 'text-success' : 'text-muted'}">{#if revision?.review.review_state === "reviewed"}<Eye size={13} aria-hidden="true" />Reviewed{:else}<EyeOff size={13} aria-hidden="true" />Unreviewed{/if}</span></td>
          <td class="whitespace-nowrap px-4 py-3"><ExperienceStatus kind="assistance" value={revision?.review.assistance} /></td>
          <td class="px-4 py-3"><VerificationStatus report={record.verification} id={record.id} /></td>
          <td class="whitespace-nowrap px-4 py-3 tabular-nums" title={record.source_error ?? (record.duration_s == null ? "Duration unavailable" : undefined)}>{record.duration_s == null ? "—" : duration(record.duration_s)}</td>
          <td class="whitespace-nowrap px-4 py-3 text-muted">{#if revision}<time datetime={new Date(revision.saved_at_unix_ms).toISOString()} title={new Date(revision.saved_at_unix_ms).toLocaleString()}>{new Date(revision.saved_at_unix_ms).toLocaleDateString()}</time>{:else}Not reviewed{/if}</td>
        </tr>
      {:else}<tr><td colspan={columns.length} class="px-4 py-10 text-center text-muted">{loaded ? "No records match the current filters. Import recordings or adjust the table filters." : "Catalog not loaded. Use Refresh to try again."}</td></tr>{/each}
    </tbody>
  </table>
</div>
