// src/lib/components/welcome/actions.js
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { open as openDialog, save as saveDialog, ask } from '@tauri-apps/plugin-dialog';
import { homeDir, basename, dirname } from '@tauri-apps/api/path';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
import { project as projectStore } from '$lib/stores/projectStore.js';

// --- showWelcomeScreen --- (No changes)
export async function showWelcomeScreen() {
	const welcomeWindow = getCurrentWindow();
	if (!welcomeWindow) { console.error("ACTION_ERROR: showWelcomeScreen could not get the current window handle."); return; }
	try {
		console.log("[WelcomeScreen] Attempting to show and focus window:", welcomeWindow.label);
		const isVisible = await welcomeWindow.isVisible();
		if (!isVisible) { await welcomeWindow.show(); console.log("[WelcomeScreen] Shown window:", welcomeWindow.label); } else { console.log("[WelcomeScreen] Window already visible:", welcomeWindow.label); }
		await welcomeWindow.unminimize();
		await welcomeWindow.setFocus();
		console.log("[WelcomeScreen] Unminimized and focused window:", welcomeWindow.label);
	} catch (error) { console.error("[WelcomeScreen] Error in showWelcomeScreen for window:", welcomeWindow.label, error); }
}

// --- loadProjects --- (No changes)
export async function loadProjects({ setRecentProjects, setStatusMessage, setIsLoading } = {}) {
  if (setIsLoading) setIsLoading(true);
  if (setStatusMessage) setStatusMessage('Loading recent projects...'); else console.log('[ProjectActions] Loading projects (no UI setters)...');
  try {
    const projects = await invoke('load_recent_projects');
    const validProjects = Array.isArray(projects) ? projects : [];
    if (setRecentProjects) setRecentProjects(validProjects);
    if (setStatusMessage) setStatusMessage(`Loaded ${validProjects.length} projects.`);
    console.log(`[ProjectActions] Loaded ${validProjects.length} projects.`);
    return validProjects;
  } catch (error) {
    console.error("[ProjectActions] Failed to load recent projects:", error);
    if (setStatusMessage) setStatusMessage(`Error loading recent projects: ${error?.message || error}`);
    if (setRecentProjects) setRecentProjects([]);
    throw error;
  } finally { if (setIsLoading) setIsLoading(false); }
}


