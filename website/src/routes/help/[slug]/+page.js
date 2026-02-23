import { error, redirect } from '@sveltejs/kit';
import { base } from '$app/paths';

export async function load({ params, parent }) {
    if (params.slug === 'overview') {
        throw redirect(301, `${base}/help`);
    }

    const { articles } = await parent();
    
    const modules = import.meta.glob('/src/content/help/*.md');
    const path = `/src/content/help/${params.slug}.md`;
    
    if (!modules[path]) {
        throw error(404, {
            message: 'Not found'
        });
    }

    const post = await modules[path]();

    // Find current index in the pre-sorted list from layout
    const currentIndex = articles.findIndex(a => a.slug === params.slug);

    // Previous and Next logic based on global order
    const prev = currentIndex > 0 ? articles[currentIndex - 1] : null;
    const next = currentIndex < articles.length - 1 ? articles[currentIndex + 1] : null;

    return {
        content: post.default,
        meta: post.metadata,
        articles, // Still passing all articles for consistency
        prev,
        next
    };
}
