import { Marked, type RendererObject } from 'marked';

function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#039;');
}

const DANGEROUS_PROTOCOLS = /^\s*(javascript|data|vbscript)\s*:/i;

const DANGEROUS_TAGS = /<\/?(?:script|iframe|object|embed|form|input|style|link|meta|base)[^>]*>/gi;

const EVENT_HANDLER_ATTRS = /\s+on\w+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi;

const marked = new Marked({
	gfm: true,
	breaks: true
});

const renderer: RendererObject = {
	link({ href, title, text }) {
		const safeHref = !href || DANGEROUS_PROTOCOLS.test(href) ? '#' : href;
		const titleAttr = title ? ` title="${escapeHtml(title)}"` : '';
		return `<a href="${escapeHtml(safeHref)}" target="_blank" rel="noopener noreferrer"${titleAttr}>${text}</a>`;
	},
	html({ text }) {
		return escapeHtml(text);
	},
	heading({ text }) {
		return `<p class="markdown-heading"><strong>${text}</strong></p>`;
	}
};

marked.use({ renderer });

function sanitizeHtml(html: string): string {
	return html.replace(DANGEROUS_TAGS, '').replace(EVENT_HANDLER_ATTRS, '');
}

export function renderMarkdown(body: string): string {
	const raw = marked.parse(body, { async: false }) as string;
	return sanitizeHtml(raw);
}
