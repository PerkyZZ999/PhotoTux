## Best approach

The best default approach is to treat your app as three layers: structure in `.ui` templates, behavior in Rust, and visual tweaks in CSS classes rather than inline per-widget styling.
GTK’s own Rust book says creating widgets directly in code is fine, but markup makes it easier to separate logic from the user interface, and the template system gives you `TemplateChild` references plus callback binding for clean wiring.

A solid setup looks like this:

- Use `.ui` files for windows, dialogs, sidebars, preferences, and repeated widget trees.
- Use Rust subclassing plus `CompositeTemplate` for custom widgets and signal handling.
- Use CSS classes for spacing, accent styling, card-like surfaces, and stateful appearance changes.
- Follow GNOME HIG patterns first, then add brand personality second.

## Responsiveness

For a Rust + GTK4 + WGPU app like yours, keeping the GTK side declarative and lightweight helps you avoid UI code turning into a bottleneck as the app grows.
GTK4 and the gtk-rs book emphasize Rust’s strengths for app development, while the HIG focuses on clear, native interaction patterns that usually feel faster than heavily custom, nonstandard UIs.

To keep the app feeling fluid:

- Prefer native GTK widgets and Libadwaita-style patterns over custom-drawn controls unless you truly need them.
- Keep CSS small and class-based; use it for appearance, not for trying to re-implement the toolkit.
- Use composite templates for complex views so the widget hierarchy stays readable and easy to update.
- Reserve WGPU for the canvas or rendering-heavy part of the app, and keep surrounding panels, controls, and forms as normal GTK widgets.
- Reuse widget types and custom components instead of rebuilding large sections ad hoc in Rust code.

## Practical rule

A good rule of thumb is: use `.ui` for layout, CSS for theme-level styling, and Rust for state, events, and rendering integration.
That gives you a codebase that is easier to iterate on visually, more consistent with GTK conventions, and less likely to feel sluggish because presentation and behavior stay cleanly separated.