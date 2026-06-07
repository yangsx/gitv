import { Marked, type RendererObject } from 'marked';

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#039;');
}

const marked = new Marked({
	gfm: true,
	breaks: true
});

const renderer: RendererObject = {
	link({ href, title, text }) {
		const titleAttr = title ? ` title="${title}"` : '';
		return `<a href="${href}" target="_blank" rel="noopener noreferrer"${titleAttr}>${text}</a>`;
	},
	html({ text }) {
		return escapeHtml(text);
	},
	heading({ text }) {
		return `<p class="markdown-heading"><strong>${text}</strong></p>`;
	}
};

marked.use({ renderer });

export function renderMarkdown(body: string): string {
	return marked.parse(body, { async: false }) as string;
}
