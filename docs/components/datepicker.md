# DatePicker Component Specification

The `DatePicker` component is a custom UI element designed to provide a premium date selection experience while supporting Japanese-specific requirements.

## Motivation

- Native `<input type="date">` displays vary across browsers.
- Native date pickers often do not show the day of the week in Japanese (e.g., showing `()` instead of `(金)`).
- Need for a consistent, high-end look and feel across the application.

## Specifications

### Visual Design

- **Display Format**: `YYYY/MM/DD (Day)`
    - `Day` is the Japanese Kanji representation (e.g., 月, 火, 水, 木, 金, 土, 日).
- **Icons**: Uses FontAwesome `far fa-calendar-alt` for the calendar icon and `fas fa-chevron-down` for the dropdown indicator.
- **States**:
    - **Default**: White background, slate-200 border, shadow-sm.
    - **Hover**: Brand-300 border, shadow-md, brand-50 outer ring.
    - **Active**: Slight scale down (98%).
    - **Disabled**: Reduced opacity, gray background, no hover effects.
    - **Empty State**: Displays "日付を選択" (Select Date) with muted colors.

### Interactive Behavior

- Clicking anywhere on the component triggers the date picker.
- Implementation uses the native `showPicker()` API via JavaScript reflection for safety.
- Fallback to `focus()` is provided for browsers where `showPicker()` is not available.
- The actual state is maintained by a hidden native `<input type="date">` to ensure compatibility with browser features and Leptos signals.

### Data Binding

- Supports `RwSignal<String>` for two-way data binding.
- Value format is standard `YYYY-MM-DD` for interoperability with backend APIs.

## Usage

```rust
use crate::components::forms::DatePicker;

#[component]
fn MyComponent() -> impl IntoView {
    let date_signal = create_rw_signal("2025-12-28".to_string());

    view! {
        <DatePicker
            value=date_signal
            label=Some("日付設定")
            disabled=false
        />
    }
}
```

## Implementation Details

- **Location**: `frontend/src/components/forms.rs`
- **Dependencies**:
    - `chrono`: For date parsing and weekday formatting.
    - `leptos`: For component structure and signals.
    - `web-sys`: To interact with the DOM and call native APIs.
    - `wasm-bindgen`, `js-sys`: For JS interop (reflection).
