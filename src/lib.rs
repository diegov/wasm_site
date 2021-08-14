extern crate wee_alloc;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use anyhow::Error;
use serde::Deserialize;
use std::time::Duration;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlStyleElement;
use web_sys::{Document, Element, MouseEvent};
use yew::format::{Json, Nothing};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::{Task, TimeoutService};
use yew::{html, Component, ComponentLink, Html, ShouldRender};
mod canvas;
mod debug;
mod time;
mod urltools;

const REMOVE_TIMEOUT_MS: u64 = 400;
const DEFAULT_WS: &str = " ";

pub struct Model {
    link: ComponentLink<Self>,
    req: Option<FetchTask>,
    user_info: Option<UserInfo>,
    task: Option<Box<dyn Task>>,
    counter: u128,
    show_aside: bool,
    cursor: (i32, i32),
}

pub enum Msg {
    Fetch,
    UserInfo(UserInfoResponse),
    Remove(usize),
    HideAside,
    Cleanup,
    Ignore,
    MouseMove(MouseEvent),
}

#[derive(Deserialize)]
pub struct Site {
    url: String,
    me: bool,
}

#[derive(Deserialize)]
pub struct UserInfoResponse {
    name: String,
    sites: Vec<Site>,
    source: String,
}

pub struct UserInfo {
    name: String,
    sites: Vec<(Site, bool, f64)>,
    source: String,
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        Model {
            link,
            req: None,
            user_info: None,
            task: None,
            counter: 0,
            show_aside: true,
            cursor: (0, 0),
        }
    }

    fn rendered(&mut self, first_render: bool) {
        if first_render {
            self.link.send_message(Msg::Fetch);
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        self.counter += 1;
        match msg {
            Msg::Fetch => {
                self.req = Some(self.get_user_info());
                false
            }
            Msg::Cleanup => self.cleanup_items(),
            Msg::UserInfo(info_response) => {
                let user_info = map_response(info_response);
                self.update_document(&user_info);
                self.user_info = Some(user_info);
                self.show_aside = true;
                true
            }
            Msg::Remove(i) => {
                if let Some(info) = self.user_info.as_mut() {
                    info.sites[i].1 = true;
                    info.sites[i].2 = time::now();
                }

                let cleanup = self.link.callback(|_| Msg::Cleanup);

                let timeout =
                    TimeoutService::spawn(Duration::from_millis(REMOVE_TIMEOUT_MS + 5), cleanup);
                self.task = Some(Box::new(timeout));

                true
            }
            Msg::HideAside => {
                self.show_aside = false;
                true
            }
            Msg::MouseMove(event) => {
                self.cursor = (event.x(), event.y());
                // TODO: Grab child reference and pass this through the "change" method instead. Unfortunately looking through
                // the Yew codebase shows only a single use of the "change" function, internal to Yew, and nothing in the examples.
                true
            }
            Msg::Ignore => false,
        }
    }

    fn view(&self) -> Html {
        match &self.user_info {
            Some(data) => {
                let name_parts: Vec<&str> = data.name.split(' ').collect();

                let name = html! {
                    <h1>{ &data.name }</h1>
                };

                let sites = if !data.sites.is_empty() {
                    html! {
                        <ul>
                        { data.sites.iter().enumerate().map(|site| self.render_item(site, &name_parts)).collect::<Html>() }
                        </ul>
                    }
                } else {
                    // TODO: This is terrible. I just want to turn a mouse event captured in the main page into a WebGL uniform in the canvas component,
                    // to do that I have to rerender every piece of virtual DOM for every single mouse move event, and somehow trust that the DOM diff
                    // algorithm makes this clear performance atrocity run well.
                    html! {
                        <>
                        <p>{ "There's nothing left!" } { DEFAULT_WS } <button onclick=self.link.callback(move |_| Msg::Fetch) >{ "Reset" }</button></p>
                        <canvas::Model cursor=self.cursor />
                        <p>{ "(This will probably drain your battery, don't leave it running too long...)" } </p>
                        </>
                    }
                };

                let aside_class = if self.show_aside { "" } else { "removed" };

                // We'll track mouse movement to perform some tricks later
                let onmousemove = &self.link.callback(Msg::MouseMove);

                html! {
                    <body onmousemove=onmousemove>
                    <header>
                    { name }
                    </header>
                    <main>
                    { sites }
                    </main>
                    <aside class={aside_class}>
                    <h4>{ "Ugly but functional" } { DEFAULT_WS }<button onclick=self.link.callback(move |_| Msg::HideAside) >{ "Don't care" }</button> </h4>
                    </aside>
                    <footer>
                    { "Best viewed in " } <a href="https://www.mozilla.org/firefox" >{ "Firefox" }</a>
                    { " — " }
                    <a href={data.source.clone()}>{ "Source for this site" }</a>
                    </footer>
                    </body>
                }
            }
            None => html! { <p>{ "Loading..." }</p> },
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        false
    }
}

impl Model {
    fn get_user_info(&self) -> FetchTask {
        let handler = self.link.callback(
            move |response: Response<Json<Result<UserInfoResponse, Error>>>| {
                let (meta, Json(data)) = response.into_parts();
                if meta.status.is_success() {
                    match data {
                        Ok(user_info) => Msg::UserInfo(user_info),
                        Err(info) => {
                            debug::log(&info.to_string());
                            Msg::Ignore
                        }
                    }
                } else {
                    debug::log(&format!(
                        "Unable to get the sites, error: {}",
                        meta.status.as_str()
                    ));
                    Msg::Ignore
                }
            },
        );

        let url = "/sites.json";

        let request = Request::get(url)
            .header("pragma", "no-cache")
            .header("cache-control", "no-cache")
            .body(Nothing)
            .expect("Failed to create request");

        FetchService::fetch(request, handler).expect("Failed to create fetch task")
    }

    fn update_document(&self, info: &UserInfo) {
        let element_count = info.sites.len();
        let doc: Document = web_sys::window()
            .expect("no window available")
            .document()
            .expect("no document available");

        doc.set_title(&info.name);

        let item_time = 0.12;
        let wait_time = 7.0;

        let style = self.build_animation_style(element_count, item_time, wait_time);

        set_style(&doc, &style);
    }

    fn build_animation_style(
        &self,
        element_count: usize,
        item_time: f64,
        wait_time: f64,
    ) -> String {
        let total_item_time = item_time * (element_count as f64);

        let total_animation_time = total_item_time + wait_time;

        let active_ratio = item_time / total_animation_time * 2.0;

        let mut style = String::with_capacity(100 * (element_count + 1));

        // create unique keyframes name to force animation reset
        let animation_name = format!("glow{}", self.counter);

        style.push_str(
            "li:not(.removed) { 
animation: ",
        );
        style.push_str(&animation_name);
        style.push(' ');
        style.push_str(&total_animation_time.to_string());
        style.push_str(
            "s ease-in-out infinite;
}
",
        );

        style.push_str("@keyframes ");
        style.push_str(&animation_name);
        style.push_str(" {
0% {
    text-shadow: 0 0 .05em #330, 0 0 .1em #444, 0 0 .16em #dddd00, 0 0 .22em #bbbb00, 0 0 .31em #ccbb00, 0 0 .40em #bbbb00;
}
");
        style.push_str(&(active_ratio * 100.0).to_string());
        style.push_str(
            "% { text-shadow: none; } 
}

li a {
    text-shadow: none;
}
",
        );

        for i in 0..element_count {
            style.push_str(
                "
            li:not(.removed):nth-child(",
            );
            style.push_str(&element_count.to_string());
            style.push_str("n -");
            style.push_str(&(element_count - i - 1).to_string());
            style.push_str(") { animation-delay: ");
            style.push_str(&(item_time * (i as f64)).to_string());
            style.push_str("s; }\n");
        }

        style
    }

    fn cleanup_items(&mut self) -> ShouldRender {
        let should_render = if let Some(user_info) = self.user_info.as_mut() {
            let curr_time = time::now();

            let to_remove: Vec<usize> = user_info
                .sites
                .iter()
                .enumerate()
                .rev()
                .filter(|(_, (_, removed, time))| {
                    *removed && (curr_time - time) >= (REMOVE_TIMEOUT_MS as f64)
                })
                .map(|(index, _)| index)
                .collect();

            for index in to_remove.iter() {
                user_info.sites.remove(*index);
            }

            !to_remove.is_empty()
        } else {
            false
        };

        if should_render {
            if let Some(user_info) = &self.user_info {
                self.update_document(user_info);
            }
        }

        should_render
    }

    fn render_item(
        &self,
        (idx, (site, is_deleted, _)): (usize, &(Site, bool, f64)),
        name_parts: &[&str],
    ) -> Html {
        let url_string = &site.url;
        let title = urltools::abbreviate_max(url_string, name_parts, Some(30))
            .expect("Can't abbreviate url");

        let css_class = if *is_deleted { "removed" } else { "" };

        let button = html! {
            <button onclick=self.link.callback(move |_| Msg::Remove(idx)) >{ "Don't care" }</button>
        };

        let link = if site.me {
            html! {
                <a href={ url_string.clone() } rel={ "me" }>{ title }</a>
            }
        } else {
            html! {
                <a href={ url_string.clone() }>{ title }</a>
            }
        };

        html! {
            <li class={ css_class }>
            { link } { DEFAULT_WS } { button }
            </li>
        }
    }
}

fn set_style(doc: &Document, style: &str) {
    const ANIMATION_STYLE_ID: &str = "ffd52e56-7607-4b77-a298-4a7e18c27631";
    const STYLE_ID_ATTRIBUTE: &str = "data-program-style";

    if let Some(h) = doc.head() {
        let head: Element = h.into();

        for i in 0..head.children().length() {
            let child = head
                .children()
                .item(i)
                .unwrap_or_else(|| panic!("no child at index {}", i));

            if let Some(style_id) = child.get_attribute(STYLE_ID_ATTRIBUTE) {
                if style_id == ANIMATION_STYLE_ID {
                    if let Ok(derived) = child.dyn_into::<HtmlStyleElement>() {
                        let style_element: Element = derived.into();
                        style_element.set_inner_html(style);
                        return;
                    }
                }
            }
        }

        if let Ok(element) = doc.create_element("style") {
            element
                .set_attribute(STYLE_ID_ATTRIBUTE, ANIMATION_STYLE_ID)
                .expect("can't set attribute");
            element.set_inner_html(style);
            head.append_with_node_1(&element.into())
                .expect("can't append attribute");
        }
    }
}

fn map_response(response: UserInfoResponse) -> UserInfo {
    UserInfo {
        name: response.name,
        sites: response
            .sites
            .into_iter()
            .map(|s| (s, false, -1.0))
            .collect(),
        source: response.source,
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    yew::start_app::<Model>();
}
