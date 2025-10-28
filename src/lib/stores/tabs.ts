import { writable } from 'svelte/store';

export type TabId = 'app' | 'archive' | 'settings';

export const activeTab = writable<TabId>('app');
