<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import type { Snippet } from 'svelte';

  let {
    id,
    title,
    defaultWidth = 200,
    defaultTop,
    defaultBottom,
    resizable = false,
    hidden = false,
    onClose,
    actions,
    children,
  }: {
    id: string;
    title: string;
    defaultWidth?: number;
    defaultTop?: number;
    defaultBottom?: number;
    resizable?: boolean;
    hidden?: boolean;
    onClose: () => void;
    actions?: Snippet;
    children: Snippet;
  } = $props();

  let x = $state(0);
  let y = $state(0);
  let w = $state(defaultWidth);
  let ready = $state(false);

  onMount(() => {
    const saved = localStorage.getItem(id);
    if (saved) {
      try {
        const s = JSON.parse(saved) as { x: number; y: number; w: number };
        x = s.x; y = s.y; w = s.w;
      } catch { /* fall through to default */ }
    }
    if (!saved) {
      w = defaultWidth;
      x = window.innerWidth - defaultWidth;
      y = defaultTop ?? window.innerHeight - defaultWidth - (defaultBottom ?? 0);
    }
    ready = true;
  });

  function persist() {
    localStorage.setItem(id, JSON.stringify({ x, y, w }));
  }

  // ── Drag ─────────────────────────────────────────────────────────────────
  let dragging = $state(false);
  let dragStartX = 0, dragStartY = 0, originX = 0, originY = 0;

  function startDrag(e: PointerEvent) {
    e.preventDefault();
    dragging = true;
    dragStartX = e.clientX; dragStartY = e.clientY;
    originX = x; originY = y;
    window.addEventListener('pointermove', onDragMove);
    window.addEventListener('pointerup', stopDrag, { once: true });
  }

  function onDragMove(e: PointerEvent) {
    x = Math.max(0, Math.min(window.innerWidth - w, originX + e.clientX - dragStartX));
    y = Math.max(0, Math.min(window.innerHeight - 40, originY + e.clientY - dragStartY));
  }

  function stopDrag() {
    dragging = false;
    window.removeEventListener('pointermove', onDragMove);
    persist();
  }

  // ── Resize (width only — map uses aspect-ratio:1 so height follows) ──────
  let resizing = $state(false);
  let resizeStartX = 0, originW = 0;

  function startResize(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    resizing = true;
    resizeStartX = e.clientX; originW = w;
    window.addEventListener('pointermove', onResizeMove);
    window.addEventListener('pointerup', stopResize, { once: true });
  }

  function onResizeMove(e: PointerEvent) {
    w = Math.max(120, Math.min(window.innerWidth - x, originW + e.clientX - resizeStartX));
  }

  function stopResize() {
    resizing = false;
    window.removeEventListener('pointermove', onResizeMove);
    persist();
  }

  onDestroy(() => {
    window.removeEventListener('pointermove', onDragMove);
    window.removeEventListener('pointermove', onResizeMove);
  });
</script>

{#if ready}
  <div
    class="fp"
    class:dragging
    style="left:{x}px; top:{y}px; width:{w}px;{hidden ? ' display:none;' : ''}"
    role="dialog"
    aria-label={title}
  >
    <div class="fp-header" onpointerdown={startDrag}>
      <span class="fp-grip" aria-hidden="true">⠿</span>
      <span class="fp-title">{title}</span>
      <div class="fp-actions" onpointerdown={(e) => e.stopPropagation()}>
        {#if actions}{@render actions()}{/if}
        <button class="fp-close" onclick={onClose} aria-label="Close {title}">✕</button>
      </div>
    </div>
    <div class="fp-body">
      {@render children()}
    </div>
    {#if resizable}
      <div class="fp-resize" onpointerdown={startResize} aria-hidden="true">
        <svg viewBox="0 0 10 10" width="10" height="10" aria-hidden="true">
          <path d="M9 1L1 9M9 5L5 9" stroke="currentColor" stroke-width="1.5"/>
        </svg>
      </div>
    {/if}
  </div>
{/if}

<style>
  .fp {
    position: fixed;
    z-index: 50;
    background: var(--bg-panel);
    border: 1px solid var(--bd-subtle);
    border-radius: 6px;
    box-shadow: 0 4px 20px rgba(0,0,0,0.5);
    display: flex;
    flex-direction: column;
    isolation: isolate;
    min-width: 120px;
  }
  .fp.dragging { opacity: 0.9; cursor: grabbing; user-select: none; }
  .fp-header {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.25rem 0.4rem;
    border-bottom: 1px solid var(--bd-dim);
    cursor: grab;
    background: var(--bg-elevated);
    border-radius: 6px 6px 0 0;
    user-select: none;
    flex-shrink: 0;
  }
  .fp-grip {
    color: var(--tx-xdim);
    font-size: 0.75rem;
    line-height: 1;
  }
  .fp-title {
    flex: 1;
    font-size: 0.55rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    color: var(--tx-dim);
  }
  .fp-actions {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }
  .fp-close {
    background: none;
    border: none;
    color: var(--tx-xdim);
    font-size: 0.65rem;
    cursor: pointer;
    padding: 0 0.1rem;
    line-height: 1;
  }
  .fp-close:hover { color: var(--tx-hi); }
  .fp-body { flex: 1; min-height: 0; overflow: hidden; }
  .fp-resize {
    position: absolute;
    bottom: 3px;
    right: 4px;
    width: 12px;
    height: 12px;
    cursor: se-resize;
    color: var(--tx-xdim);
    opacity: 0.6;
  }
  .fp-resize:hover { opacity: 1; }
</style>
