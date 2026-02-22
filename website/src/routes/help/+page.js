export async function load() {
    const modules = import.meta.glob('/src/content/help/*.md');
    const articles = [];
    let overviewContent = null;
    let overviewMeta = null;

    for (const path in modules) {
        const module = await modules[path]();
        const slug = path.split('/').pop().replace('.md', '');

        if (slug === 'overview') {
            overviewContent = module.default;
            overviewMeta = module.metadata;
        }

        articles.push({
            slug,
            title: module.metadata.label || module.metadata.title || slug.replace(/-/g, ' '),
            description: module.metadata.description || '',
            order: module.metadata.order || 999,
            sidebarId: module.metadata.sidebarId || 'overview'
        });
    }

    // Sort by order
    articles.sort((a, b) => a.order - b.order);

    return { 
        articles,
        overviewContent,
        overviewMeta
    };
}
