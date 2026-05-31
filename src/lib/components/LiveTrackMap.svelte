<script lang="ts">
  import { packet, replay } from '$lib/stores/telemetry';
  import { settings } from '$lib/stores/sessions';
  import MapPanel from './MapPanel.svelte';
  import type { TelemetryPacket } from '$lib/types';

  let livePoints = $state<TelemetryPacket[]>([]);
  let frame = 0;
  let prevRaceOn = false;
  let bc = $state<BroadcastChannel | null>(null);

  // BroadcastChannel for pop-out window sync
  $effect(() => {
    bc = new BroadcastChannel('fh6-tel-map');
    return () => bc?.close();
  });

  $effect(() => {
    const p = $packet;
    if ($replay.active || !p) return;

    if (p.isRaceOn && !prevRaceOn) {
      livePoints = [];
      frame = 0;
    }
    prevRaceOn = p.isRaceOn;

    if (p.isRaceOn && (p.positionX !== 0 || p.positionZ !== 0)) {
      if (frame % 3 === 0) livePoints = [...livePoints, p];
      frame++;
    }
  });

  let pts = $derived($replay.active ? $replay.packets : livePoints);
  let idx = $derived($replay.active ? $replay.index : livePoints.length - 1);

  let inEvent = $derived(
    $replay.active ||
      (($packet?.currentLap ?? 0) > 0) ||
      (($packet?.lastLap ?? 0) > 0) ||
      (($packet?.lapNumber ?? 0) > 0)
  );

  let lastBroadcast = 0;
  $effect(() => {
    void pts; void idx; void inEvent;
    if (!bc) return;
    const now = Date.now();
    if (now - lastBroadcast < 50) return;
    lastBroadcast = now;
    // $state.snapshot strips Svelte 5 reactive proxies to plain objects,
    // which BroadcastChannel's structured clone can handle.
    bc.postMessage({
      type: 'map-state',
      pts: $state.snapshot(pts),
      idx,
      drawLine: inEvent,
      colorByLap: $replay.active,
      fixedTrace: $replay.active,
    });
  });
</script>

{#if $settings}
  <MapPanel
    points={pts}
    currentIndex={idx}
    drawLine={inEvent}
    colorByLap={$replay.active}
    fixedTrace={$replay.active}
    settings={$settings}
  />
{/if}
