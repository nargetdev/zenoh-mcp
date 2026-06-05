// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

// Deployed to GitHub Pages at https://nargetdev.github.io/zenoh-mcp/
export default defineConfig({
	site: 'https://nargetdev.github.io',
	base: '/zenoh-mcp',
	integrations: [
		starlight({
			title: 'zenoh-mcp',
			description:
				'Stdio MCP server with Zenoh debugging tools (get/put/subscribe/admin) and ROS2 CDR decoding. Pinned to zenoh 1.8.0.',
			social: [
				{
					icon: 'github',
					label: 'GitHub',
					href: 'https://github.com/nargetdev/zenoh-mcp',
				},
			],
			sidebar: [
				{
					label: 'Start here',
					items: [
						{ label: 'Overview', slug: 'index' },
						{ label: 'Getting started', slug: 'guides/getting-started' },
						{ label: 'Configuration', slug: 'guides/configuration' },
					],
				},
				{
					label: 'Reference',
					items: [
						{ label: 'MCP server spec', slug: 'reference/spec' },
						{ label: 'Tool reference', slug: 'reference/tools' },
						{ label: 'ROS2 CDR decoding', slug: 'reference/decoding' },
					],
				},
			],
		}),
	],
});
