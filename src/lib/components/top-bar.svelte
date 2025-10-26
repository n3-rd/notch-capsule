<script lang="ts">
  import { Banana, Archive, Settings } from '@lucide/svelte';
  const pills = [
    {
      leftPills: [
        {
          name: "App",
          icon: Banana,
        },
        {
          name: "Archive",
          icon: Archive,
        }
      ],
      rightPills: [
        {
          name: "Settings",
          icon: Settings,
        }
      ]
    }
  ];
</script>
<div class="w-full h-6 flex justify-between items-center px-4">
  <div class="flex items-center gap-3">
    {#each pills[0].leftPills as pill}
      <div class="pill flex justify-between items-center hover:bg-[#202020] cursor-pointer text-white px-4 py-1 text-xs rounded-3xl ">
        <svelte:component this={pill.icon} size={22} />
      </div>
    {/each}
  </div>
  {#each pills[0].rightPills as pill}
    <div class="pill flex justify-between items-center hover:bg-[#202020] cursor-pointer text-white px-4 py-1 text-xs rounded-3xl ">
      <svelte:component this={pill.icon} size={22} />
    </div>
  {/each}
</div>