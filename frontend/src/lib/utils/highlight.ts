import type { Highlighter } from 'shiki';

export interface HighlightToken {
	content: string;
	color?: string;
}

const HIGHLIGHT_LINE_CAP = 5000;

const THEME = 'github-dark';

const LANGUAGES = [
	'javascript',
	'typescript',
	'jsx',
	'tsx',
	'rust',
	'python',
	'go',
	'java',
	'c',
	'cpp',
	'css',
	'html',
	'json',
	'yaml',
	'toml',
	'markdown',
	'bash',
	'sql',
	'svelte',
	'xml',
	'vue'
] as const;

const EXTENSION_MAP: Record<string, string> = {
	rs: 'rust',
	ts: 'typescript',
	tsx: 'tsx',
	js: 'javascript',
	jsx: 'jsx',
	mjs: 'javascript',
	cjs: 'javascript',
	mts: 'typescript',
	cts: 'typescript',
	py: 'python',
	pyi: 'python',
	go: 'go',
	java: 'java',
	c: 'c',
	h: 'c',
	cpp: 'cpp',
	cc: 'cpp',
	cxx: 'cpp',
	hpp: 'cpp',
	hxx: 'cpp',
	css: 'css',
	scss: 'css',
	less: 'css',
	html: 'html',
	htm: 'html',
	svg: 'xml',
	json: 'json',
	jsonc: 'json',
	json5: 'json',
	yaml: 'yaml',
	yml: 'yaml',
	toml: 'toml',
	md: 'markdown',
	mdx: 'markdown',
	sh: 'bash',
	bash: 'bash',
	zsh: 'bash',
	fish: 'bash',
	sql: 'sql',
	svelte: 'svelte',
	xml: 'xml',
	vue: 'vue',
	gradle: 'groovy',
	groovy: 'groovy',
	kt: 'kotlin',
	kts: 'kotlin',
	swift: 'swift',
	rb: 'ruby',
	php: 'php',
	lua: 'lua',
	dart: 'dart',
	r: 'r',
	dockerfile: 'dockerfile',
	makefile: 'makefile',
	cmake: 'cmake',
	nix: 'nix',
	graphql: 'graphql',
	gql: 'graphql',
	proto: 'proto',
	ini: 'ini',
	conf: 'ini',
	cfg: 'ini',
	dotenv: 'dotenv',
	env: 'dotenv'
};

const FILENAME_MAP: Record<string, string> = {
	dockerfile: 'dockerfile',
	makefile: 'makefile',
	cmakelists: 'cmake',
	'.env': 'dotenv',
	'.bashrc': 'bash',
	'.zshrc': 'bash',
	'.gitignore': 'gitignore',
	'.gitattributes': 'gitignore'
};

let _highlighter: Highlighter | null = null;
let _initPromise: Promise<Highlighter> | null = null;

async function getHighlighter(): Promise<Highlighter> {
	if (_highlighter) return _highlighter;
	if (_initPromise) return _initPromise;

	_initPromise = (async () => {
		const { createHighlighter } = await import('shiki');
		const hl = await createHighlighter({
			themes: [THEME],
			langs: [...LANGUAGES]
		});
		_highlighter = hl;
		return hl;
	})();

	return _initPromise;
}

export function detectLanguage(filePath: string): string | null {
	const basename = filePath.split('/').pop()?.toLowerCase() ?? '';
	if (basename in FILENAME_MAP) return FILENAME_MAP[basename];

	const dot = basename.lastIndexOf('.');
	if (dot < 0) return null;
	const ext = basename.slice(dot + 1);
	return EXTENSION_MAP[ext] ?? null;
}

export async function highlightLines(
	code: string,
	filePath: string
): Promise<HighlightToken[][] | null> {
	const lineCount = code.split('\n').length;
	if (lineCount > HIGHLIGHT_LINE_CAP) return null;

	const lang = detectLanguage(filePath);
	if (!lang) return null;

	try {
		const hl = await getHighlighter();
		const result = hl.codeToTokens(code, {
			lang: lang as never,
			theme: THEME
		});
		return result.tokens.map((line) =>
			line.map((token) => ({
				content: token.content,
				color: token.color ?? undefined
			}))
		);
	} catch {
		return null;
	}
}
