export const notchExpandedWidth = 700;
export const notchExpandedHeight = 200;

// Global dev variable to keep notch expanded during development
export const DEV_KEEP_NOTCH_EXPANDED = false;

export { activeTab, type TabId } from './stores/tabs';
export { default as AppView } from './components/views/app-view.svelte';
export { default as ArchiveView } from './components/views/archive-view.svelte';
export { default as SettingsView } from './components/views/settings-view.svelte';