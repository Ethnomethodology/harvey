export async function load({ parent }) {
    const { articles } = await parent();
    
    // Find overview content
    const modules = import.meta.glob('/src/content/help/overview.md', { eager: true });
    const overviewModule = Object.values(modules)[0];
    
    return { 
        articles,
        overviewContent: overviewModule?.default,
        overviewMeta: overviewModule?.metadata
    };
}
