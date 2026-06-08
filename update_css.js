const fs = require('fs');

let css = `/* App-wide baseline */
body { margin: 0; padding: 0; font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; }
:root { 
    --theme-primary-50: #eff6ff;
    --theme-primary-100: #dbeafe;
    --theme-primary-200: #bfdbfe;
    --theme-primary-300: #93c5fd;
    --theme-primary-400: #60a5fa;
    --theme-primary-500: #3b82f6;
    --theme-primary-600: #2563eb;
    --theme-primary-700: #1d4ed8;
    --theme-primary-800: #1e40af;
    --theme-primary-900: #1e3a8a;
    --color-surface-50: #f9fafb; --color-surface-100: #f3f4f6; --color-surface-200: #e5e7eb; --color-surface-300: #d1d5db; --color-surface-400: #9ca3af; --color-surface-500: #6b7280; --color-surface-600: #4b5563; --color-surface-700: #374151; --color-surface-800: #1f2937; --color-surface-900: #111827; --color-surface-950: #030712;
}

body[data-theme="dark"] { 
    background-color: var(--color-surface-900); color: var(--color-surface-200); 
    --color-surface-50: #030712; --color-surface-100: #111827; --color-surface-200: #1f2937; --color-surface-300: #374151; --color-surface-400: #4b5563; --color-surface-500: #6b7280; --color-surface-600: #9ca3af; --color-surface-700: #d1d5db; --color-surface-800: #e5e7eb; --color-surface-900: #f3f4f6; --color-surface-950: #f9fafb;
}

body[data-theme="dark"] * {
    border-color: var(--color-surface-700);
}

body[data-theme="dark"] .bg-white { background-color: var(--color-surface-900) !important; }
body[data-theme="dark"] .bg-gray-50 { background-color: var(--color-surface-800) !important; }
body[data-theme="dark"] .text-gray-900 { color: var(--color-surface-50) !important; }
body[data-theme="dark"] .text-gray-800 { color: var(--color-surface-100) !important; }
body[data-theme="dark"] .text-gray-700 { color: var(--color-surface-200) !important; }
body[data-theme="dark"] .text-gray-600 { color: var(--color-surface-300) !important; }
body[data-theme="dark"] .text-gray-500 { color: var(--color-surface-400) !important; }
body[data-theme="dark"] .border-gray-200 { border-color: var(--color-surface-700) !important; }
body[data-theme="dark"] .border-gray-300 { border-color: var(--color-surface-600) !important; }
body[data-theme="dark"] input { background-color: var(--color-surface-950) !important; color: var(--color-surface-50) !important; }
body[data-theme="dark"] select { background-color: var(--color-surface-950) !important; color: var(--color-surface-50) !important; }

body[data-accent="Green"] { --theme-primary-50: #f0fdf4; --theme-primary-100: #dcfce7; --theme-primary-200: #bbf7d0; --theme-primary-300: #86efac; --theme-primary-400: #4ade80; --theme-primary-500: #22c55e; --theme-primary-600: #16a34a; --theme-primary-700: #15803d; --theme-primary-800: #166534;  --theme-primary-900: #14532d; }
body[data-accent="Purple"] { --theme-primary-50: #faf5ff; --theme-primary-100: #f3e8ff; --theme-primary-200: #e9d5ff; --theme-primary-300: #d8b4fe; --theme-primary-400: #c084fc; --theme-primary-500: #a855f7; --theme-primary-600: #9333ea; --theme-primary-700: #7e22ce; --theme-primary-800: #6b21a8; --theme-primary-900: #581c87; }
body[data-accent="Rose"] { --theme-primary-50: #fff1f2; --theme-primary-100: #ffe4e6; --theme-primary-200: #fecdd3; --theme-primary-300: #fda4af; --theme-primary-400: #fb7185; --theme-primary-500: #f43f5e; --theme-primary-600: #e11d48; --theme-primary-700: #be123c; --theme-primary-800: #9f1239; --theme-primary-900: #881337; }
body[data-accent="Amber"] { --theme-primary-50: #fffbeb; --theme-primary-100: #fef3c7; --theme-primary-200: #fde68a; --theme-primary-300: #fcd34d; --theme-primary-400: #fbbf24; --theme-primary-500: #f59e0b; --theme-primary-600: #d97706; --theme-primary-700: #b45309; --theme-primary-800: #92400e; --theme-primary-900: #215a45; }

.bg-primary-500 { background-color: var(--theme-primary-500) !important; }
.bg-primary-600 { background-color: var(--theme-primary-600) !important; }
.hover\\:bg-primary-700:hover { background-color: var(--theme-primary-700) !important; }

.text-primary-500 { color: var(--theme-primary-500) !important; }
.text-primary-600 { color: var(--theme-primary-600) !important; }
.text-primary-700 { color: var(--theme-primary-700) !important; }
.text-primary-800 { color: var(--theme-primary-800) !important; }
.border-primary-200 { border-color: var(--theme-primary-200) !important; }
.border-primary-500 { border-color: var(--theme-primary-500) !important; }
.bg-primary-50 { background-color: var(--theme-primary-50) !important; }
.bg-primary-100 { background-color: var(--theme-primary-100) !important; }
.focus\\:ring-primary-500:focus { --tw-ring-color: var(--theme-primary-500) !important; }
.hover\\:bg-primary-50:hover { background-color: var(--theme-primary-50) !important; }
`;
fs.writeFileSync('/home/maez/code/personal/theo-manager/assets/main.css', css);
