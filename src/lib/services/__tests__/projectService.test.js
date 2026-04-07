// src/lib/services/__tests__/projectService.test.js
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { project, initialState as actualInitialState } from '$lib/stores/projectStore.js';
import { loadProjectDataAndUpdateStore } from '../projectService.js';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn()
}));

vi.mock('@tauri-apps/api/event', () => ({
  emit: vi.fn(() => Promise.resolve()), // Ensure emit returns a promise
  listen: vi.fn(() => Promise.resolve(() => {})) // listen returns a promise that resolves to an unlisten function
}));

// Helper to reset store to a known initial state for testing
const getInitialProjectState = () => ({
  ...actualInitialState, // Spread the actual initial state from projectStore.js
  id: null, // Explicitly set/override 'id' for our tests
  // Ensure other relevant fields that might be touched are reset or explicitly set
  isLoading: true, // As per projectStore.js initialState
  error: null,
  statusMessage: 'Initializing...', // As per projectStore.js initialState
  name: null, // Explicitly nullify fields that will be set by loadedData
  xmlPath: null,
  baseDirectory: null,
  files: [],
  documentFiles: [],
  tableFiles: [],
  imageFiles: [],
  standaloneTranscriptFiles: [],
  documentMetadataFiles: []
});

describe('projectService - loadProjectDataAndUpdateStore', () => {
  beforeEach(() => {
    // Reset the project store to its initial state before each test
    project.set(getInitialProjectState());
    // Reset mocks
    invoke.mockReset();
    emit.mockReset();
    // listen.mockReset(); // listen is mocked above to always return a resolving promise
    // Re-apply the specific mock implementation for listen if it was more complex
    vi.mocked(listen).mockImplementation(() => Promise.resolve(() => {}));
  });

  it('should update project store with project_uuid as id from backend', async () => {
    const mockProjectUuid = 'test-uuid-12345';
    const mockBackendPayload = {
      project_name: 'Test Project Name',
      project_xml_path: '/fake/project.xml',
      base_directory: '/fake/base',
      project_uuid: mockProjectUuid,
      files: [
        {
          name: 'file1.mp4',
          path: '/fake/base/harvey_files/Media/file1/media/file1.mp4',
          relative_path: 'harvey_files/Media/file1/media/file1.mp4',
          file_type: 'media',
          is_directory: false,
          children: []
        }
      ],
      document_files: [],
      table_files: [],
      image_files: [],
      standalone_transcript_files: [],
      document_metadata_files: []
      // No need for other_fields if they are not directly set by dataToSet in the function
    };

    vi.mocked(invoke).mockResolvedValue(mockBackendPayload);
    // emit is already mocked to return a resolved promise

    await loadProjectDataAndUpdateStore('/fake/project.xml');

    const updatedProjectState = get(project);
    expect(invoke).toHaveBeenCalledWith('load_project_data', {
      projectXmlPath: '/fake/project.xml'
    });
    expect(updatedProjectState.id).toBe(mockProjectUuid);
    expect(updatedProjectState.name).toBe(mockBackendPayload.project_name);
    expect(updatedProjectState.xmlPath).toBe(mockBackendPayload.project_xml_path);
    expect(updatedProjectState.baseDirectory).toBe(mockBackendPayload.base_directory);
    expect(updatedProjectState.files).toEqual(mockBackendPayload.files); // or .length if content is complex
    expect(updatedProjectState.isLoading).toBe(false);
    expect(updatedProjectState.error).toBeNull();
    expect(updatedProjectState.statusMessage).toBe(
      `Loaded project: ${mockBackendPayload.project_name}`
    );
    expect(emit).toHaveBeenCalledWith('project-view-ready', {
      projectXmlPath: '/fake/project.xml'
    });
  });

  it('should handle errors from backend correctly', async () => {
    const errorMessage = 'Backend failed to load';
    vi.mocked(invoke).mockRejectedValue({ message: errorMessage });

    // Expect the function to throw the error so the caller can catch it
    await expect(loadProjectDataAndUpdateStore('/fake/project.xml')).rejects.toMatchObject({
      message: errorMessage
    });

    const updatedProjectState = get(project);
    expect(invoke).toHaveBeenCalledWith('load_project_data', {
      projectXmlPath: '/fake/project.xml'
    });
    expect(updatedProjectState.isLoading).toBe(false);
    expect(updatedProjectState.error).toBe(errorMessage); // Error message from backend
    expect(updatedProjectState.statusMessage).toBe('Error loading project.'); // Status message set by the function
    expect(updatedProjectState.id).toBeNull(); // Ensure id is not set on error
    // emit should not have been called
    expect(emit).not.toHaveBeenCalled();
  });

  it('should throw an error if projectXmlPath is missing', async () => {
    await expect(loadProjectDataAndUpdateStore('')).rejects.toThrow('projectXmlPath is required');

    const updatedProjectState = get(project);
    expect(updatedProjectState.isLoading).toBe(false); // Set by the error handling within the function
    expect(updatedProjectState.error).toBe('Project path is missing.');
    expect(updatedProjectState.statusMessage).toBe('Error: Project path is missing.');
    expect(invoke).not.toHaveBeenCalled();
    expect(emit).not.toHaveBeenCalled();
  });
});
