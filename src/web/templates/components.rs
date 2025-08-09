use maud::{html, Markup};

pub fn header_section() -> Markup {
    html! {
        div.center {
            span.heading {
                "Control of Weapons Act Notices"
            }
            span.subheading {
                "Gazettes sourced from the Victorian Gazette website"
            }
        }
    }
}

pub fn notice_section() -> Markup {
    html! {
        ul #notice {
            span {
                li.notice {
                    "Entries are refreshing server-side in the background - if you have Javascript disabled (this is smart!), you'll need to refresh this page to see latest entries. Otherwise this message will clear when refreshing has completed."
                }
            }
        }
    }
}

pub fn map_section() -> Markup {
    html! {
        div #map {}
    }
}

pub fn list_section(initial_list: Markup) -> Markup {
    html! {
        ul #list {
            (initial_list)
        }
    }
}

pub fn footer_section() -> Markup {
    html! {
        a.attribution href="https://github.com/benbeshara/Carcinised-Gazette-Scraper" target="_blank" {
            "Source available here under the permissive AGPL-3.0 license"
        }
    }
}
