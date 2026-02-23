import { base } from '$app/paths';

export async function load({ fetch }) {
    const response = await fetch(`${base}/api/help-articles.json`);
    const articles = await response.json();

    return { 
        articles
    };
}
