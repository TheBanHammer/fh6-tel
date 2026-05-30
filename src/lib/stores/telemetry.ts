import { writable, derived } from 'svelte/store';
import { ipc } from '$lib/ipc';
import type { TelemetryPacket } from '$lib/types';

export const packet = writable<TelemetryPacket | null>(null);
export const isConnected = writable(false);

// Replay state. When `active`, the whole dashboard renders `packets[index]`
// instead of live telemetry — no dashboard component needs to change because
// they all read `displayPacket`.
export interface ReplayState {
  active: boolean;
  packets: TelemetryPacket[];
  index: number;
  playing: boolean;
  speed: number;
  sessionId: number | null;
  label: string;
}

const emptyReplay: ReplayState = {
  active: false,
  packets: [],
  index: 0,
  playing: false,
  speed: 1,
  sessionId: null,
  label: '',
};

export const replay = writable<ReplayState>({ ...emptyReplay });

export function startReplay(
  sessionId: number,
  label: string,
  packets: TelemetryPacket[]
) {
  replay.set({
    active: true,
    packets,
    index: 0,
    playing: false,
    speed: 1,
    sessionId,
    label,
  });
}

export function exitReplay() {
  replay.set({ ...emptyReplay });
}

// Freezes at last isRaceOn=true packet so pause menu doesn't clear the display
let _frozen: TelemetryPacket | null = null;
export const displayPacket = derived(
  [packet, replay],
  ([$p, $r]): TelemetryPacket | null => {
    if ($r.active && $r.packets.length > 0) {
      const i = Math.min(Math.max($r.index, 0), $r.packets.length - 1);
      return $r.packets[i];
    }
    if ($p !== null && $p.isRaceOn) {
      _frozen = $p;
      return $p;
    }
    return _frozen ?? $p;
  }
);

export const speedMph = derived(displayPacket, ($p) =>
  $p ? $p.speedMs * 2.23694 : 0
);

export const speedKph = derived(displayPacket, ($p) =>
  $p ? $p.speedMs * 3.6 : 0
);

export const rpmPercent = derived(displayPacket, ($p) => {
  if (!$p || $p.engineMaxRpm === 0) return 0;
  return ($p.currentEngineRpm / $p.engineMaxRpm) * 100;
});

let lastPacketTime = 0;
let connectionTimer: ReturnType<typeof setInterval> | null = null;

export async function startTelemetryListener(handlers: { onError?: (msg: string) => void; onBindFailed?: (msg: string) => void } = {}) {
  await ipc.subscribeTelemetry({
    onTick: (payload) => { packet.set(payload); lastPacketTime = Date.now(); isConnected.set(true); },
    onBindFailed: handlers.onBindFailed,
    onError: handlers.onError,
  });

  if (connectionTimer) clearInterval(connectionTimer);
  connectionTimer = setInterval(() => {
    if (Date.now() - lastPacketTime > 2000) {
      isConnected.set(false);
    }
  }, 1000);
}
