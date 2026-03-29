/**
 * A global queue to manage concurrent PDF rendering tasks.
 * Prevents multiple pdf.js workers from overwhelming the browser/UI thread.
 */
class PdfThumbnailQueue {
    constructor(maxConcurrency = 2) {
        this.maxConcurrency = maxConcurrency;
        this.running = 0;
        this.queue = [];
    }

    /**
     * @param {Function} task - A function that returns a Promise
     * @returns {Promise}
     */
    add(task) {
        return new Promise((resolve, reject) => {
            this.queue.push({ task, resolve, reject });
            this.next();
        });
    }

    async next() {
        if (this.running >= this.maxConcurrency || this.queue.length === 0) {
            return;
        }

        this.running++;
        const { task, resolve, reject } = this.queue.shift();

        try {
            const result = await task();
            resolve(result);
        } catch (error) {
            reject(error);
        } finally {
            this.running--;
            this.next();
        }
    }
}

export const pdfThumbnailQueue = new PdfThumbnailQueue(2);
