function escapeHtml(text: string): string {
	return text
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#039;');
}

const DANGEROUS_PROTOCOLS = /^\s*(javascript|data|vbscript)\s*:/i;

const DANGEROUS_TAGS =
	/<\/?(?:script|iframe|object|embed|form|input|style|link|meta|base|svg|math)[^>]*>/gi;

const EVENT_HANDLER_ATTRS = /\s*\bon\w+\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+)/gi;

function renderInline(text: string): string {
	return escapeHtml(text)
		.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, linkText, href) => {
			const safeHref = !href || DANGEROUS_PROTOCOLS.test(href) ? '#' : href;
			return `<a href="${safeHref}" target="_blank" rel="noopener noreferrer">${linkText}</a>`;
		})
		.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
		.replace(/\*(.+?)\*/g, '<em>$1</em>')
		.replace(/`(.+?)`/g, '<code>$1</code>');
}

export function renderMarkdown(body: string): string {
	const lines = body.split('\n');
	const blocks: string[] = [];
	let inCodeBlock = false;
	let codeContent: string[] = [];
	let paragraph: string[] = [];
	let listItems: string[] = [];

	function flushParagraph() {
		if (paragraph.length > 0) {
			blocks.push(`<p>${paragraph.map(renderInline).join(' ')}</p>`);
			paragraph = [];
		}
	}

	function flushList() {
		if (listItems.length > 0) {
			blocks.push(`<ul>${listItems.map((item) => `<li>${renderInline(item)}</li>`).join('')}</ul>`);
			listItems = [];
		}
	}

	function flushAll() {
		flushParagraph();
		flushList();
	}

	for (const line of lines) {
		if (line.startsWith('```')) {
			flushAll();
			if (inCodeBlock) {
				blocks.push(`<pre><code>${escapeHtml(codeContent.join('\n'))}</code></pre>`);
				codeContent = [];
				inCodeBlock = false;
			} else {
				inCodeBlock = true;
			}
			continue;
		}
		if (inCodeBlock) {
			codeContent.push(line);
			continue;
		}
		const trimmed = line.trim();
		if (!trimmed) {
			flushAll();
			continue;
		}
		const headingMatch = trimmed.match(/^(#{1,6})\s+(.+)$/);
		if (headingMatch) {
			flushAll();
			blocks.push(
				`<p class="markdown-heading"><strong>${renderInline(headingMatch[2])}</strong></p>`
			);
			continue;
		}
		if (trimmed.startsWith('- ') || trimmed.startsWith('* ')) {
			flushParagraph();
			listItems.push(trimmed.slice(2));
			continue;
		}
		flushList();
		paragraph.push(trimmed);
	}

	flushAll();

	if (inCodeBlock && codeContent.length > 0) {
		blocks.push(`<pre><code>${escapeHtml(codeContent.join('\n'))}</code></pre>`);
	}

	let html = blocks.join('\n');
	html = html.replace(DANGEROUS_TAGS, '').replace(EVENT_HANDLER_ATTRS, '');
	return html;
}
