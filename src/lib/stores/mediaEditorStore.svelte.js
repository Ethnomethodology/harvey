
import { browser } from '$app/environment';

const storedEditMode = browser ? localStorage.getItem('isLexicalEditMode') : null;

class MediaEditorStore {
  isMediaEditorOpen = $state(false);
  isLexicalEditMode = $state(storedEditMode === null ? true : storedEditMode === 'true');

  constructor() {
    if (browser) {
      $effect.root(() => {
        $effect(() => {
          localStorage.setItem('isLexicalEditMode', String(this.isLexicalEditMode));
        });
      });
    }
  }
}

export const mediaEditorStore = new MediaEditorStore();