// --- openProjectWindow --- (No changes)
export async function openProjectWindow(project, { setRecentProjects, setStatusMessage, setIsLoading }) {
  if (!project || !project.path || !project.name) { setStatusMessage('Error: Invalid project data provided for opening window.'); console.error('[ProjectWindow] Invalid project data:', project); return; }
  setStatusMessage(`Opening project: ${project.name}...`);
  console.log(`[ProjectWindow] --- Starting openProjectWindow for: ${project.name} (${project.path}) ---`);
  const welcomeWindow = getCurrentWindow();
  if (!welcomeWindow) { console.error("[ProjectWindow] ACTION_ERROR: Could not get welcome window handle."); setStatusMessage("Error: Could not find main application window."); return; }
  console.log("[ProjectWindow] Welcome window handle obtained:", welcomeWindow.label);
  let projectWindowHandle = null;
  let listenerCleanupFunctions = [];
  try {
    console.log(`[ProjectWindow] Invoking 'open_project' for path: ${project.path}`);
    await invoke('open_project', { projectXmlPath: project.path });
    console.log("[ProjectWindow] Backend 'open_project' successful.");
    const safePathForLabel = project.path.replace(/[^a-zA-Z0-9\-_\/:]/g, '_');
    const windowLabel = `project-${safePathForLabel}`;
    console.log(`[ProjectWindow] Generated window label: ${windowLabel}`);
    console.log(`[ProjectWindow] Attempting to create/get window handle with label: ${windowLabel}`);
    projectWindowHandle = new WebviewWindow(windowLabel, { url: `/projectview?xmlPath=${encodeURIComponent(project.path)}`, title: `Harvey`, width: 1000, height: 700, minWidth: 800, minHeight: 600, fullscreen: false, center: true });
    console.log(`[ProjectWindow] Obtained window handle for ${windowLabel}.`);
    const errorListenerCleanup = await projectWindowHandle.once('tauri://error', (e) => {
      console.error(`[ProjectWindow] Tauri window error for ${windowLabel}:`, e.payload);
      setStatusMessage(`Error opening project window: ${e.payload}`);
      showWelcomeScreen().then(() => { loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }); }).catch(showErr => console.error("[ProjectWindow] Error showing welcome screen after project window error:", showErr));
    });
    listenerCleanupFunctions.push(errorListenerCleanup);
    const destroyListenerCleanup = await projectWindowHandle.once('tauri://destroyed', async () => {
        console.log(`[ProjectWindow] Project window ${windowLabel} destroyed. Showing welcome screen and reloading projects...`);
        try {
            await showWelcomeScreen();
            await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading });
            console.log("[ProjectWindow] Welcome screen projects reloaded after project close.");
        } catch (err) { console.error("[ProjectWindow] Error showing welcome screen or reloading projects after project close:", err); await showWelcomeScreen().catch(showErr => console.error("[ProjectWindow] Fallback showWelcomeScreen error:", showErr)); }
        listenerCleanupFunctions.forEach(cleanup => cleanup());
    });
    console.log(`[ProjectWindow] Attached basic listeners for ${windowLabel}.`);
    const readyPromise = new Promise(async (resolve, reject) => {
        let unlistenReady = null;
        const timeout = setTimeout(() => { console.error(`[ProjectWindow] Timeout waiting for 'project-view-ready' from ${windowLabel}`); if (unlistenReady) unlistenReady(); reject(new Error(`Timeout waiting for project view of ${windowLabel} to load.`)); }, 15000);
        unlistenReady = await listen('project-view-ready', (event) => { console.log(`[ProjectWindow] Received 'project-view-ready' for ${windowLabel}.`); clearTimeout(timeout); if (unlistenReady) unlistenReady(); resolve(); });
        listenerCleanupFunctions.push(unlistenReady);
    });
    console.log(`[ProjectWindow] Showing and focusing window: ${windowLabel}`);
    await projectWindowHandle.show();
    await projectWindowHandle.unminimize();
    await projectWindowHandle.setFocus();
    console.log(`[ProjectWindow] Show/Focus successful for ${windowLabel}.`);
    console.log(`[ProjectWindow] Waiting for 'project-view-ready' event from ${windowLabel}...`);
    await readyPromise;
    console.log(`[ProjectWindow] 'project-view-ready' received.`);
    console.log(`[ProjectWindow] Hiding welcome window: ${welcomeWindow.label}`);
    await welcomeWindow.hide();
    console.log(`[ProjectWindow] Welcome window hidden: ${welcomeWindow.label}`);
    console.log(`[ProjectWindow] Maximizing window: ${windowLabel}`);
    await projectWindowHandle.maximize();
    console.log(`[ProjectWindow] Maximize call successful for ${windowLabel}.`);
    console.log("[ProjectWindow] Updating global store and refreshing project list (background)...");
    projectStore.set({ ...project });
    loadProjects({}).catch(err => console.error("[ProjectWindow] Background project list refresh failed:", err));
    console.log("[ProjectWindow] Background updates initiated.");
    setStatusMessage(`Opened project: ${project.name}`);
    console.log(`[ProjectWindow] --- openProjectWindow finished successfully for: ${project.name} ---`);
  } catch (error) {
    console.error(`[ProjectWindow] ACTION_ERROR: Failed during openProjectWindow for ${project?.name}:`, error);
    setStatusMessage(`Error opening project ${project?.name}: ${error?.message || error}`);
    listenerCleanupFunctions.forEach(cleanup => cleanup());
    if (projectWindowHandle) { const win = projectWindowHandle; win.close().catch(closeErr => console.error(`[ProjectWindow] Non-critical error closing window during cleanup: ${closeErr}`)); }
    console.log("[ProjectWindow] Error occurred, ensuring welcome screen is visible and projects reloaded.");
    await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }).catch(loadErr => { console.error("[ProjectWindow] Error reloading projects after main error:", loadErr); if(setStatusMessage) setStatusMessage("Error reloading project list."); });
    await showWelcomeScreen();
  }
}


