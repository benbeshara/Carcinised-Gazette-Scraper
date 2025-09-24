use maud::{html, Markup, DOCTYPE};

pub fn base_template(content: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        html {
            (head_section())
            body {
                (content)
            }
        }
    }
}

fn head_section() -> Markup {
    html! {
        head {
            title { "Control of Weapons Acts" }
            meta name="viewport" content="width=device-width";
            link rel="icon" type="image/x-icon" href="favicon.ico";
            (leaflet_resources())
        }
    }
}

fn leaflet_resources() -> Markup {
    html! {
        link rel="stylesheet"
            href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css"
            integrity="sha256-p4NxAoJBhIIN+hmNHrzRCf9tD/miZyoHS5obTRR9BMY="
            crossorigin="";
        script type="text/javascript"
            src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js"
            integrity="sha256-20nQCchB9co0qIjJZRGuk2/Z9VM+kNiyxNV1lvTlZBo="
            crossorigin=""{}
    }
}
