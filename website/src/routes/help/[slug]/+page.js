import { error } from '@sveltejs/kit';

export async function load({ params }) {
    const modules = import.meta.glob('/src/content/help/*.md');
    const articles = [];

    // Load all articles to determine order
    for (const path in modules) {
        const module = await modules[path]();
        const slug = path.split('/').pop().replace('.md', '');
        articles.push({
            slug,
            title: module.metadata.label || module.metadata.title || slug.replace(/-/g, ' '),
            description: module.metadata.description || '',
            order: module.metadata.order || 999,
            sidebarId: module.metadata.sidebarId || 'overview'
        });
    }

    // Sort globally by order
    articles.sort((a, b) => a.order - b.order);

    const path = `/src/content/help/${params.slug}.md`;
    if (!modules[path]) {
        throw error(404, {
            message: 'Not found'
        });
    }

    const post = await modules[path]();

    // Find current index
    const currentIndex = articles.findIndex(a => a.slug === params.slug);

    // Previous and Next logic based on global order
    const prev = currentIndex > 0 ? articles[currentIndex - 1] : null;
    const next = currentIndex < articles.length - 1 ? articles[currentIndex + 1] : null;

    return {
        content: post.default,
        meta: post.metadata,
        articles, // Pass all articles for sidebar context
        prev,
        next
    };
}
