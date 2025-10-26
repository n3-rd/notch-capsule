<script lang="ts">
	import TopBar from './components/top-bar.svelte';
	import AppView from './components/views/app-view.svelte';
	import ArchiveView from './components/views/archive-view.svelte';
	import SettingsView from './components/views/settings-view.svelte';
	import { activeTab } from './stores/tabs';
</script>

<div class="notch-container">
	<TopBar />
	
	<div class="view-wrapper">
		{#if $activeTab === 'app'}
			<AppView />
		{:else if $activeTab === 'archive'}
			<ArchiveView />
		{:else if $activeTab === 'settings'}
			<SettingsView />
		{/if}
	</div>
</div>

<style>
	:global(body) {
		font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
	}

	.notch-container {
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
	}

	.view-wrapper {
		flex: 1;
		overflow: auto;
		min-height: 0;
	}
</style>