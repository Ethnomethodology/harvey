import re

with open("src-tauri/src/projectview/core_commands.rs", "r") as f:
    content = f.read()

# Locate the load_project_data function block
start_idx = content.find("#[tauri::command]\npub async fn load_project_data(project_xml_path: String) -> Result<ProjectViewData, CommandError> {")

# Find the end of the function (counting braces)
if start_idx != -1:
    brace_count = 0
    end_idx = -1
    in_func = False

    for i in range(start_idx, len(content)):
        if content[i] == '{':
            brace_count += 1
            in_func = True
        elif content[i] == '}':
            brace_count -= 1
            if in_func and brace_count == 0:
                end_idx = i + 1
                break

    if end_idx != -1:
        print(f"Found load_project_data from {start_idx} to {end_idx}")
    else:
        print("Could not find the end of load_project_data")
else:
    print("Could not find load_project_data")
