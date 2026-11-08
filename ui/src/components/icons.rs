//! Lucide-Icons als Inline-SVG. Namen wie in lucide-react (`"Loader2"`,
//! `"ChevronDown"`), damit Portierungen 1:1 übernommen werden können.
//! Fehlt ein Icon: in `icons_data.rs` ergänzen (Generator: scripts/gen-icons.cjs).

use leptos::prelude::*;

pub fn icon_markup(name: &str) -> Option<&'static str> {
    super::icons_data::ICONS
        .binary_search_by(|(n, _)| (*n).cmp(name))
        .ok()
        .map(|i| super::icons_data::ICONS[i].1)
}

/// `<Icon name="Search" class="size-4"/>` - Größe/Farbe kommen über Tailwind-Klassen.
#[component]
pub fn Icon(#[prop(into)] name: Signal<&'static str>, #[prop(into, optional)] class: MaybeProp<String>) -> impl IntoView {
    let markup = move || {
        let n = name.get();
        match icon_markup(n) {
            Some(m) => m,
            None => {
                web_sys::console::warn_1(&format!("Icon fehlt: {n}").into());
                ""
            }
        }
    };
    view! {
        <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
            class=move || format!("lucide shrink-0 {}", class.get().unwrap_or_default())
            inner_html=markup
        ></svg>
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn icons_sorted_for_binary_search() {
        let names: Vec<&str> = super::super::icons_data::ICONS.iter().map(|(n, _)| *n).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
