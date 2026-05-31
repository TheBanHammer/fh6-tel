<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { isDesktop } from '$lib/ipc';
  import { startTelemetryListener, replay } from '$lib/stores/telemetry';
  import { loadSettings, settings, saveSettings } from '$lib/stores/sessions';
  import TopBar from '$lib/components/TopBar.svelte';
  import CompassBar from '$lib/components/CompassBar.svelte';
  import CenterPanel from '$lib/components/CenterPanel.svelte';
  import TireWidget from '$lib/components/TireWidget.svelte';
  import LiveTrackMap from '$lib/components/LiveTrackMap.svelte';
  import FloatingPanel from '$lib/components/FloatingPanel.svelte';
  import LapBar from '$lib/components/LapBar.svelte';
  import SessionDrawer from '$lib/components/SessionDrawer.svelte';
  import SessionViewer from '$lib/components/SessionViewer.svelte';
  import ReplayBar from '$lib/components/ReplayBar.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';
  import type { SessionRow } from '$lib/types';

  let showSessions = $state(false);
  let showSettings = $state(false);
  let viewerSession = $state<SessionRow | null>(null);
  let toasts = $state<{ id: number; message: string }[]>([]);
  let nextToastId = 0;
  let pendingUpdate = $state<{ version: string; install: () => Promise<void> } | null>(null);
  let updateInstalling = $state(false);
  let popoutRef: Window | null = null;
  let mapPoppedOut = $state(false);
  let controlBc: BroadcastChannel | null = null;
  let heartbeatTimer: ReturnType<typeof setTimeout> | null = null;

  function onPopoutClosed() {
    mapPoppedOut = false;
    if (heartbeatTimer) { clearTimeout(heartbeatTimer); heartbeatTimer = null; }
  }

  function resetHeartbeat() {
    if (heartbeatTimer) clearTimeout(heartbeatTimer);
    // 4 s without a heartbeat → pop-out must have died without sending popout-closed.
    heartbeatTimer = setTimeout(onPopoutClosed, 4000);
  }

  async function closePopout() {
    onPopoutClosed();
    if (isDesktop) {
      try {
        const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const win = await WebviewWindow.getByLabel('map').catch(console.error);
        await win?.close();
      } catch { /* ignore */ }
    } else {
      popoutRef?.close();
    }
  }

  function addToast(message: string) {
    const id = nextToastId++;
    toasts = [...toasts, { id, message }];
    setTimeout(() => { toasts = toasts.filter(t => t.id !== id); }, 4000);
  }

  async function popOutMap() {
    if (isDesktop) {
      // Tauri: create a new WebviewWindow; focus if already open
      try {
        const { WebviewWindow } = await import('@tauri-apps/api/webviewWindow');
        const existing = await WebviewWindow.getByLabel('map').catch(() => null);
        if (existing) {
          await existing.setFocus();
          return;
        }
        await new WebviewWindow('map', {
          url: '/map',
          title: 'Track Map — FH6 Telemetry',
          width: 500,
          height: 520,
          resizable: true,
          decorations: true,
        });
      } catch (e) {
        console.error('Failed to open map window', e);
      }
    } else {
      // Browser: reuse existing pop-out if still open
      if (popoutRef && !popoutRef.closed) {
        popoutRef.focus();
        return;
      }
      popoutRef = window.open('/map', 'fh6-map', 'width=500,height=520,resizable=yes');
    }
  }

  onDestroy(() => controlBc?.close());

  onMount(async () => {
    await loadSettings();
    await startTelemetryListener({ onError: (m) => addToast(m), onBindFailed: (m) => addToast(m) });
    controlBc = new BroadcastChannel('fh6-tel-map');
    controlBc.onmessage = (e: MessageEvent<{ type: string }>) => {
      if (e.data?.type === 'popout-opened') { mapPoppedOut = true; resetHeartbeat(); }
      if (e.data?.type === 'popout-heartbeat') { resetHeartbeat(); }
      if (e.data?.type === 'popout-closed') { onPopoutClosed(); }
    };
    if (isDesktop) {
      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const info = await invoke<{ version: string; is_deb: boolean } | null>('check_for_update');
        if (info) {
          pendingUpdate = {
            version: info.version,
            install: async () => {
              updateInstalling = true;
              await invoke('install_update', { isDeb: info.is_deb });
            },
          };
        }
      } catch {
        // Offline or update endpoint unreachable — ignore
      }
    }
  });

  let s = $derived($settings);

  // Apply theme to <html> element whenever settings change
  $effect(() => {
    const theme = s?.theme ?? 'dark';
    document.documentElement.setAttribute('data-theme', theme);
  });

  // Replaying takes over the live dashboard — get the overlays out of the way.
  $effect(() => {
    if ($replay.active) {
      showSessions = false;
      viewerSession = null;
    }
  });
