i# Frontend Theme System

## Purpose

Defines how themes are represented and applied in the frontend.

The theme system provides semantic design tokens that allow UI components to remain independent of specific colour palettes.

## Source of Truth

Theme tokens are defined as CSS variables in:

apps/frontend/app/globals.css

Tailwind v4 reads these variables through the `@theme` block and generates the corresponding utility classes.

## Theme Layers

The system has three layers.

### Default Tokens

Defined in the `@theme` block.

These provide the baseline semantic tokens used by all components.

Examples include:

color-background  
color-card  
color-primary  
color-muted  
color-border

### Dark Mode

Dark mode overrides are defined under the `.dark` selector.

### Named Themes

Named themes override tokens using attribute selectors:

[data-theme-name="standard"]  
[data-theme-name="high_roller"]  
[data-theme-name="oldtime"]

Each theme may define its own light and dark variants.

## Runtime Theme Control

The frontend applies themes by setting attributes on the root document element.

Attributes used:

data-theme-name  
data-colour-scheme

The `dark` class is applied when the resolved colour scheme is dark.

## Theme Preferences

User preferences are stored in:

localStorage

Keys:

theme_name  
colour_scheme

The application may also persist these preferences to the backend.

## Using Theme Tokens

Components must use semantic Tailwind utilities derived from theme tokens.

Examples:

bg-background  
bg-card  
text-foreground  
text-muted  
border-border  
bg-primary  
bg-success  
bg-warning  
bg-destructive

Components must not use hard-coded Tailwind palette colours unless introducing a new semantic token.

## Adding New Tokens

To introduce a new token:

1. Define the variable in the `@theme` block.
2. Add dark mode overrides if necessary.
3. Add overrides for any named themes.
4. Use the generated semantic utility in components.

No Tailwind configuration changes are required for colour tokens.
