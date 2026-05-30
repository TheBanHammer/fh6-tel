import type { TelemetryPacket, SessionLap } from './types';

/** One contiguous run of packets sharing a lapNumber. */
export interface LapSegment {
  lapNumber: number;
  startIndex: number;
  packets: TelemetryPacket[];
}

/** Selector-chip metadata derived from segments + recorded lap times. */
export interface LapChip {
  key: string;          // stable unique id, e.g. "2-1"
  lapNumber: number;    // raw, 0-based
  label: string;        // "Lap 3" or "Lap 3 (2)" for rewind duplicates
  lapTime: number | null; // recorded lap seconds, null = partial/no time
  isBest: boolean;
  color: string;
  segment: LapSegment;
}

export interface MetricDef {
  label: string;
  dash: number[];                       // uPlot dash pattern; [] = solid
  value: (p: TelemetryPacket) => number;
}

export interface MetricGroup {
  title: string;
  metrics: MetricDef[];
}

export interface ChartSeries {
  label: string;
  stroke: string;
  dash: number[];
  data: (number | null)[];
}

export interface AlignedChart {
  x: number[];
  series: ChartSeries[];
}

/** Forza Data Out telemetry streams at a fixed 60 packets per second. */
export const TELEMETRY_HZ = 60;

/** Colors assigned to selected laps in AnalysisTab (cycled when all are in use). */
export const LAP_PALETTE = [
  '#3b82f6', '#f59e0b', '#22c55e', '#a855f7', '#ef4444', '#06b6d4',
];

/** Minimum currentLap seconds before a reset to ~0 counts as a genuine lap
 *  crossing (matches the backend's MIN_LAP_SECS in session.rs). */
const MIN_LAP_SECS = 20.0;

/** Group packets into per-lap segments by detecting currentLap resets.
 *
 *  The Forza packet's `lapNumber` field is a free-running race counter that
 *  never resets per session and is contaminated by grace-period runs that
 *  emit lapNumber=0. The correct boundary signal is currentLap (the live lap
 *  clock, byte 304) dropping from >MIN_LAP_SECS back to <1 s while racing —
 *  exactly how the Rust backend detects lap completions in session.rs.
 *
 *  Each segment's `lapNumber` is its sequential position (0, 1, 2 …), which
 *  matches session_laps.lap_number from the DB so time lookup is direct. */
export function splitLaps(packets: TelemetryPacket[]): LapSegment[] {
  const segments: LapSegment[] = [];
  let current: LapSegment | null = null;
  let prevCurrentLap = 0.0;

  for (let i = 0; i < packets.length; i++) {
    const p = packets[i];

    if (!current) {
      current = { lapNumber: 0, startIndex: i, packets: [] };
      segments.push(current);
    } else if (p.isRaceOn && prevCurrentLap > MIN_LAP_SECS && p.currentLap < 1.0) {
      current = { lapNumber: segments.length, startIndex: i, packets: [] };
      segments.push(current);
    }

    current.packets.push(p);

    // Only advance prevCurrentLap during racing; grace-period packets
    // (isRaceOn=false) hold currentLap frozen at its last value, so skipping
    // them here prevents false resets when racing resumes after a pause.
    if (p.isRaceOn) prevCurrentLap = p.currentLap;
  }

  return segments;
}

/** Build selector-chip metadata. Color is left empty — AnalysisTab assigns colors
 *  dynamically at selection time so only visible laps consume palette slots.
 *  The recorded lap time maps to the FIRST occurrence of a lapNumber;
 *  rewind-duplicate runs are labelled "(n)" and treated as partial. */
export function buildLapChips(segments: LapSegment[], laps: SessionLap[]): LapChip[] {
  const timeByLap = new Map<number, number>();
  for (const l of laps) if (l.lapTime > 0) timeByLap.set(l.lapNumber, l.lapTime);

  let bestNumber = -1;
  let bestTime = Infinity;
  for (const l of laps) {
    if (l.lapTime > 0 && l.lapTime < bestTime) {
      bestTime = l.lapTime;
      bestNumber = l.lapNumber;
    }
  }

  const occurrences = new Map<number, number>();
  return segments.map((segment) => {
    const occ = (occurrences.get(segment.lapNumber) ?? 0) + 1;
    occurrences.set(segment.lapNumber, occ);
    const lapTime = occ === 1 ? timeByLap.get(segment.lapNumber) ?? null : null;
    return {
      key: `${segment.lapNumber}-${occ}`,
      lapNumber: segment.lapNumber,
      label: `Lap ${segment.lapNumber + 1}${occ > 1 ? ` (${occ})` : ''}`,
      lapTime,
      isBest: occ === 1 && segment.lapNumber === bestNumber,
      color: '',
      segment,
    };
  });
}

/** The four chart groups. Speed accessor/label switch on the units setting,
 *  matching the previous SessionViewer behavior. Dash patterns separate metrics
 *  within a group: solid, dashed, dotted, dash-dot. */
export function metricGroups(useMph: boolean): MetricGroup[] {
  const speedFactor = useMph ? 2.23694 : 3.6;
  const speedLabel = useMph ? 'Speed (mph)' : 'Speed (kph)';
  return [
    {
      title: 'Driver Inputs',
      metrics: [
        { label: 'Throttle %', dash: [], value: (p) => (p.throttle / 255) * 100 },
        { label: 'Brake %', dash: [6, 4], value: (p) => (p.brake / 255) * 100 },
        { label: 'Clutch %', dash: [2, 3], value: (p) => (p.clutch / 255) * 100 },
      ],
    },
    {
      title: 'Speed & Engine',
      metrics: [
        { label: speedLabel, dash: [], value: (p) => p.speedMs * speedFactor },
        {
          label: 'RPM %',
          dash: [6, 4],
          value: (p) => (p.engineMaxRpm > 0 ? (p.currentEngineRpm / p.engineMaxRpm) * 100 : 0),
        },
      ],
    },
    {
      title: 'G-Forces',
      metrics: [
        { label: 'Lateral g', dash: [], value: (p) => p.accelX / 9.80665 },
        { label: 'Longitudinal g', dash: [6, 4], value: (p) => p.accelZ / 9.80665 },
      ],
    },
    {
      title: 'Tire Temps (°C)',
      metrics: [
        { label: 'FL', dash: [], value: (p) => p.tireTempFl },
        { label: 'FR', dash: [6, 4], value: (p) => p.tireTempFr },
        { label: 'RL', dash: [2, 3], value: (p) => p.tireTempRl },
        { label: 'RR', dash: [8, 3, 2, 3], value: (p) => p.tireTempRr },
      ],
    },
  ];
}

/** Build one chart's aligned data for the selected laps. x is time-into-lap in
 *  seconds (sample index / 60) up to the longest selected lap; each lap's series
 *  is padded with null past its own end so uPlot draws a clean gap. */
export function buildChart(group: MetricGroup, selected: LapChip[]): AlignedChart {
  const maxLen = selected.reduce((m, c) => Math.max(m, c.segment.packets.length), 0);
  const x = Array.from({ length: maxLen }, (_, i) => i / TELEMETRY_HZ);
  const series: ChartSeries[] = [];
  for (const chip of selected) {
    const pkts = chip.segment.packets;
    for (const metric of group.metrics) {
      const data: (number | null)[] = new Array(maxLen).fill(null);
      for (let i = 0; i < pkts.length; i++) data[i] = metric.value(pkts[i]);
      series.push({
        label: `L${chip.lapNumber + 1} ${metric.label}`,
        stroke: chip.color,
        dash: metric.dash,
        data,
      });
    }
  }
  return { x, series };
}
