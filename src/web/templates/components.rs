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

pub fn update_notice() -> Markup {
    html! {
        span.update {
            "You may notice that not all declared areas as posted on the Victoria Police website have an entry in this list. Victoria Police have recently begun posting notices for areas on their website which are not gazetted. This is concerning because it is in violation of "
            a href="https://content.legislation.vic.gov.au/sites/default/files/2025-08/90-24aa076-authorised.pdf" alt="Current authorised version of the Control of Weapons Act 1990" target="_blank" {
                "section 10D(4)(a) of the Control of Weapons Act 1990"
            }
            ". Victoria Police continue to have a presence and attempt to enforce the act in these areas. This is unlawful. The act can only come into effect during the period specified in the notice after its publication in the Government Gazette (under section 10D(6)). I have submitted FOI requests and questions to the police commissioner but have yet to receive a response."
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

pub fn list_section(initial_list: &Markup) -> Markup {
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
