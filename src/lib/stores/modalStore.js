// src/lib/stores/modalStore.js
import { writable } from 'svelte/store';

export const modalStore = writable({
  isHeaderConfirmationDialogOpen: false,
  headerConfirmationData: null,
  onConfirm: null
});

export function showHeaderConfirmationModal(tablePath, previewData, onConfirm) {
  modalStore.set({
    isHeaderConfirmationDialogOpen: true,
    headerConfirmationData: { tablePath, previewData },
    onConfirm
  });
}

export function hideHeaderConfirmationModal() {
  modalStore.set({
    isHeaderConfirmationDialogOpen: false,
    headerConfirmationData: null,
    onConfirm: null
  });
}
