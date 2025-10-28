
from playwright.sync_api import sync_playwright
import time

def run_verification():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        try:
            # Navigate to the app
            page.goto("http://localhost:1420")
            time.sleep(2) # Wait for app to load

            # Create a project
            page.click('button:has-text("Create Project")')
            page.fill('input[type="text"]', 'Test Project')
            page.click('button:has-text("Create")')
            time.sleep(2) # Wait for project to be created

            # Open the layout settings modal
            page.click('button[title="View Settings"]')
            time.sleep(1)

            # Change waveform orientation to Vertical
            page.locator('.ui-select:has-text("Horizontal")').click()
            page.click('button:has-text("Vertical")')

            # Enable dual transcript mode
            page.locator('.ui-select:has-text("Disable")').click()
            page.click('button:has-text("Enable")')

            # Take a screenshot
            page.screenshot(path="jules-scratch/verification/verification.png")

            print("Verification script ran successfully.")

        except Exception as e:
            print(f"An error occurred: {e}")
            page.screenshot(path="jules-scratch/verification/error.png")

        finally:
            browser.close()

if __name__ == "__main__":
    run_verification()
