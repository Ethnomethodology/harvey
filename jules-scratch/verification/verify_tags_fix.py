import re
from playwright.sync_api import Page, expect

def test_tags_load(page: Page):
    """
    This test verifies that the tags load correctly in the "Tags" tab.
    """
    # 1. Arrange: Go to the application.
    page.goto("http://localhost:5173")

    # 2. Act: Click on the first project in the list.
    # The projects are divs with role="button".
    project_list = page.get_by_role("button", name=re.compile("Open project:"))
    expect(project_list.first).to_be_visible()
    project_list.first.click()

    # 3. Act: Click on the "Tags" tab.
    # The tags tab is a button with the text "Tags".
    tags_tab = page.get_by_role("button", name="Tags")
    expect(tags_tab).to_be_visible()
    tags_tab.click()

    # 4. Assert: Check that the tags are loaded.
    # The tags are in a list, so we look for a list item.
    # We'll wait for at least one tag to be visible.
    tag_list = page.locator("ul")
    first_tag = tag_list.locator("li").first
    expect(first_tag).to_be_visible()

    # 5. Screenshot: Capture the final result for visual verification.
    page.screenshot(path="jules-scratch/verification/verification.png")
