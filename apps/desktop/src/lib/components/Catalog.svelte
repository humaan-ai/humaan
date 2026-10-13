<script lang="ts">
  import { toast } from 'svelte-sonner';
  import { studio } from '$lib/studio.svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { NativeSelect } from '$lib/components/ui/native-select/index.js';
  import * as Dialog from '$lib/components/ui/dialog/index.js';
  import Blocks from '@lucide/svelte/icons/blocks';
  import Search from '@lucide/svelte/icons/search';
  import Download from '@lucide/svelte/icons/download';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Settings from '@lucide/svelte/icons/settings';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import Box from '@lucide/svelte/icons/box';
  import Check from '@lucide/svelte/icons/check';
  import StudioSidebar from './StudioSidebar.svelte';
  import WorkspaceColumn from './WorkspaceColumn.svelte';
  import PartThumbnail from './PartThumbnail.svelte';
  import BuilderViewport from './BuilderViewport.svelte';
  import { sampleCatalog, filtered, type Entry } from '$lib/catalog/model';
  import type { Robot } from '$lib/builder/model';
  let items = $state(sampleCatalog());
  let query = $state(''), kind = $state('all'), availability = $state('all');
  let selected = $state<Entry | null>(null), preview = $state<Robot | null>(null), selectedLink = $state('');
  let settingsOpen = $state(false);
  const visible = $derived(filtered(items, query, kind, availability));
  const kinds = [{ id: 'all', label: 'All items' }, { id: 'part', label: 'Parts' }, { id: 'component', label: 'Components' }, { id: 'module', label: 'Modules' }, { id: 'robot', label: 'Robots' }];
  function open(entry: Entry) { selected = entry; preview = entry.robot; selectedLink = ''; }
  function download() { if (!selected) return; selected.downloaded = true; toast.success('Download preview complete. No files were downloaded.'); }
  function addToBuilder() { toast.info('Builder integration is coming next. This sample does not change your project.'); }
  function sync() { toast.info('Cloud synchronization is not connected in this preview.'); }
