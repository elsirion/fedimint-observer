# Color Palette & Styling Comparison

## ✅ Matching Color Palette

### Background Colors
- **Body Background**: `dark:bg-gray-900` (#111827) ✅ MATCHES
- **Navbar**: `bg-white` (light) / `dark:bg-gray-900` (dark) ✅ MATCHES
- **Tables**: `bg-white` (light) / `dark:bg-gray-800` (dark) ✅ MATCHES
- **Table Header**: `bg-gray-50` (light) / `dark:bg-gray-700` (dark) ✅ MATCHES

### Text Colors
- **Primary Text**: `text-gray-900` (light) / `dark:text-white` (dark) ✅ MATCHES
- **Secondary Text**: `text-gray-500` (light) / `dark:text-gray-400` (dark) ✅ MATCHES
- **Links**: `text-blue-600` (light) / `dark:text-blue-500` (dark) ✅ MATCHES
- **Link Hover**: `hover:text-blue-700` ✅ MATCHES

### Border Colors
- **Default**: `border-gray-200` (light) / `dark:border-gray-700` (dark) ✅ MATCHES
- **Table Borders**: `border-gray-700` (dark) ✅ MATCHES

### Component Colors
- **Buttons Primary**: `bg-blue-700` / `hover:bg-blue-800` ✅ MATCHES
- **Buttons Secondary**: `bg-white border-gray-200` (light) / `dark:bg-gray-800` (dark) ✅ MATCHES
- **Active Nav Link**: `bg-blue-700` / `md:text-blue-700` ✅ MATCHES

### Badge Colors
- **Info**: `bg-blue-100 text-blue-800` (light) / `dark:bg-blue-900 dark:text-blue-300` (dark) ✅ MATCHES
- **Warning**: `bg-yellow-100 text-yellow-800` (light) / `dark:bg-yellow-900 dark:text-yellow-300` (dark) ✅ MATCHES
- **Error**: `bg-red-100 text-red-800` (light) / `dark:bg-red-900 dark:text-red-300` (dark) ✅ MATCHES
- **Success**: `bg-green-100 text-green-800` (light) / `dark:bg-green-900 dark:text-green-300` (dark) ✅ MATCHES

### Alert Colors
- **Info**: `text-blue-800 bg-blue-50` (light) / `dark:text-blue-400` (dark) ✅ MATCHES
- **Warning**: `text-yellow-800 bg-yellow-50` (light) / `dark:text-yellow-300` (dark) ✅ MATCHES
- **Error**: `text-red-800 bg-red-50` (light) / `dark:text-red-400` (dark) ✅ MATCHES
- **Success**: `text-green-800 bg-green-50` (light) / `dark:text-green-400` (dark) ✅ MATCHES

### Special Colors
- **Star Rating**: `text-yellow-300` ✅ MATCHES
- **GitHub Button**: `dark:bg-gray-800 dark:text-gray-400` ✅ MATCHES
- **Divider**: `border-gray-300` ✅ MATCHES

## ✅ Typography

### Font Sizes
- **Heading (h1)**: `text-2xl` (24px) ✅ MATCHES
- **Stats Numbers**: `text-4xl` (36px) ✅ MATCHES
- **Table Caption**: `text-lg` (18px) ✅ MATCHES
- **Table Headers**: `text-xs uppercase` ✅ MATCHES
- **Body Text**: `text-sm` (14px) ✅ MATCHES
- **Small Text**: `text-xs` (12px) ✅ MATCHES

### Font Weights
- **Bold**: `font-bold` ✅ MATCHES
- **Semibold**: `font-semibold` ✅ MATCHES
- **Medium**: `font-medium` ✅ MATCHES
- **Normal**: Default ✅ MATCHES

## ✅ Spacing & Layout

### Container
- **Max Width**: `max-w-6xl` (1152px) ✅ MATCHES
- **Padding**: `px-4` (16px) ✅ MATCHES
- **Container**: `mx-auto` (centered) ✅ MATCHES

### Component Spacing
- **Navbar Padding**: `p-4` ✅ MATCHES
- **Table Cell Padding**: `px-6 py-4` ✅ MATCHES
- **Stats Spacing**: `space-x-10` ✅ MATCHES
- **Margin Bottom**: `mb-4`, `mb-2` etc. ✅ MATCHES

## ✅ Special Features

### Dark Mode
- **Enabled by Default**: Yes ✅ MATCHES
- **Dark Class**: Added to `<html>` element ✅ MATCHES
- **Body Background**: `dark:bg-gray-900` ✅ MATCHES

### Formatting
- **Bitcoin Display**:
  - Table cells: `XX.XXXXXX BTC` ✅ MATCHES
  - Total Volume: Bitcoin symbol + number (no text) ✅ MATCHES
- **Number Formatting**: Thousand separators ✅ MATCHES
- **Decimal Precision**: 6 decimals for BTC ✅ MATCHES

### Icons
- **GitHub Icon**: SVG path identical ✅ MATCHES
- **Bitcoin Symbol**: SVG path identical ✅ MATCHES
- **Star Icon**: SVG path identical ✅ MATCHES
- **Copy Icon**: SVG path identical ✅ MATCHES
- **Checkmark Icon**: SVG path for copied state ✅ ADDED

## 📊 Summary

**Color Palette Match: 100%** ✅

All Tailwind CSS color classes used in the React version exactly match the Leptos version:
- Gray scale: gray-50, gray-100, gray-200, gray-300, gray-400, gray-500, gray-600, gray-700, gray-800, gray-900
- Blue scale: blue-50, blue-100, blue-300, blue-400, blue-500, blue-600, blue-700, blue-800, blue-900
- Yellow scale: yellow-50, yellow-100, yellow-300, yellow-800, yellow-900
- Red scale: red-50, red-100, red-400, red-800, red-900
- Green scale: green-50, green-100, green-400, green-600, green-700, green-800, green-900

Both implementations use the same Tailwind CSS framework with identical class names, ensuring perfect color consistency.
