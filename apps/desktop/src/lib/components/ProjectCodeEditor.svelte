<script lang="ts">
  import { onMount } from 'svelte';
  import { basicSetup } from 'codemirror';
  import { Compartment, EditorState, Prec } from '@codemirror/state';
  import { EditorView } from '@codemirror/view';
  import { LanguageDescription } from '@codemirror/language';
  import { languages } from '@codemirror/language-data';
  import { oneDark } from '@codemirror/theme-one-dark';

  let { name, content, readOnly, onchange }: {
    name: string; content: string; readOnly: boolean; onchange: (value: string) => void;
  } = $props();
  let host: HTMLDivElement;
  let minimap: HTMLCanvasElement;
  let scrollPercent = $state(0);
  let navigateMap: (fraction: number) => void = () => {};
  function point(event: PointerEvent) {
    const bounds = minimap.getBoundingClientRect();
    navigateMap((event.clientY - bounds.top) / bounds.height);
  }
  onMount(() => {
    const language = new Compartment();
    let frame = 0;
    let lastScroll = -Infinity;
    let fadeTimer: ReturnType<typeof setTimeout> | undefined;
    function onScroll() {
      lastScroll = performance.now();
      clearTimeout(fadeTimer);
      fadeTimer = setTimeout(scheduleMap, 700);
      scheduleMap();
    }
    function scheduleMap() { cancelAnimationFrame(frame); frame = requestAnimationFrame(drawMap); }
    function drawMap() {
      const width = minimap.clientWidth, height = minimap.clientHeight;
      if (!width || !height) return;
      const ratio = window.devicePixelRatio || 1;
      minimap.width = width * ratio; minimap.height = height * ratio;
      const ctx = minimap.getContext('2d'); if (!ctx) return;
      ctx.scale(ratio, ratio); ctx.clearRect(0, 0, width, height);
      const doc = view.state.doc;
      const step = Math.max(1, Math.ceil(doc.lines / (height / 2)));
      const lineHeight = Math.min(3, height / Math.ceil(doc.lines / step));
      ctx.font = '2px monospace';
      for (let line = 1, row = 0; line <= doc.lines; line += step, row++) {
        const text = doc.line(line).text.slice(0, 140);
        ctx.fillStyle = /^\s*(#|\/\/)/.test(text) ? '#7296b5' : '#929da9';
        ctx.fillText(text, 4, row * lineHeight + 3, width - 8);
      }
      const scroll = view.scrollDOM;
      const total = Math.max(scroll.scrollHeight, scroll.clientHeight);
      const thumbHeight = Math.max(12, height * scroll.clientHeight / total);
      const maxScroll = Math.max(1, total - scroll.clientHeight);
      scrollPercent = Math.round(scroll.scrollTop / maxScroll * 100);
      const top = (height - thumbHeight) * scroll.scrollTop / maxScroll;
      const opacity = Math.max(0, Math.min(1, (lastScroll + 900 - performance.now()) / 200));
      if (opacity > 0 && total > scroll.clientHeight) {
        ctx.globalAlpha = opacity;
        ctx.fillStyle = '#ffffff12'; ctx.fillRect(0, top, width, thumbHeight);
        ctx.strokeStyle = '#ffffff24'; ctx.strokeRect(.5, top + .5, width - 1, thumbHeight - 1);
        ctx.globalAlpha = 1;
        if (performance.now() >= lastScroll + 700) scheduleMap();
      }
    }
    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: content,
        extensions: [
          basicSetup, oneDark, language.of([]), EditorState.readOnly.of(readOnly),
          EditorView.contentAttributes.of({ 'aria-label': `Editor ${name}`, 'aria-readonly': String(readOnly) }),
          Prec.high(EditorView.theme({
            '&': { height: '100%', backgroundColor: '#181818', color: '#d4d4d4', fontSize: '13px' },
            '.cm-scroller': { overflow: 'auto', fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace', lineHeight: '20px' },
            '.cm-content': { padding: '12px 0', caretColor: '#fff' },
            '.cm-gutters': { backgroundColor: '#181818', color: '#6e7681', border: 'none' },
            '.cm-lineNumbers .cm-gutterElement': { minWidth: '40px', padding: '0 12px 0 8px' },
            '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: '#ffffff06' },
            '&.cm-focused': { outline: 'none' },
            '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: '#264f78' },
          }, { dark: true })),
          EditorView.updateListener.of(update => { if (update.docChanged) onchange(update.state.doc.toString()); if (update.docChanged || update.viewportChanged || update.geometryChanged) scheduleMap(); }),
        ],
      }),
    });
    navigateMap = fraction => {
      const scroll = view.scrollDOM;
      scroll.scrollTop = Math.max(0, Math.min(1, fraction)) * (scroll.scrollHeight - scroll.clientHeight);
      scheduleMap();
    };
    view.scrollDOM.addEventListener('scroll', onScroll);
    const observer = new ResizeObserver(scheduleMap); observer.observe(host);
    scheduleMap();
    let disposed = false;
    const filename = /\.(urdf|xacro|mjcf|dae)$/i.test(name) ? 'robot.xml' : name;
    const support = LanguageDescription.matchFilename(languages, filename);
    if (support) void support.load().then(extension => {
      if (!disposed) view.dispatch({ effects: language.reconfigure(extension) });
    }).catch(() => { /* Plain text remains editable if an optional language cannot load. */ });
    return () => { disposed = true; clearTimeout(fadeTimer); observer.disconnect(); cancelAnimationFrame(frame); view.scrollDOM.removeEventListener('scroll', onScroll); view.destroy(); };
  });
</script>

<div class="flex min-h-0 flex-1 overflow-hidden bg-[#181818]">
  <div bind:this={host} class="min-h-0 min-w-0 flex-1 overflow-hidden" data-editor-surface></div>
  <canvas bind:this={minimap} class="h-full w-24 shrink-0 touch-none cursor-pointer focus-visible:outline-1 focus-visible:outline-ring" role="slider" tabindex="0" aria-label="Editor minimap" aria-valuemin={0} aria-valuemax={100} aria-valuenow={scrollPercent} aria-orientation="vertical"
    onpointerdown={(event) => { minimap.setPointerCapture(event.pointerId); point(event); }}
    onpointermove={(event) => { if (minimap.hasPointerCapture(event.pointerId)) point(event); }}
    onkeydown={(event) => {
      const delta = event.key === 'ArrowDown' ? 5 : event.key === 'ArrowUp' ? -5 : 0;
      if (delta || event.key === 'Home' || event.key === 'End') {
        event.preventDefault(); navigateMap(event.key === 'Home' ? 0 : event.key === 'End' ? 1 : (scrollPercent + delta) / 100);
      }
    }}></canvas>
</div>
