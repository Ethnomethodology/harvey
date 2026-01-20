// src/lib/components/welcome/actions.js
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog, save as saveDialog, ask } from '@tauri-apps/plugin-dialog';
import { homeDir, basename, dirname } from '@tauri-apps/api/path';
import { project as projectStore } from '$lib/stores/projectStore.js';
import { goto } from '$app/navigation';

export async function showWelcomeScreen() {
	console.log("[WelcomeScreen] Navigating to home.");
	await goto('/');
}

// --- loadProjects --- (No changes)
export async function loadProjects({ setRecentProjects, setIsLoading } = {}) {
  if (setIsLoading) setIsLoading(true);
  console.log('[ProjectActions] Loading projects...');
  try {
    const projects = await invoke('load_recent_projects');
    const validProjects = Array.isArray(projects) ? projects : [];
    if (setRecentProjects) setRecentProjects(validProjects);
    console.log(`[ProjectActions] Loaded ${validProjects.length} projects.`);
    return validProjects;
  } catch (error) {
    console.error("[ProjectActions] Failed to load recent projects:", error);
    if (setRecentProjects) setRecentProjects([]);
    throw error;
  } finally { if (setIsLoading) setIsLoading(false); }
}


export async function openProjectWindow(project) {
	if (!project || !project.path || !project.name) {
		console.error('[ProjectWindow] Invalid project data:', project);
		return;
	}
	console.log(`Opening project: ${project.name}...`);
	try {
		await invoke('open_project', { projectXmlPath: project.path });
		projectStore.update((current) => ({ ...current, ...project }));
		await goto(`/projectview?xmlPath=${encodeURIComponent(project.path)}`);
		console.log(`Opened project: ${project.name}`);
	} catch (error) {
		console.error(`[ProjectWindow] ACTION_ERROR: Failed during openProjectWindow for ${project?.name}:`, error);
		await showWelcomeScreen();
	}
}


// --- *** handleCreateProject (MODIFIED) *** ---
export async function handleCreateProject({ setRecentProjects, setIsLoading }) {
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
      console.log('Project creation cancelled.');
      return;
    }

    projectName = await basename(desiredProjectPath);
    parentLocation = await dirname(desiredProjectPath);

    if (!projectName || !parentLocation) {
      throw new Error("Could not determine project name or parent location from selected path.");
    }

    console.log(`Creating project '${projectName}' in ${parentLocation}...`);
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
                console.log(`Overwriting existing project '${projectName}'...`);
                // --- Second attempt with overwrite ---
                createdProjectFilePath = await invoke('create_project', {
                    name: projectName,
                    parentLocation,
                    overwrite: true // Set overwrite flag
                });
                console.log("[ProjectActions] Project creation with overwrite successful.");
            } else {
                console.log('Project creation cancelled (did not overwrite).');
                 setIsLoading(false); // Ensure loading stops if cancelled here
                return; // Exit if user cancels overwrite
            }
        } else {
            // If it's a different error, rethrow it
            throw initialError;
        }
    }

    // --- Proceed to open if successful ---
    console.log(`Project '${projectName}' created/overwritten successfully! Opening...`);
    console.log("Project ready, path:", createdProjectFilePath);

    if (createdProjectFilePath && projectName) {
        const newProject = { name: projectName, path: createdProjectFilePath };
        console.log("Constructed new project object for opening:", newProject);
        // *** Call openProjectWindow ***
        await openProjectWindow(newProject);
        // openProjectWindow handles setIsLoading internally
    } else {
         // This should ideally not happen if invoke succeeded, but good to check
         throw new Error("Project created/overwritten, but resulting path or name is invalid.");
    }

  } catch (error) {
    // Catch errors from initial create, overwrite create, or opening
    console.error("ACTION_ERROR: Failed to create or open project:", error);
    // Ensure loading state is cleared and welcome screen is shown
    setIsLoading(false);
    await showWelcomeScreen().catch(e => console.error("Error showing welcome screen after create error:", e));
    console.log("Reloading projects after error in handleCreateProject.");
    // Reload projects to reflect any potential cleanup done by backend (if overwrite failed partially)
    await loadProjects({ setRecentProjects, setIsLoading }).catch(e => console.error("Error reloading projects after create error:", e));
  }
  // No finally needed as openProjectWindow/loadProjects handle setIsLoading
}