// --- *** handleCreateProject (MODIFIED) *** ---
export async function handleCreateProject({ setStatusMessage, setRecentProjects, setIsLoading }) {
  let createdProjectFilePath = null;
  let projectName = null;
  let parentLocation = null;
  let desiredProjectPath = null; // Store the initially selected path

  try {
    // Modified defaultPath to open at the user's home directory
    const defaultPath = await homeDir();
    desiredProjectPath = await saveDialog({
      title: 'Create New Project Folder',
      defaultPath: defaultPath,
      // It's good practice to ensure the dialog suggests a folder, not a file.
      // However, the saveDialog in Tauri is typically for saving a file,
      // but we are using its path to create a folder.
      // If the underlying native dialog has an option to pick a folder or ensure
      // the path is treated as a directory, that would be ideal.
      // For now, basename and dirname will be used on the result.
      // If `defaultPath` itself needs to be a file for the dialog to work as expected
      // (e.g., it tries to put "Untitled Harvey Project" *in* the dialog's filename box),
      // we might need to adjust this. Let's test with just homeDir() first.
      // If needed, we could append a generic filename like 'project_location'
      // to `defaultPath` if the dialog requires a file-like path, and then use `dirname`
      // on `desiredProjectPath` before `basename` for the project name.
      // For now, keeping it simple:
    });

    if (!desiredProjectPath) {
      setStatusMessage('Project creation cancelled.');
      return;
    }

    projectName = await basename(desiredProjectPath);
    parentLocation = await dirname(desiredProjectPath);

    if (!projectName || !parentLocation) {
      throw new Error("Could not determine project name or parent location from selected path.");
    }

    setStatusMessage(`Creating project '${projectName}' in ${parentLocation}...`);
    setIsLoading(true);

    // --- Initial create attempt ---
    try {
        createdProjectFilePath = await invoke('create_project', {
            name: projectName,
            parentLocation,
            overwrite: false // First attempt, don't overwrite
        });
        console.log("[ProjectActions] Initial project creation successful.");
    } catch (initialError) {
        // Check if it's the specific directory exists error
        if (initialError?.message?.startsWith("E_DIR_EXISTS:")) {
            console.warn(`[ProjectActions] Directory exists error: ${initialError.message}`);
            const userConfirmation = await ask(
                `A project named "${projectName}" already exists in this location. Do you want to delete the existing project and its contents and create a new one?`,
                { title: 'Confirm Overwrite', type: 'warning', okLabel: 'Overwrite', cancelLabel: 'Cancel' }
            );

            if (userConfirmation) {
                console.log("[ProjectActions] User confirmed overwrite. Attempting creation with overwrite=true.");
                setStatusMessage(`Overwriting existing project '${projectName}'...`);
                // --- Second attempt with overwrite ---
                createdProjectFilePath = await invoke('create_project', {
                    name: projectName,
                    parentLocation,
                    overwrite: true // Set overwrite flag
                });
                console.log("[ProjectActions] Project creation with overwrite successful.");
            } else {
                setStatusMessage('Project creation cancelled (did not overwrite).');
                 setIsLoading(false); // Ensure loading stops if cancelled here
                return; // Exit if user cancels overwrite
            }
        } else {
            // If it's a different error, rethrow it
            throw initialError;
        }
    }

    // --- Proceed to open if successful ---
    setStatusMessage(`Project '${projectName}' created/overwritten successfully! Opening...`);
    console.log("Project ready, path:", createdProjectFilePath);

    if (createdProjectFilePath && projectName) {
        const newProject = { name: projectName, path: createdProjectFilePath };
        console.log("Constructed new project object for opening:", newProject);
        // *** Call openProjectWindow ***
        await openProjectWindow(newProject, { setStatusMessage, setRecentProjects, setIsLoading });
        // openProjectWindow handles setIsLoading internally
    } else {
         // This should ideally not happen if invoke succeeded, but good to check
         throw new Error("Project created/overwritten, but resulting path or name is invalid.");
    }

  } catch (error) {
    // Catch errors from initial create, overwrite create, or opening
    console.error("ACTION_ERROR: Failed to create or open project:", error);
    setStatusMessage(`Error during project creation/opening: ${error?.message?.replace('E_DIR_EXISTS:', '') || error}`); // Clean up error message prefix
    // Ensure loading state is cleared and welcome screen is shown
    setIsLoading(false);
    await showWelcomeScreen().catch(e => console.error("Error showing welcome screen after create error:", e));
    console.log("Reloading projects after error in handleCreateProject.");
    // Reload projects to reflect any potential cleanup done by backend (if overwrite failed partially)
    await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }).catch(e => console.error("Error reloading projects after create error:", e));
  }
  // No finally needed as openProjectWindow/loadProjects handle setIsLoading
}

