import os
from datetime import datetime, timezone

# --- Create the XML content ---
# Note: Using a simple f-string for this one-off file is easier than
# setting up a full XML library for this specific structure.
def create_xml_content(project_name, project_path):
    now_ts = int(datetime.now(timezone.utc).timestamp())

    xml_content = f"""
<ProjectInfo>
    <name>{project_name}</name>
    <path>{project_path}</path>
    <created_ts>{now_ts}</created_ts>
    <last_opened_ts>{now_ts}</last_opened_ts>
</ProjectInfo>
"""
    return xml_content

# --- Main execution ---
if __name__ == "__main__":
    # Define project details
    project_name = "test-project"
    # Get the absolute path of the current working directory
    # The script will be run from the root of the repo, which is /app
    base_dir = os.getcwd()
    project_dir = os.path.join(base_dir, "jules-scratch", "verification", "test-project")
    project_xml_path = os.path.join(project_dir, f"{project_name}.harvey.xml")

    # Create the XML string
    xml_data = create_xml_content(project_name, project_xml_path)

    # Ensure the directory exists
    os.makedirs(project_dir, exist_ok=True)

    # Write the content to the .harvey.xml file
    try:
        with open(project_xml_path, "w") as f:
            f.write(xml_data.strip())
        print(f"Successfully created project file at: {project_xml_path}")
    except IOError as e:
        print(f"Error writing to file: {e}")