// --- handleOpenProject --- (No changes)
export async function handleOpenProject({ setRecentProjects, setIsLoading }) {
  try {
    const selected = await openDialog({ multiple: false, filters: [{ name: 'Harvey Project Files', extensions: [ 'harvey.xml'] }], title: 'Open Harvey Project File' });
    if (selected && typeof selected === 'string' && !Array.isArray(selected)) {
      const projectXmlPath = selected;
      console.log(`Importing/Opening project file: ${projectXmlPath}...`);
      setIsLoading(true);
      const project = await invoke('import_project', { projectXmlPath });
      if (project && project.path && project.name) { console.log(`Project "${project.name}" imported/found. Opening...`); console.log("Project imported/found via dialog:", project); await openProjectWindow(project); } else { console.error("Import process returned invalid project data:", project); setIsLoading(false); await showWelcomeScreen(); await loadProjects({ setRecentProjects, setIsLoading }); }
    } else if (Array.isArray(selected)) { console.log('Cannot open multiple projects at once.'); } else { console.log('Open project cancelled.'); }
  } catch (error) {
    console.error("Failed to open project XML file:", error);
    setIsLoading(false);
    await showWelcomeScreen();
    console.log("Reloading projects after error in handleOpenProject.");
    await loadProjects({ setRecentProjects, setIsLoading });
  }
}

// --- handleOpenRecent --- (No changes)
export async function handleOpenRecent(project) {
  console.log("handleOpenRecent called for project:", project ? `${project.name} (${project.path})` : 'undefined');
  if (!project || !project.path || !project.name) { console.error("handleOpenRecent received invalid project data:", project); return; }
  await openProjectWindow(project);
}

// --- handleMenuAction --- (No changes)
export async function handleMenuAction(action, project, { setRecentProjects, setOpenMenu, setProjectToRename, setIsRenameModalOpen, setIsLoading }) {
  setOpenMenu(null);
  try {
    console.log(`handleMenuAction: Action='${action}', Project='${project.name}' Path='${project.path}'`);
    switch (action) {
      case 'Open': await openProjectWindow(project); break;
      case 'Locate': console.log(`Locating project: ${project.name}...`); setIsLoading(true); try { await invoke('locate_in_finder', { projectXmlPath: project.path }); console.log(`Located project: ${project.name}`); } catch (locateError) { console.error(`ACTION_ERROR: Failed during locate_in_finder for ${project?.path}:`, locateError); } finally { setIsLoading(false); } break;
      case 'Rename': console.log("Opening rename modal for project:", project); setProjectToRename(project); setIsRenameModalOpen(true); break;
      case 'Remove': { console.log(`Preparing to remove ${project.name}...`); const confirmationRemove = await ask(`Are you sure you want to remove "${project.name}" from the list? This will not delete the project files.`, { title: 'Confirm Removal', type: 'warning', okLabel: 'Remove', cancelLabel: 'Cancel' }); if (confirmationRemove) { console.log(`Removing "${project.name}" from list...`); setIsLoading(true); try { await invoke('remove_project_from_list', { projectXmlPath: project.path }); console.log(`"${project.name}" removed from list.`); } catch(removeError) { console.error(`ACTION_ERROR: Failed during remove_project_from_list for ${project?.path}:`, removeError); } finally { await loadProjects({ setRecentProjects, setIsLoading }); } } else { console.log('Removal cancelled.'); } break; }
      case 'Delete': { console.log(`Preparing to delete ${project.name}...`); const confirmationDelete = await ask(`Are you sure you want to permanently delete the project "${project.name}"? This action cannot be undone and will delete the project folder and its contents (or just the XML file if the folder name doesn't match).`, { title: 'Confirm Deletion', type: 'error', okLabel: 'Delete Permanently', cancelLabel: 'Cancel' }); if (confirmationDelete) { console.log(`Deleting project "${project.name}" from disk...`); setIsLoading(true); try { await invoke('delete_project', { projectXmlPath: project.path }); console.log(`Project "${project.name}" deleted from disk.`); } catch(deleteError) { console.error(`ACTION_ERROR: Failed during delete_project invoke for ${project?.path}:`, deleteError); } finally { await loadProjects({ setRecentProjects, setIsLoading }); } } else { console.log('Delete cancelled.'); } break; }
      default: console.warn("Unknown menu action:", action);
    }
  } catch (error) { console.error(`ACTION_ERROR: Failed to perform action ${action} for project ${project?.path}:`, error); setIsLoading(false); }
}

// --- handleRenameConfirm --- (No changes)
export async function handleRenameConfirm(event, { setRecentProjects, setIsLoading }) {
  const { projectXmlPath, newName } = event.detail;
  if (!projectXmlPath || !newName || typeof projectXmlPath !== 'string' || typeof newName !== 'string' || newName.trim() === '') { console.error("Rename confirmation missing valid path or new name:", event.detail); return; }
  const trimmedNewName = newName.trim();
  try {
    console.log(`Renaming project to "${trimmedNewName}"...`);
    setIsLoading(true);
    await invoke('rename_project', { projectXmlPath, newName: trimmedNewName });
    console.log(`Project renamed successfully to "${trimmedNewName}"!`);
    await loadProjects({ setRecentProjects, setIsLoading });
  } catch (error) { console.error("Failed to rename project:", error); await loadProjects({ setRecentProjects, setIsLoading }); }
}

// --- handleRenameCancel --- (No changes)
export function handleRenameCancel() {
  console.log('Rename cancelled.');
}