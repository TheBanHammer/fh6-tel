<script lang="ts">
  import { onDestroy } from 'svelte';
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';
  import type { TelemetryPacket, SessionLap } from '$lib/types';
  import { splitLaps, buildLapChips, metricGroups, buildChart, LAP_PALETTE } from '$lib/analysis';

  let {
    packets,
    laps,
    useMph,
  }: {
    packets: TelemetryPacket[];
    laps: SessionLap[];
    useMph: boolean;
  } = $props();

  // Split once; $derived only recomputes if `packets`/`laps` identity changes,
  // never on a chip toggle.
  let chips = $derived(buildLapChips(splitLaps(packets), laps));

  let selectedKeys = $state<Set<string>>(new Set());
  // Colors are assigned at selection time so only laps currently being compared
  // consume palette slots — prevents collisions in sessions with 15+ laps.
  let colorAssignments = $state<Map<string, string>>(new Map());

  // Default selection = best lap (or first chip) once chips are available.
  $effect(() => {
    if (chips.length && selectedKeys.size === 0) {
      const best = chips.find((c) => c.isBest) ?? chips[0];
      selectedKeys = new Set([best.key]);
      colorAssignments = new Map([[best.key, LAP_PALETTE[0]]]);
    }
  });

  let selected = $derived(chips.filter((c) => selectedKeys.has(c.key)));

  // Inject assigned colors before passing to buildChart.
  let coloredSelected = $derived(
    selected.map((chip) => ({ ...chip, color: colorAssignments.get(chip.key) ?? LAP_PALETTE[0] }))
  );

  function toggle(key: string) {
    const nextKeys = new Set(selectedKeys);
    const nextColors = new Map(colorAssignments);
    if (nextKeys.has(key)) {
      if (nextKeys.size === 1) return; // always keep at least one lap selected
      nextKeys.delete(key);
      nextColors.delete(key);
    } else {
      nextKeys.add(key);
      // Pick the first palette color not currently in use; fall back to cycling.
      const used = new Set(nextColors.values());
      const free = LAP_PALETTE.find((c) => !used.has(c))
        ?? LAP_PALETTE[(nextKeys.size - 1) % LAP_PALETTE.length];
      nextColors.set(key, free);
    }
    selectedKeys = nextKeys;
    colorAssignments = nextColors;
  }

  function formatLap(seconds: number) {
    const m = Math.floor(seconds / 60);
    const s = (seconds % 60).toFixed(1).padStart(4, '0');
    return `${m}:${s}`;
  }

  let chartHost = $state<HTMLDivElement | null>(null);
  let plots: uPlot[] = [];

  function destroyPlots() {
    for (const p of plots) p.destroy();
    plots = [];
  }
  onDestroy(destroyPlots);

  function fmtElapsed(s: number) {
    const m = Math.floor(s / 60);
    return `${m}:${(s % 60).toFixed(2).padStart(5, '0')}`;
  }

  // Floating tooltip — table layout: metric rows × lap columns.
  // position:absolute/pointer-events:none means it never causes layout shift.
  function makeTooltip(
    metrics: string[],
    laps: Array<{ label: string; color: string }>,
  ): uPlot.Plugin {
    let el: HTMLDivElement;
    const nm = metrics.length;
    // CSS grid: metric-label col + one value col per lap
    const colTemplate = `max-content ${laps.map(() => 'minmax(38px,auto)').join(' ')}`;

    return {
      hooks: {
        init(u: uPlot) {
          el = document.createElement('div');
          el.className = 'u-tt';
          u.over.appendChild(el);
          u.over.addEventListener('mouseleave', () => { el.style.display = 'none'; });
        },
        setCursor(u: uPlot) {
          const idx = u.cursor.idx;
          if (idx == null) { el.style.display = 'none'; return; }
          const x = (u.data[0] as number[])[idx];
          if (x == null) { el.style.display = 'none'; return; }

          // Header row: blank + one colored lap label per lap
          let grid = `<div></div>`;
          for (const lap of laps) {
            grid += `<div class="u-tt-hdr">
              <span class="u-tt-sw" style="background:${lap.color}"></span>${lap.label}
            </div>`;
          }

          // Data rows: metric label + value per lap
          // Series order from buildChart: for each lap, for each metric
          // → u.data[lapIdx * nm + metricIdx + 1]
          for (let mi = 0; mi < nm; mi++) {
            grid += `<div class="u-tt-metric">${metrics[mi]}</div>`;
            for (let li = 0; li < laps.length; li++) {
              const v = (u.data[li * nm + mi + 1] as (number | null)[])[idx];
              grid += `<div class="u-tt-val">${v != null ? v.toFixed(1) : '—'}</div>`;
            }
          }

          el.innerHTML =
            `<div class="u-tt-time">${fmtElapsed(x)}</div>` +
            `<div class="u-tt-grid" style="grid-template-columns:${colTemplate}">${grid}</div>`;
          el.style.display = 'block';

          const cx = u.cursor.left ?? 0;
          const tw = el.offsetWidth;
          const flip = cx + 16 + tw > u.over.clientWidth;
          el.style.left = flip ? `${cx - tw - 4}px` : `${cx + 12}px`;
          el.style.top = `${Math.max(0, (u.cursor.top ?? 0) - el.offsetHeight / 2)}px`;
        },
      },
    };
  }

  // Rebuild charts when selection / colors / units / host change.
  $effect(() => {
    const host = chartHost;
    const sel = coloredSelected;
    const groups = metricGroups(useMph);
    if (!host || sel.length === 0) {
      destroyPlots();
      return;
    }

    destroyPlots();
    host.innerHTML = '';

    for (const g of groups) {
      const chart = buildChart(g, sel);

      const wrap = document.createElement('div');
      wrap.className = 'chart-block';
      const h = document.createElement('div');
      h.className = 'chart-title';
      h.textContent = g.title;
      wrap.appendChild(h);
      const mount = document.createElement('div');
      wrap.appendChild(mount);
      host.appendChild(wrap);

      const opts: uPlot.Options = {
        width: host.clientWidth || 700,
        height: 180,
        legend: { show: false },
        cursor: { sync: { key: 'replay-analysis' } },
        plugins: [makeTooltip(
          g.metrics.map((m) => m.label),
          sel.map((c) => ({ label: `L${c.lapNumber + 1}`, color: c.color })),
        )],
        scales: { x: { time: false } },
        series: [
          { label: 'Elapsed (s)' },
          ...chart.series.map((s) => ({
            label: s.label,
            stroke: s.stroke,
            dash: s.dash,
            width: 1.25,
          })),
        ],
        axes: [
          { stroke: '#6b7280', grid: { stroke: '#1f2937' } },
          { stroke: '#6b7280', grid: { stroke: '#1f2937' } },
        ],
      };

      const data = [chart.x, ...chart.series.map((s) => s.data)] as uPlot.AlignedData;
      plots.push(new uPlot(opts, data, mount));
    }
  });
