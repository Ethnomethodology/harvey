import re
import time
from playwright.sync_api import Page, expect

def test_new_tag_creation(page: Page):
    """
    This test verifies that creating a new tag works correctly and the tag is available across the project.
    """
    # 1. Arrange: Go to the application and open a project.
    page.goto("http://localhost:5173")
    project_list = page.get_by_role("button", name=re.compile("Open project:"))
    expect(project_list.first).to_be_visible()
    project_list.first.click()

    # 2. Act: Open the first document.
    # The documents are in a list. We'll click the first one.
    doc_list = page.locator("ul[aria-label='Document list'] > li")
    expect(doc_list.first).to_be_visible()
    doc_list.first.click()

    # Give time for the document to load.
    time.sleep(2)

    # 3. Act: Create a highlight.
    # We will select some text in the document viewer.
    # The document viewer is an iframe, so we need to switch to it first.
    iframe = page.frame_locator('iframe[title="PDF viewer"]')
    # The text is in a div with class 'textLayer'.
    text_layer = iframe.locator('.textLayer').first
    # Select some text to trigger the highlight popup.
    text_layer.select_text()

    # The "Add Highlight" button should appear.
    add_highlight_button = iframe.locator('button:has-text("Add Highlight")')
    expect(add_highlight_button).to_be_visible()
    add_highlight_button.click()

    # 4. Act: Create a new tag.
    # The highlights panel should be visible.
    highlights_panel = page.locator('div.h-full.bg-white.dark\\:bg-gray-800').nth(1)
    expect(highlights_panel).to_be_visible()

    # Find the tag multi-select component.
    tag_multi_select = highlights_panel.locator('.relative.w-full').first
    tag_multi_select.click()

    # Type the new tag name in the input.
    new_tag_name = f"new-tag-{int(time.time())}"
    tag_input = tag_multi_select.locator('input[type="text"]')
    tag_input.fill(new_tag_name)

    # Click the "Create new tag" button.
    create_tag_button = tag_multi_select.locator(f'li:has-text("+ Create new tag \\"{new_tag_name}\\"")')
    expect(create_tag_button).to_be_visible()
    create_tag_button.click()

    # 5. Assert: Check that the new tag is assigned to the highlight.
    assigned_tag = highlights_panel.locator(f'span:has-text("{new_tag_name}")')
    expect(assigned_tag).to_be_visible()

    # 6. Act: Go to the "Tags" tab.
    tags_tab = page.get_by_role("button", name="Tags")
    expect(tags_tab).to_be_visible()
    tags_tab.click()

    # 7. Assert: Check that the new tag is in the list of all tags.
    tag_list = page.locator("ul")
    new_tag_in_list = tag_list.locator(f'li:has-text("{new_tag_name}")')
    expect(new_tag_in_list).to_be_visible()

    # 8. Screenshot: Capture the final result for visual verification.
    page.screenshot(path="jules-scratch/verification/verification.png")