</script>
<div class="flex h-full min-h-0 gap-1">
  <StudioSidebar title="Catalog">
    <div class="px-3 pb-3 pt-2"><div class="relative"><Search size={14} class="pointer-events-none absolute left-2.5 top-2.5 text-muted"/><Input class="w-full pl-8" aria-label="Search catalog" placeholder="Search catalog" bind:value={query} oninput={() => { selected = null; preview = null; }}/></div></div>
    <nav aria-label="Catalog categories" class="space-y-1 px-2">{#each kinds as category}<button class="flex h-8 w-full items-center justify-between rounded px-2 text-xs hover:bg-hover {kind === category.id ? 'bg-selection text-title' : 'text-muted'}" aria-pressed={kind === category.id} onclick={() => { kind = category.id; selected = null; preview = null; }}><span>{category.label}</span><span class="text-[10px] text-muted">{items.filter(e => category.id === 'all' || e.item.kind === category.id).length}</span></button>{/each}</nav>
    <div class="mt-4 border-t border-border p-3 text-xs"><label>Availability<NativeSelect class="mt-2 w-full" aria-label="Catalog availability" bind:value={availability} onchange={() => { selected = null; preview = null; }}><option value="all">All items</option><option value="offline">Downloaded · Demo</option></NativeSelect></label></div>
    <div class="mt-auto border-t border-border p-3 text-xs text-muted"><div class="flex items-center justify-between"><span>Humaan Cloud</span><Button size="icon-sm" variant="ghost" aria-label="Catalog connection settings" onclick={() => settingsOpen = true}><Settings size={14}/></Button></div><p class="mt-2 leading-relaxed">Preview collection · Cloud connection coming soon.</p></div>
  </StudioSidebar>
  <WorkspaceColumn active={studio.activeView === 'catalog'}>
    <section class="workbench-panel relative flex min-h-0 flex-1 flex-col" aria-label="Catalog workspace">
      <header class="flex h-[35px] shrink-0 items-center gap-2 border-b border-border px-3 text-xs"><Blocks size={15}/><span class="font-medium text-title">Catalog</span>{#if selected}<span class="truncate text-muted">/ {selected.item.name}</span>{/if}</header>
      <div class="flex min-h-[57px] shrink-0 items-center gap-3 border-b border-border px-4 py-3">{#if selected}<Button variant="outline" onclick={() => { selected = null; preview = null; }}><ArrowLeft size={14}/>All items</Button>{:else}<span class="text-xs text-muted">{visible.length} {visible.length === 1 ? 'item' : 'items'}</span>{/if}<div class="ml-auto flex gap-2"><Button variant="outline" size="icon-sm" aria-label="Refresh catalog" onclick={sync}><RefreshCw size={14}/></Button></div></div>
      <div class="border-b border-border px-4 py-2 text-xs text-muted"><span class="mr-2 rounded bg-selection px-1.5 py-0.5 text-title">Preview</span>Sample items · Downloads, Builder integration, and supplier links are demonstrations.</div>
      <div class="min-h-0 flex-1 overflow-auto">
        {#if selected}
          <div class="grid gap-6 p-5 xl:grid-cols-[minmax(0,1.2fr)_minmax(280px,1fr)]">
            <div class="min-w-0"><div class="flex h-80 overflow-hidden rounded-lg border border-border bg-[#181818]">{#if preview}{#key selected.key}<BuilderViewport robot={preview} selected={selectedLink} onselect={id => selectedLink = id}/>{/key}{:else}<div class="m-auto text-center text-xs text-muted"><Blocks size={36} class="mx-auto mb-4"/>Download this version to inspect its 3D model.</div>{/if}</div>{#if preview}<p class="mt-3 text-xs text-muted">{preview.links.length} {preview.links.length === 1 ? 'link' : 'links'} · {preview.joints.length} joints · {Object.keys(preview.assets).length} mesh assets</p>{/if}</div>
            <div class="min-w-0"><div class="flex items-center gap-2 text-xs text-muted"><span class="capitalize">{selected.item.kind}</span><span>·</span><span>{selected.item.version}</span>{#if selected.downloaded}<span class="ml-auto flex items-center gap-1"><Check size={12}/>Downloaded · Demo</span>{/if}</div><h1 class="mt-3 text-xl font-medium text-title">{selected.item.name}</h1><p class="mt-2 text-xs text-muted">{selected.item.publisher}</p><p class="mt-5 whitespace-pre-wrap text-sm leading-relaxed">{selected.item.description}</p>
              <div class="mt-6 flex flex-wrap gap-2">{#if !selected.downloaded}<Button onclick={download}><Download size={14}/>Download {selected.item.version}</Button>{:else}<Button onclick={addToBuilder}><Box size={14}/>Add to project</Button>{/if}</div>
              <dl class="mt-6 space-y-4 border-t border-border pt-4 text-xs"><div><dt class="text-muted">License</dt><dd class="mt-1 text-title">{selected.item.license}</dd></div><div><dt class="text-muted">Compatibility · publisher supplied</dt><dd class="mt-1">{#if selected.item.compatibility.length}<ul class="space-y-1">{#each selected.item.compatibility as note}<li>{note}</li>{/each}</ul>{:else}No compatibility information provided.{/if}</dd></div><div><dt class="text-muted">Source</dt><dd class="mt-1 break-all">Humaan Cloud · Sample collection</dd></div></dl>
              {#if selected.item.tags.length}<div class="mt-4 flex flex-wrap gap-1">{#each selected.item.tags as tag}<span class="rounded border border-border px-2 py-1 text-[10px] text-muted">{tag}</span>{/each}</div>{/if}
              <div class="mt-6 border-t border-border pt-4"><h2 class="text-sm font-medium text-title">Hardware</h2><p class="mt-2 text-xs leading-relaxed text-muted">{selected.item.suppliers.length ? 'Purchase physical hardware from the listed suppliers. Digital downloads do not include hardware.' : 'No supplier links provided for this item.'}</p>{#each selected.item.suppliers as supplier}<Button class="mt-3 mr-2" variant="outline" onclick={() => toast.info('Supplier links will be available when Humaan Cloud is connected.')}>{supplier.name}<ExternalLink size={13}/></Button>{/each}</div>
            </div>
          </div>
        {:else if visible.length}
          <div class="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-4 p-5">{#each visible as entry (entry.key)}<button class="overflow-hidden rounded-lg border border-border bg-[#181818] text-left transition-colors hover:border-muted focus-visible:outline-2 focus-visible:outline-ring" onclick={() => open(entry)} aria-label={`View ${entry.item.name} ${entry.item.version}`}>
            <PartThumbnail name={entry.item.name} load={() => Promise.resolve(entry.robot)}/>
            <div class="border-t border-border p-4"><div class="flex justify-between gap-2 text-[10px] text-muted"><span class="capitalize">{entry.item.kind}</span><span>{entry.downloaded ? 'Downloaded · Demo' : 'Sample'}</span></div><h2 class="mt-2 truncate text-sm font-medium text-title">{entry.item.name}</h2><p class="mt-1 truncate text-xs text-muted">{entry.item.publisher} · {entry.item.version}</p><p class="mt-3 line-clamp-2 min-h-8 text-xs leading-4 text-muted">{entry.item.description}</p></div>
          </button>{/each}</div>
        {:else}<div class="flex min-h-80 flex-col items-center justify-center p-10 text-center"><Blocks size={38} strokeWidth={1} class="text-muted"/><h1 class="mt-5 text-lg font-medium text-title">No matching items</h1><p class="mt-3 text-sm text-muted">Try another search, category, or availability filter.</p><Button class="mt-6" variant="outline" onclick={() => { query = ''; kind = 'all'; availability = 'all'; }}>Clear filters</Button></div>{/if}
      </div>
    </section>
  </WorkspaceColumn>
</div>
<Dialog.Root bind:open={settingsOpen}><Dialog.Content><Dialog.Header><Dialog.Title>Connect Humaan Cloud</Dialog.Title><Dialog.Description>Discover community modules, download versioned packages, and find matching hardware from suppliers.</Dialog.Description></Dialog.Header><div class="rounded-md border border-border bg-hover p-4 text-sm"><p class="font-medium text-title">Coming soon</p><p class="mt-2 leading-relaxed text-muted">This Catalog is an interactive preview. Account connection, synchronization, and downloads will be available when the Humaan Cloud service is connected.</p></div><Dialog.Footer><Button onclick={() => settingsOpen = false}>Got it</Button></Dialog.Footer></Dialog.Content></Dialog.Root>
