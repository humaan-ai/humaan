<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import { ContextMenu } from 'bits-ui';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import { canDeleteProjectFile, type ProjectEntry } from '$lib/project-files';

  let { entry, disabled = false, ondelete, children }: {
    entry: ProjectEntry;
    disabled?: boolean;
    ondelete: (entry: ProjectEntry, trigger: HTMLButtonElement | null) => void;
    children: Snippet;
  } = $props();
  let trigger = $state<HTMLDivElement | null>(null);
  async function requestDelete() {
    // Let the menu finish closing before the confirmation takes focus.
    await tick();
    ondelete(entry, trigger?.querySelector('button') ?? null);
  }
  function openFromKeyboard(event: KeyboardEvent) {
    if (entry.directory || !trigger || !(event.key === 'ContextMenu' || (event.shiftKey && event.key === 'F10'))) return;
    event.preventDefault();
    const bounds = trigger.getBoundingClientRect();
    trigger.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, cancelable: true, clientX: bounds.left + 16, clientY: bounds.bottom }));
  }
</script>

<ContextMenu.Root>
  <ContextMenu.Trigger bind:ref={trigger} disabled={entry.directory} onkeydown={openFromKeyboard}>
    {@render children()}
  </ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="z-50 min-w-40 rounded-md border border-border-strong bg-popover p-1 shadow-xl outline-none"
      onCloseAutoFocus={(event) => { event.preventDefault(); trigger?.querySelector('button')?.focus(); }}>
      <ContextMenu.Item disabled={disabled || !canDeleteProjectFile(entry)} onSelect={() => void requestDelete()}
        class="flex cursor-default items-center gap-2 rounded px-2 py-1.5 text-xs text-error outline-none data-[highlighted]:bg-selection data-[disabled]:opacity-45">
        <Trash2 size={14} aria-hidden="true" />Delete
      </ContextMenu.Item>
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
