use maud::{html, Markup, PreEscaped};

pub fn get_styles() -> Markup {
    html! {
        style {
            (PreEscaped(include_str!("../styles/main.css")))
        }
    }
}
