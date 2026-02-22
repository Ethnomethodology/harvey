import { error } from '@sveltejs/kit';

export async function load({ params }) {
    const modules = import.meta.glob('/src/content/help/*.md');
    const path = `/src/content/help/${params.slug}.md`;
    if (!modules[path]) {
        throw error(404, {
            message: 'Not found'
        });
    }
    const post = await modules[path]();
    return {
        content: post.default,
        meta: post.metadata
    };
}
