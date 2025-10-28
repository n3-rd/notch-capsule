<script lang="ts">
	import { Banana, Archive, Settings } from '@lucide/svelte';
	import { activeTab, type TabId } from '$lib/stores/tabs';

	const pills = [
		{
			leftPills: [
				{
					id: 'app' as TabId,
					name: 'App',
					icon: Banana
				},
				{
					id: 'archive' as TabId,
					name: 'Archive',
					icon: Archive
				}
			],
			rightPills: [
				{
					id: 'settings' as TabId,
					name: 'Settings',
					icon: Settings
				}
			]
		}
	];

	function selectTab(tabId: TabId) {
		activeTab.set(tabId);
	}
</script>

<div class="flex h-6 w-full items-center justify-between px-4">
	<div class="flex items-center gap-3">
		{#each pills[0].leftPills as pill}
			<button
				class="pill flex cursor-pointer items-center justify-between rounded-3xl px-4 py-1 text-xs text-white transition-colors"
				class:active={$activeTab === pill.id}
				onclick={() => selectTab(pill.id)}
			>
				<svelte:component this={pill.icon} size={18} />
			</button>
		{/each}
	</div>
	{#each pills[0].rightPills as pill}
		<button
			class="pill flex cursor-pointer items-center justify-between rounded-3xl px-4 py-1 text-xs text-white transition-colors"
			class:active={$activeTab === pill.id}
			onclick={() => selectTab(pill.id)}
		>
			<svelte:component this={pill.icon} size={18} />
		</button>
	{/each}
</div>

<style>
	.pill {
		background: transparent;
		border: none;
	}

	.pill:hover {
		background-color: #202020;
	}

	.pill.active {
		background-color: #2a2a2a;
	}
</style>
