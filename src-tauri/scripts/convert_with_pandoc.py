# src-tauri/scripts/convert_with_pandoc.py
import pypandoc
import sys
import os

def convert_file(input_file, output_file, to_format):
    """
    Converts a file from one format to another using pypandoc.
    """
    try:
        print(f"Starting conversion from {input_file} to {output_file} (format: {to_format})")
        
        # Ensure the output directory exists
        output_dir = os.path.dirname(output_file)
        if not os.path.exists(output_dir):
            os.makedirs(output_dir)
            print(f"Created output directory: {output_dir}")

        pypandoc.convert_file(input_file, to_format, outputfile=output_file)
        
        print(f"Successfully converted file and saved to {output_file}")
        
    except Exception as e:
        print(f"Error during pandoc conversion: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: python convert_with_pandoc.py <input_file> <output_file> <to_format>", file=sys.stderr)
        sys.exit(1)
    
    input_path = sys.argv[1]
    output_path = sys.argv[2]
    output_format = sys.argv[3]

    if not os.path.exists(input_path):
        print(f"Error: Input file not found at {input_path}", file=sys.stderr)
        sys.exit(1)
        
    convert_file(input_path, output_path, output_format)