</script>

<div class="lap-chips">
  {#each chips as chip (chip.key)}
    <button
      type="button"
      class="chip"
      class:on={selectedKeys.has(chip.key)}
      style:--chip={colorAssignments.get(chip.key) ?? 'var(--bd-muted)'}
      onclick={() => toggle(chip.key)}
    >
      <span class="dot"></span>
      <span class="chip-label">{chip.label}</span>
      <span class="chip-time">{chip.lapTime != null ? formatLap(chip.lapTime) : 'partial'}</span>
      {#if chip.isBest}<span class="chip-best">best</span>{/if}
    </button>
  {/each}
</div>

<div class="charts" bind:this={chartHost}></div>

<style>
  .lap-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-bottom: 1rem;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    background: var(--bg-card);
    border: 1px solid var(--bd-dim);
    border-radius: 999px;
    color: var(--tx-dim);
    font-size: 0.78rem;
    padding: 0.3rem 0.7rem;
    cursor: pointer;
  }
  .chip.on {
    border-color: var(--chip);
    color: var(--tx-hi);
    background: color-mix(in srgb, var(--chip) 16%, transparent);
  }
  .chip .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: var(--chip);
    opacity: 0.4;
  }
  .chip.on .dot {
    opacity: 1;
  }
  .chip-time {
    font-variant-numeric: tabular-nums;
    color: var(--tx-mid);
  }
  .chip-best {
    font-size: 0.62rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #d8b4fe;
  }
  .charts {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  :global(.chart-block) {
    background: var(--bg-card);
    border: 1px solid var(--bd-dim);
    border-radius: 8px;
    padding: 0.6rem 0.75rem 0.75rem;
  }
  :global(.chart-title) {
    color: var(--tx-mid);
    font-size: 0.8rem;
    font-weight: 600;
    margin-bottom: 0.4rem;
  }
  :global(.uplot) {
    background: transparent !important;
  }
  :global(.uplot .u-select) {
    background: color-mix(in srgb, var(--ac) 22%, transparent);
    border: 1px solid var(--ac);
  }
  :global(.u-tt) {
    position: absolute;
    pointer-events: none;
    display: none;
    background: var(--bg-panel);
    border: 1px solid var(--bd-subtle);
    border-radius: 6px;
    padding: 0.35rem 0.55rem;
    font-size: 0.72rem;
    z-index: 10;
    box-shadow: 0 2px 10px rgba(0, 0, 0, 0.45);
  }
  :global(.u-tt-time) {
    color: var(--tx-dim);
    font-variant-numeric: tabular-nums;
    font-size: 0.68rem;
    margin-bottom: 0.3rem;
  }
  :global(.u-tt-grid) {
    display: grid;
    column-gap: 0.55rem;
    row-gap: 0.12rem;
    align-items: center;
  }
  :global(.u-tt-hdr) {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.25rem;
    color: var(--tx-mid);
    font-weight: 600;
    padding-bottom: 0.15rem;
    border-bottom: 1px solid var(--bd-dim);
  }
  :global(.u-tt-sw) {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
  }
  :global(.u-tt-metric) {
    color: var(--tx-mid);
    white-space: nowrap;
    padding-right: 0.3rem;
  }
  :global(.u-tt-val) {
    font-variant-numeric: tabular-nums;
    color: var(--tx-hi);
    font-weight: 600;
    text-align: right;
    white-space: nowrap;
  }
</style>
