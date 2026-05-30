import { writable } from 'svelte/store';
import { ipc } from '$lib/ipc';
import type { SessionRow, TelemetryPacket, AppSettings, SessionLap } from '$lib/types';

export const sessions = writable<SessionRow[]>([]);
export const settings = writable<AppSettings | null>(null);

export async function loadSessions() { sessions.set(await ipc.getSessions()); }
export async function loadSessionPackets(sessionId: number): Promise<TelemetryPacket[]> { return ipc.getSessionPackets(sessionId); }
export async function loadSessionLaps(sessionId: number): Promise<SessionLap[]> { return ipc.getSessionLaps(sessionId); }
export async function deleteSession(sessionId: number) { await ipc.deleteSession(sessionId); await loadSessions(); }
export async function clearAllSessions() { await ipc.clearAllSessions(); await loadSessions(); }
export async function renameSession(sessionId: number, name: string | null) { await ipc.renameSession(sessionId, name); await loadSessions(); }
export async function setSessionBookmark(sessionId: number, bookmarked: boolean) { await ipc.setSessionBookmark(sessionId, bookmarked); await loadSessions(); }
export async function loadSettings(): Promise<AppSettings> { const s = await ipc.getSettings(); settings.set(s); return s; }
export async function saveSettings(s: AppSettings) { await ipc.saveSettings(s); settings.set(s); }
