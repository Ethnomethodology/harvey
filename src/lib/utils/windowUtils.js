import { getCurrentWindow, LogicalSize, currentMonitor } from '@tauri-apps/api/window';
import { WELCOME_WIDTH, WELCOME_HEIGHT, DEFAULT_MIN_HEIGHT } from '$lib/constants/windowSize.js';

/**
 * Resizes the application window to the "Welcome Screen" dimensions,
 * but ensures the requested height doesn't exceed the monitor's available vertical space.
 * This prevents clipping on Windows when the screen is smaller than the requested 800px height.
 */
export async function resizeToSafeWelcomeSize() {
  try {
    const appWindow = getCurrentWindow();
    const monitor = await currentMonitor();
    
    let targetWidth = WELCOME_WIDTH;
    let targetHeight = WELCOME_HEIGHT;
    
    if (monitor) {
      const scaleFactor = monitor.scaleFactor;
      // Calculate logical dimensions (Physical / ScaleFactor)
      const logicalHeight = monitor.size.height / scaleFactor;
      const logicalWidth = monitor.size.width / scaleFactor;
      
      // Use a buffer (e.g., 100px) to account for taskbars on Windows or menubars/docks
      // Ensure we don't shrink below the default minimum height
      targetHeight = Math.max(DEFAULT_MIN_HEIGHT, Math.min(WELCOME_HEIGHT, Math.floor(logicalHeight - 100)));
      
      // Ensure width fits as well (though 1024 is usually safe)
      targetWidth = Math.min(WELCOME_WIDTH, Math.floor(logicalWidth - 40));
      
      console.log(`[WindowUtils] Calculated safe welcome size: ${targetWidth}x${targetHeight} (Monitor logical size: ${Math.floor(logicalWidth)}x${Math.floor(logicalHeight)})`);
    } else {
      console.warn('[WindowUtils] Could not detect monitor state, falling back to default welcome size constants.');
    }

    // Apply the min size first to avoid conflicts
    await appWindow.setMinSize(new LogicalSize(targetWidth, DEFAULT_MIN_HEIGHT));
    // Set the actual window size
    await appWindow.setSize(new LogicalSize(targetWidth, targetHeight));
    // Center the window on the current monitor
    await appWindow.center();
  } catch (err) {
    console.error('[WindowUtils] Failed to perform safe window resize:', err);
  }
}
