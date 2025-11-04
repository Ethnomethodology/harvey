<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import MultiSelect from './MultiSelect.svelte';

	// Define GroupData directly in this file
	interface GroupData {
		id: string;
		project_id: string;
		name: string;
		description?: string | null; // Optional description
	}

	export let fileAssetRelativePath: string;
	export let projectId: string;
	export let allProjectGroups: GroupData[] = [];
	export let initiallyAssignedGroups: GroupData[] = [];
	export let isEditable = true; // New prop

	const dispatch = createEventDispatcher();

	async function handleUpdate({ detail }) {
		const { items } = detail;
		const originalAssignedIds = new Set(initiallyAssignedGroups.map(g => g.id));
		const newAssignedIds = new Set(items.map(g => g.id));

		const added = items.filter(g => !originalAssignedIds.has(g.id));
		const removed = initiallyAssignedGroups.filter(g => !newAssignedIds.has(g.id));

		for (const group of added) {
			try {
				await invoke('add_file_to_existing_group', {
					projectId: projectId,
					groupId: group.id,
					fileAssetRelativePath: fileAssetRelativePath
				});
				dispatch('groupsUpdated', { action: 'added', group });
			} catch (err) {
				console.error('Error adding file to group:', err);
				dispatch('error', `Failed to add group: ${err}`);
			}
		}

		for (const group of removed) {
			try {
				await invoke('remove_file_from_group', {
					projectId: projectId,
					groupId: group.id,
					fileAssetRelativePath: fileAssetRelativePath
				});
				dispatch('groupsUpdated', { action: 'removed', group });
			} catch (err) {
				console.error('Error removing file from group:', err);
				dispatch('error', `Failed to remove group: ${err}`);
			}
		}
	}

	function handleCreateNewGroup() {
		if (!isEditable) return;
		dispatch('createNewGroup');
	}
</script>

<MultiSelect
	allItems={allProjectGroups}
	assignedItems={initiallyAssignedGroups}
	{isEditable}
	displayField="name"
	itemType="group"
	on:update={handleUpdate}
	on:createitem={handleCreateNewGroup}
/>
