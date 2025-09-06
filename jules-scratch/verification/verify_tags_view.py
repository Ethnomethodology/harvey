from playwright.sync_api import sync_playwright, expect

def run(playwright):
    browser = playwright.chromium.launch(headless=True)
    context = browser.new_context()
    page = context.new_page()

    try:
        # 1. Start the app
        page.goto("http://localhost:6001/", timeout=10000)

        # 2. Navigate to project view
        # Click on the first project in the list.
        page.locator(".project-card").first.click()

        # 3. Navigate to Tags view
        page.get_by_role("button", name="Tags").click()

        # 4. Select a tag
        # Click on the first tag in the list.
        page.locator("li.p-2.rounded-md").first.click()

        # Wait for the table to be visible
        expect(page.locator(".tabulator")).to_be_visible()

        # 5. Take a screenshot
        screenshot_path = "jules-scratch/verification/tags_view.png"
        page.screenshot(path=screenshot_path)
        print(f"Screenshot saved to {screenshot_path}")

    except Exception as e:
        print(f"An error occurred: {e}")
        # Take a screenshot on error to help debug
        page.screenshot(path="jules-scratch/verification/error.png")
        print("Error screenshot saved to jules-scratch/verification/error.png")

    finally:
        browser.close()

with sync_playwright() as playwright:
    run(playwright)
