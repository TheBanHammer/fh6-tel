<script lang="ts">
  import { loadSessionPackets, loadSessionLaps, renameSession, setSessionBookmark, settings } from '$lib/stores/sessions';
  import { startReplay } from '$lib/stores/telemetry';
  import { carName } from '$lib/car-name';
  import type { TelemetryPacket, SessionRow, SessionLap } from '$lib/types';
  import MapPanel from './MapPanel.svelte';
  import AnalysisTab from './AnalysisTab.svelte';

  let {
    session,
    useMph = true,
    onClose,
  }: {
    session: SessionRow;
    useMph: boolean;
    onClose: () => void;
  } = $props();

  type Tab = 'analysis' | 'map' | 'replay';
  let tab = $state<Tab>('analysis');

  let packets = $state<TelemetryPacket[]>([]);
  let laps = $state<SessionLap[]>([]);
  let loading = $state(true);

  let bestLapNumber = $derived(
    laps.length
      ? laps.reduce((b, l) => (l.lapTime < b.lapTime ? l : b)).lapNumber
      : -1
  );

  // Header edit state
  let editing = $state(false);
  let draftName = $state('');
  let bookmarked = $state(session.bookmarked);

  let defaultLabel = $derived(
    `${carName(session.carOrdinal)} — ${new Date(session.startedAt).toLocaleString()}`
  );
  let displayName = $derived(session.name ?? defaultLabel);

  $effect(() => {
    loadSessionPackets(session.id).then((p) => {
      packets = p;
      loading = false;
    });
    loadSessionLaps(session.id).then((l) => (laps = l));
  });

  function formatClock(seconds: number) {
    if (!isFinite(seconds) || seconds < 0) seconds = 0;
    const m = Math.floor(seconds / 60);
    const s = (seconds % 60).toFixed(1).padStart(4, '0');
    return `${m}:${s}`;
  }

  function startEdit() {
    draftName = session.name ?? '';
    editing = true;
  }

  async function commitName() {
    editing = false;
    const v = draftName.trim();
    await renameSession(session.id, v.length ? v : null);
    session.name = v.length ? v : null;
  }

  async function toggleBookmark() {
    bookmarked = !bookmarked;
    session.bookmarked = bookmarked;
    await setSessionBookmark(session.id, bookmarked);
  }

  function beginReplay() {
    startReplay(session.id, displayName, packets);
    onClose();
  }
</script>

