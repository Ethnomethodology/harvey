from playwright.sync_api import sync_playwright
import time

def run_cuj(page):
    page.goto("http://localhost:1420")
    page.wait_for_timeout(5000)

    # We can't really do the full live transcription in playwright because it requires
    # the python setup, downloading models, tauri backend etc.
    # We will just verify that the page loads properly.

    page.screenshot(path="/home/jules/verification/screenshots/verification.png")
    page.wait_for_timeout(1000)

if __name__ == "__main__":
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        context = browser.new_context(
            record_video_dir="/home/jules/verification/videos"
        )
        page = context.new_page()
        try:
            run_cuj(page)
        finally:
            context.close()
            browser.close()
