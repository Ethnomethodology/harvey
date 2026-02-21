# src-tauri/scripts/convert_with_pandoc.py
import pypandoc
import sys
import os
import argparse

def convert_file(input_file, output_file, to_format, reference_doc=None, extra_pandoc_args=None):
    """
    Converts a file from one format to another using pypandoc.
    """
    try:
        print(f"Starting conversion from {input_file} to {output_file} (format: {to_format})")
        
        # Ensure the output directory exists
        output_dir = os.path.dirname(output_file)
        if output_dir and not os.path.exists(output_dir):
            os.makedirs(output_dir)
            print(f"Created output directory: {output_dir}")

        extra_args = []
        if to_format == 'docx' and reference_doc and os.path.exists(reference_doc):
            print(f"Using reference document: {reference_doc}")
            extra_args.append(f"--reference-doc={reference_doc}")

        if extra_pandoc_args:
            print(f"Passing extra arguments to pandoc: {extra_pandoc_args}")
            extra_args.extend(extra_pandoc_args)

        pypandoc.convert_file(input_file, to_format, outputfile=output_file, extra_args=extra_args)
        
        print(f"Successfully converted file and saved to {output_file}")
        
    except Exception as e:
        print(f"Error during pandoc conversion: {e}", file=sys.stderr)
        sys.exit(1)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Convert files using Pandoc")
    parser.add_argument("input", help="Input file path")
    parser.add_argument("output", help="Output file path")
    parser.add_argument("format", help="Target format")
    parser.add_argument("--reference-doc", help="Optional reference DOCX for styling")
    
    # Use parse_known_args to capture any additional flags (like --lua-filter)
    args, unknown = parser.parse_known_args()

    if not os.path.exists(args.input):
        print(f"Error: Input file not found at {args.input}", file=sys.stderr)
        sys.exit(1)
        
    convert_file(args.input, args.output, args.format, args.reference_doc, extra_pandoc_args=unknown)