<div class="overlay" role="dialog" aria-modal="true">
  <div class="viewer">
    <header>
      <div class="title-area">
        {#if editing}
          <input
            class="name-input"
            bind:value={draftName}
            placeholder={defaultLabel}
            onkeydown={(e) => {
              if (e.key === 'Enter') commitName();
              if (e.key === 'Escape') (editing = false);
            }}
            onblur={commitName}
          />
        {:else}
          <button class="name-display" onclick={startEdit} title="Click to rename">
            {displayName}
            <span class="edit-hint">✎</span>
          </button>
        {/if}
        <button
          class="star"
          class:on={bookmarked}
          onclick={toggleBookmark}
          title={bookmarked ? 'Remove bookmark' : 'Bookmark'}
        >
          {bookmarked ? '★' : '☆'}
        </button>
      </div>
      <button class="close" onclick={onClose}>✕</button>
    </header>

    <div class="tabs">
      <button class:active={tab === 'analysis'} onclick={() => (tab = 'analysis')}>
        Analysis
      </button>
      <button class:active={tab === 'map'} onclick={() => (tab = 'map')}>
        Map
      </button>
      <button class:active={tab === 'replay'} onclick={() => (tab = 'replay')}>
        Replay
      </button>
    </div>

    <div class="content">
      {#if loading}
        <p class="status">Loading {session.packetCount} packets…</p>
      {:else if packets.length === 0}
        <p class="status">No telemetry recorded for this session.</p>
      {:else if tab === 'analysis'}
        <AnalysisTab {packets} {laps} {useMph} />
      {:else if tab === 'map'}
        <div class="map-tab">
          {#if $settings}
            <MapPanel points={packets} colorByLap drawLine fixedTrace settings={$settings} />
          {/if}
          <p class="replay-help">
            Driven line from recorded world position. Each colour is a separate lap.
            Configure tiles &amp; calibration in Settings → Track Map; without them
            this shows a plain vector trace.
          </p>
        </div>
      {:else}
        <div class="replay-panel">
          <div class="meta">
            <div><span>Car</span><strong>{carName(session.carOrdinal)}</strong></div>
            <div><span>Duration</span><strong>{formatClock(packets.length / 60)}</strong></div>
            <div><span>Samples</span><strong>{packets.length}</strong></div>
            <div>
              <span>Best lap</span>
              <strong>{session.bestLap ? formatClock(session.bestLap) : '—'}</strong>
            </div>
          </div>

          {#if laps.length}
            <div class="laps">
              <div class="laps-title">Lap times</div>
              {#each laps as lap}
                <div class="lap-row" class:best={lap.lapNumber === bestLapNumber}>
                  <span>Lap {lap.lapNumber + 1}</span>
                  <span class="lap-time">{formatClock(lap.lapTime)}</span>
                </div>
              {/each}
            </div>
          {/if}

          <p class="replay-help">
            Replays this session through the live dashboard with a timeline you can
            scrub, play and speed up.
          </p>
          <button class="replay-go" onclick={beginReplay}>▶ Replay on dashboard</button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
  }
  .viewer {
    width: min(900px, 94vw);
    height: min(800px, 92vh);
    background: var(--bg-panel);
    border: 1px solid var(--bd-subtle);
    border-radius: 10px;
    display: flex;
    flex-direction: column;
    box-shadow: 0 12px 48px rgba(0, 0, 0, 0.6);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 0.9rem 1.1rem;
    border-bottom: 1px solid var(--bd-dim);
  }
  .title-area {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    min-width: 0;
    flex: 1;
  }
  .name-display {
    background: none;
    border: none;
    color: var(--tx-hi);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    text-align: left;
    padding: 0.2rem 0.3rem;
    border-radius: 4px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name-display:hover {
    background: var(--bg-elevated);
  }
  .edit-hint {
    color: var(--tx-dim);
    font-size: 0.8rem;
    margin-left: 0.4rem;
  }
  .name-input {
    flex: 1;
    background: var(--bg-elevated);
    border: 1px solid var(--ac);
    color: var(--tx-hi);
    font-size: 1rem;
    padding: 0.35rem 0.5rem;
    border-radius: 4px;
  }
  .star {
    background: none;
    border: none;
    cursor: pointer;
    font-size: 1.2rem;
    color: var(--tx-dim);
    line-height: 1;
  }
  .star.on {
    color: #fbbf24;
  }
  .close {
    background: none;
    border: none;
    color: var(--tx-dim);
    font-size: 1.1rem;
    cursor: pointer;
  }
  .close:hover {
    color: var(--tx-hi);
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    padding: 0.5rem 1rem 0;
    border-bottom: 1px solid var(--bd-dim);
  }
  .tabs button {
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    color: var(--tx-dim);
    font-size: 0.85rem;
    padding: 0.5rem 0.9rem;
    cursor: pointer;
  }
  .tabs button.active {
    color: var(--tx-hi);
    border-bottom-color: var(--ac);
  }
  .content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 1rem;
  }
  .status {
    color: var(--tx-dim);
    text-align: center;
    padding: 3rem;
  }
  .map-tab {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
    height: 100%;
  }
  .map-tab :global(.track) {
    max-width: min(560px, 100%);
    flex: 1;
  }
  .replay-panel {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1.5rem;
    padding: 2rem 1rem;
  }
  .meta {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    width: 100%;
    max-width: 560px;
  }
  .meta > div {
    display: flex;
    flex-direction: column;
    align-items: center;
    background: var(--bg-card);
    border: 1px solid var(--bd-dim);
    border-radius: 8px;
    padding: 0.8rem 0.5rem;
  }
  .meta span {
    color: var(--tx-dim);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .meta strong {
    color: var(--tx-hi);
    font-size: 1rem;
    margin-top: 0.25rem;
  }
  .laps {
    width: 100%;
    max-width: 360px;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .laps-title {
    color: var(--tx-dim);
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    margin-bottom: 0.25rem;
  }
  .lap-row {
    display: flex;
    justify-content: space-between;
    padding: 0.35rem 0.6rem;
    border-radius: 5px;
    background: var(--bg-card);
    border: 1px solid var(--bd-dim);
    color: var(--tx-mid);
    font-size: 0.82rem;
  }
  .lap-row.best {
    border-color: #a855f7;
    color: #d8b4fe;
    font-weight: 700;
  }
  .lap-time {
    font-variant-numeric: tabular-nums;
  }
  .replay-help {
    color: var(--tx-lo);
    font-size: 0.85rem;
    text-align: center;
    max-width: 420px;
  }
  .replay-go {
    background: var(--ac);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 0.7rem 1.6rem;
    font-size: 0.95rem;
    font-weight: 600;
    cursor: pointer;
  }
  .replay-go:hover {
    filter: brightness(1.1);
  }
</style>
