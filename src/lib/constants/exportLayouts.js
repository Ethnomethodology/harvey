export const DOCX_LAYOUT_OPTIONS = [
	{
		id: 'layout-1',
		name: 'Detailed Table',
		description: '| No | Timestamp | Speaker | Text |',
		columns: 4,
		rustLayoutKey: 'Layout1',
		previewClasses: 'grid grid-cols-10 gap-1 text-xs p-1 border rounded',
		columnStyles: [
			{ content: '#', class: 'col-span-1 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
			{ content: 'Time', class: 'col-span-2 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
			{ content: 'Spk', class: 'col-span-2 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
			{ content: 'Text', class: 'col-span-5 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
		],
	},
	{
		id: 'layout-2',
		name: 'Segment Block',
		description: '| No | Timestamp |\n| Speaker | Text |',
		columns: 2, // Visually two, but data-wise could be seen as 2x2
		rustLayoutKey: 'Layout2',
		previewClasses: 'grid grid-cols-4 gap-1 text-xs p-1 border rounded',
		columnStyles: [
			// Row 1
			{ content: '#', class: 'col-span-1 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
			{ content: 'Time', class: 'col-span-3 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
			// Row 2
			{ content: 'Spk', class: 'col-span-1 row-start-2 bg-slate-300 dark:bg-slate-600 p-0.5 rounded text-center truncate' },
			{ content: 'Text', class: 'col-span-3 row-start-2 bg-slate-300 dark:bg-slate-600 p-0.5 rounded text-center truncate' },
		],
	},
	{
		id: 'layout-3',
		name: 'Timestamped Paragraph',
		description: '| Timestamp Speaker |\n| Text |',
		columns: 1, // Visually one, but data-wise could be seen as 1x2
		rustLayoutKey: 'Layout3',
		previewClasses: 'grid grid-cols-1 gap-1 text-xs p-1 border rounded',
		columnStyles: [
			{ content: 'Time Spk', class: 'col-span-1 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
			{ content: 'Text', class: 'col-span-1 row-start-2 bg-slate-300 dark:bg-slate-600 p-0.5 rounded text-center truncate' },
		],
	},
	{
		id: 'layout-4',
		name: 'Speaker & Text',
		description: '| Speaker | Text |',
		columns: 2,
		rustLayoutKey: 'Layout4',
		previewClasses: 'grid grid-cols-10 gap-1 text-xs p-1 border rounded',
		columnStyles: [
			{ content: 'Spk', class: 'col-span-3 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' }, // Approx 25%
			{ content: 'Text', class: 'col-span-7 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' }, // Approx 75%
		],
	},
	{
		id: 'layout-5',
		name: 'Plain Text',
		description: '| Text |',
		columns: 1,
		rustLayoutKey: 'Layout5',
		previewClasses: 'grid grid-cols-1 gap-1 text-xs p-1 border rounded',
		columnStyles: [
			{ content: 'Text', class: 'col-span-1 bg-slate-200 dark:bg-slate-700 p-0.5 rounded text-center truncate' },
		],
	},
];

export const DOCX_LAYOUT_COLUMN_CONFIGS = {
	Layout1: { colgroup: ["5%", "15%", "15%", "65%"] },
	Layout2: { colgroup: ["5%", "15%", "25%", "75%"] }, // Updated for 2x2 logical layout: No, Timestamp, Speaker, Text
	Layout3: { colgroup: ["100%"] }, // Single column for both rows
	Layout4: { colgroup: ["25%", "75%"] },
	Layout5: { colgroup: ["100%"] },
};
