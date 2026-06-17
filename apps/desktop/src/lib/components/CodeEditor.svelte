<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { projectState } from "$lib/project.svelte";
  import { createHighlighter, type Highlighter } from "shiki";
  import { shikiToCodeMirror } from "@cmshiki/shiki";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment } from "@codemirror/state";
  import { keymap } from "@codemirror/view";
  import { showMinimap } from "$lib/minimap";

  let editorElement = $state<HTMLDivElement | null>(null);
  let view = $state<EditorView | null>(null);
  let highlighter = $state<Highlighter | null>(null);
  
  let currentPath = $derived(projectState.selectedFile);
  
  // Compartment to dynamically swap languages when the file changes
  const languageConf = new Compartment();

  // Helper to extract file extension and match to Shiki lang
  function getLanguageFromFilename(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase();
    switch (ext) {
      case 'js':
      case 'jsx':
        return 'javascript';
      case 'ts':
      case 'tsx':
        return 'typescript';
      case 'rs':
        return 'rust';
      case 'css':
        return 'css';
      case 'html':
        return 'html';
      case 'json':
        return 'json';
      case 'md':
      case 'markdown':
        return 'markdown';
      case 'svelte':
        return 'svelte';
      case 'toml':
        return 'toml';
      case 'yaml':
      case 'yml':
        return 'yaml';
      case 'py':
        return 'python';
      case 'sh':
      case 'bash':
        return 'shell';
      default:
        return 'text';
    }
  }

  const customTheme = EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: "#1a1a1a",
      color: "#ccccccdd",
    },
    ".cm-scroller": {
      overflow: "auto",
      fontFamily: 'var(--font-mono, SF Mono, Monaco, monospace)',
      fontSize: "13px",
      lineHeight: "1.6",
    },
    ".cm-content": {
      caretColor: "#ffffff",
      padding: "16px 0",
    },
    "&.cm-focused": {
      outline: "none"
    },
    ".cm-cursor, .cm-dropCursor": {
      borderLeftColor: "#ffffff"
    },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
      backgroundColor: "#343434 !important"
    },
    ".cm-gutters": {
      backgroundColor: "#1a1a1a",
      color: "#505050",
      borderRight: "1px solid #ffffff0d",
      paddingLeft: "8px",
    },
    ".cm-gutterElement": {
      padding: "0 8px",
    },
    ".cm-activeLine": {
      backgroundColor: "#29292950"
    },
    ".cm-activeLineGutter": {
      backgroundColor: "#29292950",
      color: "#ffffff"
    },
    ".cm-minimap": {
      backgroundColor: "#141414",
      borderLeft: "1px solid #ffffff0d",
    },
    ".cm-minimap-overlay": {
      backgroundColor: "#ffffff0a",
      border: "1px solid #ffffff15",
    }
  });

  onMount(async () => {
    // 1. Initialize Shiki
    highlighter = await createHighlighter({
      themes: ["github-dark"],
      langs: [
        "javascript", "typescript", "rust", "svelte", 
        "css", "html", "json", "markdown", "toml", 
        "yaml", "python", "shell"
      ]
    });

    if (!editorElement) return;

    // 2. Initialize Shiki CodeMirror extension
    const lang = currentPath ? getLanguageFromFilename(currentPath) : "text";
    const { shiki } = await shikiToCodeMirror({
      highlighter,
      themes: {
        dark: "github-dark",
      },
      defaultColor: "dark",
      lang,
    });

    // 3. Setup keymap for saving
    const saveKeymap = keymap.of([
      {
        key: "Mod-s",
        run: () => {
          projectState.saveFile();
          return true;
        }
      }
    ]);

    // 4. Create Editor State
    const state = EditorState.create({
      doc: projectState.fileContent,
      extensions: [
        basicSetup,
        customTheme,
        saveKeymap,
        languageConf.of(shiki),
        showMinimap.compute(["doc"], () => {
          return {
            displayText: "characters",
            showOverlay: "always",
            create: () => {
              const dom = document.createElement("div");
              return { dom };
            }
          };
        }),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            const newContent = update.state.doc.toString();
            if (newContent !== projectState.fileContent) {
              projectState.fileContent = newContent;
              projectState.isDirty = true;
            }
          }
        }),
      ],
    });

    // 5. Create Editor View
    view = new EditorView({
      state,
      parent: editorElement,
    });
  });

  // Watch for external selected file changes or content loading updates
  let lastLoadedPath = "";
  $effect(() => {
    if (!view) return;
    
    // When the path changes, we also need to update the Shiki highlighting language!
    const path = currentPath || "";
    const isNewFile = path !== lastLoadedPath;
    
    if (isNewFile && path) {
      lastLoadedPath = path;
      updateLanguageAndDoc(path);
    } else if (!path) {
      // Clear document
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: "" }
      });
      lastLoadedPath = "";
    }
  });

  // Watch for loading changes, e.g. when content finishes reading from disk
  $effect(() => {
    if (!view || projectState.isLoading) return;
    const docText = view.state.doc.toString();
    if (projectState.fileContent !== docText) {
      view.dispatch({
        changes: { from: 0, to: view.state.doc.length, insert: projectState.fileContent }
      });
    }
  });

  async function updateLanguageAndDoc(path: string) {
    if (!view || !highlighter) return;

    const lang = getLanguageFromFilename(path);
    
    // Re-create the Shiki CodeMirror extension for the new language
    const { shiki } = await shikiToCodeMirror({
      highlighter,
      themes: {
        dark: "github-dark",
      },
      defaultColor: "dark",
      lang,
    });

    // Dispatch configuration change to swap Shiki lang extension and set new doc content
    view.dispatch({
      effects: languageConf.reconfigure(shiki),
      changes: { from: 0, to: view.state.doc.length, insert: projectState.fileContent }
    });
  }

  onDestroy(() => {
    if (view) {
      view.destroy();
    }
  });
</script>

<div class="relative flex-1 h-full w-full overflow-hidden">
  {#if projectState.isLoading}
    <div class="absolute inset-0 bg-background/50 flex items-center justify-center z-10">
      <span class="text-xs text-muted animate-pulse">Loading file...</span>
    </div>
  {/if}
  
  <div bind:this={editorElement} class="h-full w-full"></div>
</div>

<style>
  :global(.cm-editor) {
    outline: none !important;
  }
</style>
