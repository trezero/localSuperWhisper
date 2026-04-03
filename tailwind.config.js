/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        surface: {
          DEFAULT: "#242424",
          dark: "#1a1a1a",
          light: "#2e2e2e",
          hover: "#383838",
        },
        accent: {
          DEFAULT: "#8b5cf6",
          hover: "#7c3aed",
          muted: "rgba(139, 92, 246, 0.15)",
        },
        text: {
          primary: "#e4e4e7",
          secondary: "#a1a1aa",
          muted: "#71717a",
        },
      },
      fontFamily: {
        sans: ['"Segoe UI"', "system-ui", "sans-serif"],
      },
    },
  },
  plugins: [],
};
