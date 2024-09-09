/** @type {import('tailwindcss').Config} */
export default {
	content: ['./src/**/*.{html,js,svelte,ts}'],
	theme: {
		extend: {
			keyframes: {
				blink: {
					'0%, 100%': { opacity: 1 },
					'50%': { opacity: 0 }
				}
			},
			animation: {
				blink: 'blink 1s step-start infinite'
			}
		}
	},
	plugins: []
};
