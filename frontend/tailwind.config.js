/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        paper: "#f7f2e4",
        paperPage: "#ece5d3",
        paperWash: "#efe8d6",
        ink: "#211d17",
        inkSoft: "#6a6152",
        rule: "#2a251d",
        ruleSoft: "#c0b59d",
        accentStamp: "#8f1f17",
        voteUpBg: "#d7ebd9",
        voteUpBorder: "#6fa878",
        voteUpInk: "#153d1e",
        voteUpBar: "#4d8a57",
        voteDownBg: "#f8dcd9",
        voteDownBorder: "#d48b84",
        voteDownInk: "#5e1c18",
        voteDownBar: "#a33b33",
      },
      fontFamily: {
        display: ['"Playfair Display"', 'Georgia', 'serif'],
        body: ['"Source Serif 4"', 'Georgia', 'serif'],
        mono: ['"IBM Plex Mono"', '"Courier New"', 'monospace'],
      },
    },
  },
  plugins: [],
};
