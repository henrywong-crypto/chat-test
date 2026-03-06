/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./crates/app/src/**/*.rs",
    "./style/**/*.scss",
  ],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Fira Code", "monospace"],
      },
      colors: {
        // Neutral grays (open-webui inspired)
        gray: {
          50:  "#f9fafb",
          100: "#f3f4f6",
          200: "#e5e7eb",
          300: "#d1d5db",
          400: "#9ca3af",
          500: "#6b7280",
          600: "#4b5563",
          700: "#374151",
          800: "#1f2937",
          850: "#18212f",
          900: "#111827",
          950: "#0d1117",
        },
      },
      transitionProperty: {
        width: "width",
      },
      spacing: {
        "sidebar": "var(--sidebar-width, 260px)",
      },
    },
  },
  plugins: [
    require("@tailwindcss/typography"),
  ],
};
