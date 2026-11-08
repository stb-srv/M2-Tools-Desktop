//! Button im Stil der bisherigen shadcn-Variante (gleiche Tailwind-Klassen).

// Allgemeine Helfer-API: nicht jede Funktion/Variante wird aktuell genutzt.
#![allow(dead_code)]

use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Secondary,
    Ghost,
    Destructive,
    Link,
}

#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum ButtonSize {
    #[default]
    Default,
    Xs,
    Sm,
    Lg,
    Icon,
    IconXs,
    IconSm,
    IconLg,
}

const BASE: &str = "group/button inline-flex shrink-0 items-center justify-center rounded-lg border border-transparent bg-clip-padding text-sm font-medium whitespace-nowrap transition-all outline-none select-none focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 active:translate-y-px disabled:pointer-events-none disabled:opacity-50 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

pub fn variant_class(v: ButtonVariant) -> &'static str {
    match v {
        ButtonVariant::Default => "bg-primary text-primary-foreground hover:bg-primary/80",
        ButtonVariant::Outline => "border-border bg-background hover:bg-muted hover:text-foreground dark:border-input dark:bg-input/30 dark:hover:bg-input/50",
        ButtonVariant::Secondary => "bg-secondary text-secondary-foreground hover:bg-[color-mix(in_oklch,var(--secondary),var(--foreground)_5%)]",
        ButtonVariant::Ghost => "hover:bg-muted hover:text-foreground dark:hover:bg-muted/50",
        ButtonVariant::Destructive => "bg-destructive/10 text-destructive hover:bg-destructive/20 focus-visible:border-destructive/40 focus-visible:ring-destructive/20 dark:bg-destructive/20 dark:hover:bg-destructive/30 dark:focus-visible:ring-destructive/40",
        ButtonVariant::Link => "text-primary underline-offset-4 hover:underline",
    }
}

pub fn size_class(s: ButtonSize) -> &'static str {
    match s {
        ButtonSize::Default => "h-8 gap-1.5 px-2.5",
        ButtonSize::Xs => "h-6 gap-1 rounded-[min(var(--radius-md),10px)] px-2 text-xs [&_svg:not([class*='size-'])]:size-3",
        ButtonSize::Sm => "h-7 gap-1 rounded-[min(var(--radius-md),12px)] px-2.5 text-[0.8rem] [&_svg:not([class*='size-'])]:size-3.5",
        ButtonSize::Lg => "h-9 gap-1.5 px-2.5",
        ButtonSize::Icon => "size-8",
        ButtonSize::IconXs => "size-6 rounded-[min(var(--radius-md),10px)] [&_svg:not([class*='size-'])]:size-3",
        ButtonSize::IconSm => "size-7 rounded-[min(var(--radius-md),12px)]",
        ButtonSize::IconLg => "size-9",
    }
}

/// Vollständige Klassenliste - für Elemente, die wie ein Button aussehen
/// sollen, aber keiner sind (z.B. ein `<label>` um ein Datei-Input).
pub fn button_class(v: ButtonVariant, s: ButtonSize, extra: &str) -> String {
    format!("{BASE} {} {} {extra}", variant_class(v), size_class(s))
}

/// ```ignore
/// <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| ...>"Text"</Button>
/// ```
#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(into, optional)] class: MaybeProp<String>,
    #[prop(into, optional)] disabled: Signal<bool>,
    #[prop(into, optional)] title: MaybeProp<String>,
    /// "button" (Standard) oder "submit".
    #[prop(optional, into)] r#type: Option<&'static str>,
    #[prop(optional, into)] on_click: Option<Callback<leptos::ev::MouseEvent>>,
    children: Children,
) -> impl IntoView {
    view! {
        <button
            type=r#type.unwrap_or("button")
            data-slot="button"
            class=move || button_class(variant, size, &class.get().unwrap_or_default())
            disabled=move || disabled.get()
            title=move || title.get()
            on:click=move |e| {
                if let Some(cb) = on_click {
                    cb.run(e)
                }
            }
        >
            {children()}
        </button>
    }
}
