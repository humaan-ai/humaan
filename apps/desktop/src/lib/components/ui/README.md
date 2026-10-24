# Shared workbench controls

These components are sourced from shadcn-svelte (Nova), with Bits UI handling
accessible interactions. `components.json` records the registry configuration.
Keep local theme changes when updating generated components.

Use `Button`, `Input`, `Textarea`, `NativeSelect`, `Checkbox`, and the `Dialog`,
`Tabs`, and `Tooltip` namespaces from this directory. Use `IconButton` for
icon-only actions and always provide an accessible label. Bind DOM references
with `bind:ref` and initialize them to `null`, rather than `undefined`.

- Standard buttons and single-line fields: 32px; toolbar icon buttons: 28px.
- Activity-bar buttons use `size="icon-lg"` (40px); status-bar controls may use
  20px to fit the existing footer. Cards and list rows can use `h-auto`.
- Neutral actions use the default `outline` button variant. Primary actions
  explicitly use `variant="default"`; icon actions use `ghost`; destructive
  actions use `destructive`.
- Palette and radius tokens live in `src/app.css`. Change component styles here,
  rather than adding screen-specific border, padding, focus, or disabled rules.
- Pass layout classes (width, margin, alignment) at call sites. Keep form labels,
  disabled conditions, and submit-button types explicit.
- Dialog content is conditionally mounted while open for immediate dismissal
  and reliable overlay cleanup. Preserve operation guards and focus restoration.
- Bits UI state selectors use `data-[state=...]` and
  `data-[orientation=...]`. Do not assume shorthand variants are defined.
- Workspace tabs retain their existing close-tab/navigation logic with shared
  buttons; settings tabs use the Bits UI-backed Tabs components.
- The status notice retains a native button for its Svelte action, but obtains
  its appearance from the shared `buttonVariants` function.