// --- handleOpenProject --- (No changes)
export async function handleOpenProject({ setStatusMessage, setRecentProjects, setIsLoading }) {
  try {
    const selected = await openDialog({ multiple: false, filters: [{ name: 'Harvey Project Files', extensions: [ 'harvey.xml'] }], title: 'Open Harvey Project File' });
    if (selected && typeof selected === 'string' && !Array.isArray(selected)) {
      const projectXmlPath = selected;
      setStatusMessage(`Importing/Opening project file: ${projectXmlPath}...`);
      setIsLoading(true);
      const project = await invoke('import_project', { projectXmlPath });
      if (project && project.path && project.name) { setStatusMessage(`Project "${project.name}" imported/found. Opening...`); console.log("Project imported/found via dialog:", project); await openProjectWindow(project, { setStatusMessage, setRecentProjects, setIsLoading }); } else { setStatusMessage("Failed to import or retrieve details for the selected project file."); console.error("Import process returned invalid project data:", project); setIsLoading(false); await showWelcomeScreen(); await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }); }
    } else if (Array.isArray(selected)) { setStatusMessage('Cannot open multiple projects at once.'); } else { setStatusMessage('Open project cancelled.'); }
  } catch (error) {
    console.error("Failed to open project XML file:", error);
    setStatusMessage(`Error opening project: ${error?.message || error}`);
    setIsLoading(false);
    await showWelcomeScreen();
    console.log("Reloading projects after error in handleOpenProject.");
    await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading });
  }
}

// --- handleOpenRecent --- (No changes)
export async function handleOpenRecent(project, { setStatusMessage, setRecentProjects, setIsLoading }) {
  console.log("handleOpenRecent called for project:", project ? `${project.name} (${project.path})` : 'undefined');
  if (!project || !project.path || !project.name) { console.error("handleOpenRecent received invalid project data:", project); setStatusMessage("Error: Cannot open invalid project entry."); return; }
  await openProjectWindow(project, { setStatusMessage, setRecentProjects, setIsLoading });
}