</script>

{#if pendingUpdate}
  <div class="update-bar">
    <span>Update v{pendingUpdate.version} available</span>
    <button class="update-install" disabled={updateInstalling} onclick={() => pendingUpdate?.install()}>
      {updateInstalling ? 'Installing…' : 'Install & restart'}
    </button>
    <button class="update-dismiss" onclick={() => (pendingUpdate = null)}>✕</button>
  </div>
{/if}

<div class="dashboard">
  <TopBar
    useMph={s?.useMph ?? true}
    onSettings={() => (showSettings = true)}
    onSessions={() => (showSessions = !showSessions)}
    tiresVisible={s?.tiresVisible ?? true}
    mapEnabled={s?.mapEnabled ?? false}
    {mapPoppedOut}
    onToggleTires={async () => { if (s) await saveSettings({ ...s, tiresVisible: !(s.tiresVisible ?? true) }); }}
    onToggleMap={async () => {
      if (mapPoppedOut) { await closePopout(); return; }
      if (s) await saveSettings({ ...s, mapEnabled: !s.mapEnabled });
    }}
  />
  <CompassBar />

  <div class="main">
    <div class="center-area">
      <CenterPanel useMph={s?.useMph ?? true} />
    </div>
  </div>

  {#if s?.tiresVisible ?? true}
    <FloatingPanel
      id="fh6-tires"
      title="TIRES"
      defaultWidth={200}
      defaultTop={64}
      onClose={async () => { if (s) await saveSettings({ ...s, tiresVisible: false }); }}
    >
      <TireWidget
        tireTempCold={s?.tireTempCold ?? 60}
        tireTempOptimal={s?.tireTempOptimal ?? 85}
        tireTempHot={s?.tireTempHot ?? 110}
      />
    </FloatingPanel>
  {/if}

  {#if s?.mapEnabled}
    <FloatingPanel
      id="fh6-map"
      title="TRACK MAP"
      defaultWidth={200}
      defaultBottom={56}
      resizable
      hidden={mapPoppedOut}
      onClose={async () => { if (s) await saveSettings({ ...s, mapEnabled: false }); }}
    >
      {#snippet actions()}
        <button
          class="popout-btn"
          onclick={popOutMap}
          title="Pop out map"
          aria-label="Pop out map"
        >⤢</button>
      {/snippet}
      <LiveTrackMap />
    </FloatingPanel>
  {/if}

  <div class="lap-bar">
    <LapBar />
  </div>
</div>

{#if showSessions}
  <SessionDrawer
    onClose={() => (showSessions = false)}
    onOpen={(session) => (viewerSession = session)}
  />
{/if}

{#if viewerSession}
  <SessionViewer
    session={viewerSession}
    useMph={s?.useMph ?? true}
    onClose={() => (viewerSession = null)}
  />
{/if}

<ReplayBar />

{#if toasts.length > 0}
  <div class="toast-stack">
    {#each toasts as toast (toast.id)}
      <div class="toast">{toast.message}</div>
    {/each}
  </div>
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
{/if}

<style>
  /* ── Theme: CSS custom properties ───────────────────────────────────────── */
  :global(:root) {
    /* Dark (default) */
    --bg-body:    #030712;
    --bg-panel:   #060c14;
    --bg-card:    #080e18;
    --bg-elevated:#0d1420;
    --bg-track:   #151e2e;
    --bd-dim:     #131d2e;
    --bd-subtle:  #1e2a3a;
    --bd-muted:   #252f42;
    --bd-strong:  #2a3a50;
    --tx-hi:      #f9fafb;
    --tx-mid:     #e5e7eb;
    --tx-lo:      #9ca3af;
    --tx-dim:     #6b7280;
    --tx-xdim:    #4b5563;
    --tx-ghost:   #374151;
    --ac:         #3b82f6;
    --ac-dim:     #1e3a5f;
    --adi-sky:    #0a1628;
    --adi-ground: #1a1008;
  }

  :global([data-theme="cobalt2"]) {
    --bg-body:    #122738;
    --bg-panel:   #163448;
    --bg-card:    #193549;
    --bg-elevated:#1e4060;
    --bg-track:   #1a3b58;
    --bd-dim:     #1f4e6a;
    --bd-subtle:  #235a7a;
    --bd-muted:   #2a6d91;
    --bd-strong:  #337ba0;
    --tx-hi:      #ffffff;
    --tx-mid:     #e1efff;
    --tx-lo:      #9acfdf;
    --tx-dim:     #7eb8d4;
    --tx-xdim:    #5a96b8;
    --tx-ghost:   #3d7a9c;
    --ac:         #ffc600;
    --ac-dim:     #7a5e00;
    --adi-sky:    #0f2d47;
    --adi-ground: #1a2808;
  }

  :global([data-theme="purple"]) {
    --bg-body:    #0e0b1a;
    --bg-panel:   #130e24;
    --bg-card:    #18132e;
    --bg-elevated:#1f1840;
    --bg-track:   #1c1538;
    --bd-dim:     #251c4a;
    --bd-subtle:  #2d2260;
    --bd-muted:   #3a2b78;
    --bd-strong:  #4a3590;
    --tx-hi:      #f5f0ff;
    --tx-mid:     #ddd4ff;
    --tx-lo:      #b8a8e8;
    --tx-dim:     #8b6bb1;
    --tx-xdim:    #6248a0;
    --tx-ghost:   #4a3570;
    --ac:         #c084fc;
    --ac-dim:     #581c87;
    --adi-sky:    #0e0b28;
    --adi-ground: #1a0a2a;
  }

  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: var(--bg-body);
    color: var(--tx-hi);
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    overflow: hidden;
    height: 100vh;
    width: 100vw;
  }

  /* App-wide slim themed scrollbars (WebView2/Chromium + Firefox) */
  :global(*) {
    scrollbar-width: thin;
    scrollbar-color: var(--bd-strong) transparent;
  }
  :global(*::-webkit-scrollbar) { width: 9px; height: 9px; }
  :global(*::-webkit-scrollbar-track) { background: transparent; }
  :global(*::-webkit-scrollbar-thumb) {
    background: var(--bd-strong);
    border-radius: 5px;
    border: 2px solid transparent;
    background-clip: padding-box;
  }
  :global(*::-webkit-scrollbar-thumb:hover) {
    background: var(--tx-ghost);
    background-clip: padding-box;
  }
  :global(*::-webkit-scrollbar-corner) { background: transparent; }

  .dashboard {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .main {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .center-area { background: var(--bg-body); overflow: hidden; width: 100%; height: 100%; }
  .lap-bar { height: clamp(2.5rem, 5.5vh, 4rem); flex-shrink: 0; }

  .update-bar {
    position: fixed; top: 0; left: 0; right: 0; z-index: 300;
    display: flex; align-items: center; gap: 0.75rem;
    padding: 0.35rem 1rem;
    background: var(--ac-dim); border-bottom: 1px solid var(--ac);
    font-size: 0.78rem; color: var(--tx-mid);
  }
  .update-bar span { flex: 1; }
  .update-install {
    background: var(--ac); color: #fff; border: none; border-radius: 4px;
    padding: 0.2rem 0.65rem; font-size: 0.75rem; cursor: pointer;
  }
  .update-install:disabled { opacity: 0.6; cursor: default; }
  .update-dismiss {
    background: none; border: none; color: var(--tx-dim);
    font-size: 0.85rem; cursor: pointer; padding: 0 0.25rem;
  }
  .update-dismiss:hover { color: var(--tx-hi); }

  .toast-stack {
    position: fixed; bottom: 4rem; left: 50%; transform: translateX(-50%);
    display: flex; flex-direction: column; gap: 0.5rem; z-index: 200;
    pointer-events: none;
  }
  .toast {
    background: var(--bg-elevated); border: 1px solid #ef4444; border-radius: 6px;
    color: #fca5a5; font-size: 0.8rem; padding: 0.5rem 1rem;
    max-width: 420px; text-align: center;
  }

  .popout-btn {
    background: none;
    border: none;
    color: var(--tx-xdim);
    font-size: 0.75rem;
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }
  .popout-btn:hover { color: var(--tx-hi); }
</style>
