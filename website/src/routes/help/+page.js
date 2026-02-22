export async function load() {
    const modules = import.meta.glob('/src/content/help/*.md');
    const articles = [];

    for (const path in modules) {
        const module = await modules[path]();
        const slug = path.split('/').pop().replace('.md', '');
        articles.push({
            slug,
            title: module.metadata.title || slug.replace(/-/g, ' '),
            description: module.metadata.description || '',
            order: module.metadata.order || 999
        });
    }

    // Sort by order
    articles.sort((a, b) => a.order - b.order);

    return { articles };
}
