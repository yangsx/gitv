import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';
import svelteParser from 'svelte-eslint-parser';

export default ts.config(
	js.configs.recommended,
	...ts.configs.recommended,
	...svelte.configs['flat/recommended'],
	prettier,
	{
		ignores: ['build/**', '.svelte-kit/**', 'package/**']
	},
	{
		files: ['**/*.svelte', '**/*.ts'],
		languageOptions: {
			globals: {
				window: 'readonly',
				document: 'readonly',
				URLSearchParams: 'readonly',
				ResizeObserver: 'readonly',
				devicePixelRatio: 'readonly',
				HTMLCanvasElement: 'readonly',
				HTMLDivElement: 'readonly',
				CanvasRenderingContext2D: 'readonly',
				Event: 'readonly',
				Map: 'readonly'
			}
		}
	},
	{
		files: ['**/*.svelte'],
		languageOptions: {
			parser: svelteParser,
			parserOptions: {
				parser: ts.parser,
				svelteConfig: {
					compilerOptions: {
						runes: true
					}
				}
			}
		}
	},
	{
		rules: {
			'no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
			'@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
			'svelte/no-unused-svelte-ignore': 'off'
		}
	}
);
