import { getConfig } from './config';

// These are loaded from config at runtime, but provide defaults for static imports
export const notchExpandedWidth = getConfig().dimensions.expanded_width.value;
export const notchExpandedHeight = getConfig().dimensions.expanded_height.value;

// Global dev variable to keep notch expanded during development
export const DEV_KEEP_NOTCH_EXPANDED = false;

export { activeTab, type TabId } from './stores/tabs';
export { default as AppView } from './components/views/app-view.svelte';
export { default as ArchiveView } from './components/views/archive-view.svelte';
export { default as SettingsView } from './components/views/settings-view.svelte';
