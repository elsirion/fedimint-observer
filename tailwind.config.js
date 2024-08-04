/** @type {import('tailwindcss').Config} */
module.exports = {
  content: {
    files: ["*.html", "./fmo_frontend/src/**/*.rs"],
  },
  plugins: [
    require('flowbite/plugin')({
      charts: true,
    })
  ],
  // https://github.com/fedimint/ui/blob/master/packages/ui/src/theme.tsx
  theme: {
    fontFamily: {
      heading: ['Space Grotesk', 'monospace'],
      body: ['Inter', 'sans-serif'],
    },
  },
};
