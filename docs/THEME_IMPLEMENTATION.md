# Theme Implementation Guide

This document outlines the completed theme system implementation for the Timekeeper frontend.

## Overview
The theme system provides a centralized way to manage light/dark/high-contrast themes, replacing scattered `dark:` class usage throughout the application.

## Components

### 1. Theme Types (`state/theme.rs`)
- `Theme` enum: Light, Dark, HighContrast
- `ThemeState` struct: Global state with DOM class management
- Auto-detects system preference on init
- Listen for system theme changes via `MediaQueryList`

### 2. Theme Provider (`components/theme.rs`)
- `ThemeProvider`: Wraps app with theme class on root div
- `ThemeToggle`: Switcher button with sun/moon icons and transitions
- Uses semantic Tailwind utilities instead of hardcoded classes

### 3. Tailwind Config (`tailwind.config.js`)
- `darkMode: 'class'` for manual control
- Semantic color tokens: `primary`, `success`, `danger`, `warning`
- Surface colors: `surface.light/dark`, `text-primary`, `text-secondary`
- Theme-agnostic utilities: `shadow-theme-switch`

## Usage

### In App Root
```rust
// In your main app component
view! {
    <ThemeProvider>
        // Your app content here
        <Header />
        <main>...</main>
    </ThemeProvider>
}
```

### In Components
```rust
// Access current theme
let theme_state = use_theme();
let current = theme_state.current();

// Toggle theme
theme_state.toggle();

// Check theme in views
<Show when=move || current.get() == Theme::Dark>
    <DarkModeContent />
</Show>
```

### In Styling
Instead of:
```html
<div class="bg-white dark:bg-gray-800 text-gray-900 dark:text-gray-100">
```

Use semantic tokens:
```html
<div class="bg-surface-light dark:bg-surface-dark text-text-primary dark:text-text-primary">
```

## Next Steps

1. **Refactor Existing Components**
   - Replace all `dark:` class variants with theme token usage
   - Focus on: `Layout`, `Header`, cards, forms

2. **Add Theme Persistence** (Optional)
   - Store user preference in localStorage
   - Respect manual selection over system preference

3. **High Contrast Theme** (Optional)
   - Define specific color adjustments
   - Add to toggle cycle: Light → Dark → High Contrast → Light

## Benefits Achieved

- ✅ Centralized theme state management
- ✅ System preference detection and auto-switching
- ✅ Semantic color tokens for consistent theming
- ✅ Eliminates scattered `dark:` conditional classes
- ✅ Ready for additional themes (high contrast, custom branding)