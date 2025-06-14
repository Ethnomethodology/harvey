// src/lib/stores/notificationStore.js
import { writable } from 'svelte/store';
import { v4 as uuidv4 } from 'uuid';

const NOTIFICATION_TIMEOUT_MS = 5000; // Default timeout

/**
 * @typedef {Object} Notification
 * @property {string} id - Unique identifier
 * @property {'success' | 'error' | 'info' | 'warning'} type - Notification type
 * @property {string} message - The notification message
 * @property {number} [duration] - Optional duration in ms. If 0 or undefined, notification is persistent.
 * @property {number} timestamp - Timestamp of when the notification was added
 */

/** @type {import('svelte/store').Writable<Notification[]>} */
const notifications = writable([]);

/**
 * Adds a new notification to the store.
 * @param {string} message - The message to display.
 * @param {'success' | 'error' | 'info' | 'warning'} type - The type of notification.
 * @param {number} [duration=NOTIFICATION_TIMEOUT_MS] - Optional duration in ms. 0 for persistent.
 */
function addNotification(message, type, duration = NOTIFICATION_TIMEOUT_MS) {
    const id = uuidv4();
    const newNotification = {
        id,
        message,
        type,
        duration,
        timestamp: Date.now()
    };

    notifications.update(currentNotifications => {
        // Add new notification to the top (or bottom, depending on desired visual stacking)
        // For bottom-right stacking where new ones appear above old ones, add to end of array.
        return [...currentNotifications, newNotification];
    });

    if (typeof duration === 'number' && duration > 0) {
        console.log(`[notificationStore] Setting auto-dismiss for notification ID ${id} with duration: ${duration}`);
        setTimeout(() => {
            dismissNotification(id);
        }, duration);
    }
}

/**
 * Removes a notification from the store by its ID.
 * @param {string} id - The ID of the notification to remove.
 */
function dismissNotification(id) {
    notifications.update(currentNotifications =>
        currentNotifications.filter(n => n.id !== id)
    );
}

/**
 * Clears all notifications.
 */
function clearAllNotifications() {
    notifications.set([]);
}

export default {
    subscribe: notifications.subscribe, // Expose the store's subscribe method
    add: addNotification,
    dismiss: dismissNotification,
    clearAll: clearAllNotifications
};
