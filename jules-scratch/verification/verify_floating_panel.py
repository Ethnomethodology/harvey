from playwright.sync_api import sync_playwright

with sync_playwright() as p:
    browser = p.chromium.launch(headless=True)
    page = browser.new_page()
    page.goto("http://localhost:1420")

    # Click on the project
    page.click('text="Test Project"')

    # Navigate to the "Tags" tab
    page.click('text="Tags"')

    # Click on a tag
    page.click('text="test"')

    # Click on the "Comments" button for the first highlight
    page.click('button[title="Comments"]')

    # Wait for the panel to slide in
    page.wait_for_selector('.fixed.top-0.right-0')

    # Take a screenshot
    page.screenshot(path="jules-scratch/verification/verification.png")

    browser.close()