// --- handleMenuAction --- (No changes)
export async function handleMenuAction(action, project, { setStatusMessage, setRecentProjects, setOpenMenu, setProjectToRename, setIsRenameModalOpen, setIsLoading }) {
  setOpenMenu(null);
  try {
    console.log(`handleMenuAction: Action='${action}', Project='${project.name}' Path='${project.path}'`);
    switch (action) {
      case 'Open': await openProjectWindow(project, { setStatusMessage, setRecentProjects, setIsLoading }); break;
      case 'Locate': setStatusMessage(`Locating project: ${project.name}...`); setIsLoading(true); try { await invoke('locate_in_finder', { projectXmlPath: project.path }); setStatusMessage(`Located project: ${project.name}`); } catch (locateError) { console.error(`ACTION_ERROR: Failed during locate_in_finder for ${project?.path}:`, locateError); setStatusMessage(`Error locating project: ${locateError?.message || locateError}`); } finally { setIsLoading(false); } break;
      case 'Rename': console.log("Opening rename modal for project:", project); setProjectToRename(project); setIsRenameModalOpen(true); break;
      case 'Remove': { setStatusMessage(`Preparing to remove ${project.name}...`); const confirmationRemove = await ask(`Are you sure you want to remove "${project.name}" from the list? This will not delete the project files.`, { title: 'Confirm Removal', type: 'warning', okLabel: 'Remove', cancelLabel: 'Cancel' }); if (confirmationRemove) { setStatusMessage(`Removing "${project.name}" from list...`); setIsLoading(true); try { await invoke('remove_project_from_list', { projectXmlPath: project.path }); setStatusMessage(`"${project.name}" removed from list.`); } catch(removeError) { console.error(`ACTION_ERROR: Failed during remove_project_from_list for ${project?.path}:`, removeError); setStatusMessage(`Error removing project from list: ${removeError?.message || removeError}`); } finally { await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }); } } else { setStatusMessage('Removal cancelled.'); } break; }
      case 'Delete': { setStatusMessage(`Preparing to delete ${project.name}...`); const confirmationDelete = await ask(`Are you sure you want to permanently delete the project "${project.name}"? This action cannot be undone and will delete the project folder and its contents (or just the XML file if the folder name doesn't match).`, { title: 'Confirm Deletion', type: 'error', okLabel: 'Delete Permanently', cancelLabel: 'Cancel' }); if (confirmationDelete) { setStatusMessage(`Deleting project "${project.name}" from disk...`); setIsLoading(true); try { await invoke('delete_project', { projectXmlPath: project.path }); setStatusMessage(`Project "${project.name}" deleted from disk.`); } catch(deleteError) { console.error(`ACTION_ERROR: Failed during delete_project invoke for ${project?.path}:`, deleteError); setStatusMessage(`Error deleting project: ${deleteError?.message || deleteError}`); } finally { await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }); } } else { setStatusMessage('Delete cancelled.'); } break; }
      default: console.warn("Unknown menu action:", action); setStatusMessage(`Unknown action: ${action}`);
    }
  } catch (error) { console.error(`ACTION_ERROR: Failed to perform action ${action} for project ${project?.path}:`, error); setStatusMessage(`Error performing ${action}: ${error?.message || error}`); setIsLoading(false); }
}

// --- handleRenameConfirm --- (No changes)
export async function handleRenameConfirm(event, { setStatusMessage, setRecentProjects, setIsLoading }) {
  const { projectXmlPath, newName } = event.detail;
  if (!projectXmlPath || !newName || typeof projectXmlPath !== 'string' || typeof newName !== 'string' || newName.trim() === '') { console.error("Rename confirmation missing valid path or new name:", event.detail); setStatusMessage("Error: Could not proceed with rename (invalid data)."); return; }
  const trimmedNewName = newName.trim();
  try {
    setStatusMessage(`Renaming project to "${trimmedNewName}"...`);
    setIsLoading(true);
    await invoke('rename_project', { projectXmlPath, newName: trimmedNewName });
    setStatusMessage(`Project renamed successfully to "${trimmedNewName}"!`);
    await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading });
  } catch (error) { console.error("Failed to rename project:", error); setStatusMessage(`Error renaming project: ${error?.message || error}`); await loadProjects({ setRecentProjects, setStatusMessage, setIsLoading }); }
}

// --- handleRenameCancel --- (No changes)
export function handleRenameCancel({ setStatusMessage }) {
  setStatusMessage('Rename cancelled.');
}