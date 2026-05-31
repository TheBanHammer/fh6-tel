<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import MapPanel from '$lib/components/MapPanel.svelte';
  import { loadSettings, settings } from '$lib/stores/sessions';
  import type { TelemetryPacket } from '$lib/types';

  type MapState = {
    type: 'map-state';
    pts: TelemetryPacket[];
    idx: number;
    drawLine: boolean;
    colorByLap: boolean;
    fixedTrace: boolean;
  };

  let pts = $state<TelemetryPacket[]>([]);
  let idx = $state(-1);
  let drawLine = $state(false);
  let colorByLap = $state(false);
  let fixedTrace = $state(false);
  let bc: BroadcastChannel;

  onMount(async () => {
    await loadSettings();
    bc = new BroadcastChannel('fh6-tel-map');
    bc.onmessage = (e: MessageEvent<MapState>) => {
      const d = e.data;
      if (d.type !== 'map-state') return;
      pts = d.pts;
      idx = d.idx;
      drawLine = d.drawLine;
      colorByLap = d.colorByLap;
      fixedTrace = d.fixedTrace;
    };
  });

  onDestroy(() => bc?.close());
</script>

<svelte:head><title>Track Map — FH6 Telemetry</title></svelte:head>

<div class="map-window">
  {#if $settings}
    <MapPanel
      points={pts}
      currentIndex={idx}
      {drawLine}
      {colorByLap}
      {fixedTrace}
      settings={$settings}
    />
  {:else}
    <p class="waiting">Waiting for map data…</p>
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    background: #030712;
    overflow: hidden;
  }
  .map-window {
    width: 100vw;
    height: 100vh;
    display: flex;
    align-items: stretch;
  }
  .map-window :global(.map-host),
  .map-window :global(.track) {
    width: 100% !important;
    height: 100% !important;
    aspect-ratio: unset !important;
    border-radius: 0;
  }
  .waiting {
    color: #6b7280;
    font-family: system-ui, sans-serif;
    font-size: 0.85rem;
    margin: auto;
  }
</style>
