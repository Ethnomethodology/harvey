from playwright.sync_api import sync_playwright, expect

def run_verification():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        try:
            # Step 1: Navigate to the application
            page.goto("http://localhost:5173", timeout=60000)

            # Give it a moment to load, just in case
            page.wait_for_load_state('networkidle')

            # Step 2: Take a screenshot of the initial page
            page.screenshot(path="jules-scratch/verification/01_initial_page.png")

            # Step 3: Find and click the sample data file to open the table
            # Based on the file structure, the sample CSV is likely visible.
            page.get_by_role("link", name="sample_data.csv").click()

            # Wait for the table to be visible. The table is inside a div with class 'tabulator'
            table_locator = page.locator(".tabulator")
            expect(table_locator).to_be_visible(timeout=30000)
            page.screenshot(path="jules-scratch/verification/02_table_loaded.png")

            # Step 4: Test range selection and highlighting
            # Let's select a 2x2 block of cells.
            # We'll drag from the cell in the first data row, first data column
            # to the cell in the second data row, second data column.

            # Tabulator cells are identified by row position and column field.
            # First data cell is row 1, column 'A' (assuming headers are A, B, C...)
            first_cell = page.locator('.tabulator-row[aria-rowindex="1"] .tabulator-cell[aria-colindex="1"]')
            # Second cell is row 2, column 'B'
            second_cell = page.locator('.tabulator-row[aria-rowindex="2"] .tabulator-cell[aria-colindex="2"]')

            # Perform the drag
            first_cell.hover()
            page.mouse.down()
            second_cell.hover()
            page.mouse.up()

            page.screenshot(path="jules-scratch/verification/03_range_selected.png")

            # Step 5: Right-click and apply highlight
            # Right-click on the selection (we'll use the second cell)
            second_cell.click(button="right")

            # The context menu should appear. Let's find the "Highlight Selection" item.
            highlight_menu_item = page.locator(".tabulator-menu-item[aria-label='Highlight Selection']")
            expect(highlight_menu_item).to_be_visible()
            highlight_menu_item.hover() # Hover to open the submenu

            # Now click a color. Let's pick the yellow one.
            # The label has HTML in it, so we can't use get_by_text easily.
            # We'll find it by the color value in the style attribute.
            yellow_color_option = page.locator(".tabulator-menu-item:has-text('Yellow')")
            expect(yellow_color_option).to_be_visible()
            yellow_color_option.click()

            # The menu should close and the cells should be highlighted.
            page.screenshot(path="jules-scratch/verification/04_range_highlighted.png")

            # Step 6: Verify the highlight was applied
            # Check the background color of one of the cells.
            expect(first_cell).to_have_css("background-color", "rgb(255, 255, 0)") # yellow

            # Step 7: Test column selection
            # Click on the header for column 'B'
            column_b_header = page.locator(".tabulator-col[tabulator-field='B']")
            column_b_header.click()
            page.screenshot(path="jules-scratch/verification/05_column_selected.png")

            # Step 8: Test row selection
            # Click on the row header for the third row (index 3)
            row_3_header = page.locator('.tabulator-row[aria-rowindex="3"] .range-header-col')
            row_3_header.click()
            page.screenshot(path="jules-scratch/verification/06_row_selected.png")

        except Exception as e:
            print(f"An error occurred: {e}")
            page.screenshot(path="jules-scratch/verification/error.png")
        finally:
            browser.close()

if __name__ == "__main__":
    run_verification()
