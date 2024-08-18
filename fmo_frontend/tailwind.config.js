/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "**/*.rs"],
  },
  plugins: [
  ],
  // https://github.com/fedimint/ui/blob/master/packages/ui/src/theme.tsx
  theme: {
    fontFamily: {
      heading: ['Space Grotesk', 'monospace'],
      body: ['Inter', 'sans-serif'],
    },
  },
};
