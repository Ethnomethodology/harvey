import { json } from '@sveltejs/kit';

export const prerender = true;

export async function GET() {
    const modules = import.meta.glob('/src/content/help/*.md', { eager: true });
    const articles = [];

    for (const path in modules) {
        const module = modules[path];
        const slug = path.split('/').pop().replace('.md', '');
        const metadata = module.metadata || {};

        articles.push({
            slug,
            title: metadata.label || metadata.title || slug.replace(/-/g, ' '),
            description: metadata.description || '',
            order: metadata.order || 999,
            sidebarId: metadata.sidebarId || 'overview'
        });
    }

    // Sort by order
    articles.sort((a, b) => a.order - b.order);

    return json(articles);
}
