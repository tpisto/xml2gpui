use gpui::*;

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use xml2gpui_macros::tailwind_to_gpui;

#[derive(Debug)]
pub struct Component {
    pub elem: String,
    pub text: Option<String>,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Component>,
}

pub fn parse_xml(xml: String) -> Component {
    let mut reader = Reader::from_str(xml.as_str());
    reader
        .expand_empty_elements(true)
        .check_end_names(true)
        .trim_text(true);

    let mut buf = Vec::new();
    let mut stack: Vec<Component> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Eof) => break,
            Ok(event) => match event {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    let elem_name = String::from_utf8(e.local_name().as_ref().to_vec()).unwrap();
                    let attributes = e
                        .html_attributes()
                        .map(|a| {
                            if let Ok(a) = a {
                                (
                                    String::from_utf8(a.key.local_name().as_ref().to_vec())
                                        .unwrap(),
                                    a.decode_and_unescape_value(&reader).unwrap().into_owned(),
                                )
                            } else {
                                // println!("Attributes are: {:?}", e.attributes());
                                // panic!("Error reading attribute");
                                ("error".to_string(), "error".to_string())
                            }
                        })
                        .collect::<Vec<(String, String)>>();

                    let component = Component {
                        elem: elem_name,
                        text: None,
                        attributes,
                        children: Vec::new(),
                    };

                    if let Event::Empty(_) = event {
                        // For Event::Empty, add directly to the parent if exists
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(component);
                        }
                    } else {
                        // For Event::Start, push onto the stack for potential nesting
                        stack.push(component);
                    }
                }
                Event::End(_) => {
                    if stack.len() > 1 {
                        let finished_component = stack.pop().unwrap();
                        if let Some(parent) = stack.last_mut() {
                            parent.children.push(finished_component);
                        }
                    }
                }
                Event::Text(e) => {
                    let text = e.unescape().unwrap();
                    if let Some(parent) = stack.last_mut() {
                        parent.text = Some(text.into_owned());
                    }
                }
                _ => (),
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
        }
        buf.clear();
    }

    stack.pop().unwrap_or_else(|| Component {
        elem: "error".to_string(),
        text: Some("error".to_string()),
        attributes: vec![],
        children: vec![],
    })
}

// I can't use dynamic trait objects, because Styled and IntoElement are not object-safe (have : Sized supertrait)
// https://doc.rust-lang.org/reference/items/traits.html#object-safety
// Sized must not be a supertrait. In other words, it must not require Self: Sized.
pub enum ComponentType {
    Div(Div),
    Img(Img),
    Svg(Svg),
}

pub fn render_component(component: &Component) -> ComponentType {
    let element = match component.elem.as_str() {
        "div" => {
            let mut element = div();

            // Recursively render children and add them
            if !component.children.is_empty() {
                let children_elements = component.children.iter().map(render_component);
                for child in children_elements {
                    match child {
                        ComponentType::Div(div) => element = element.child(div),
                        ComponentType::Img(img) => element = element.child(img),
                        ComponentType::Svg(svg) => element = element.child(svg),
                    }
                }
            }

            // Add text if exists
            if let Some(text) = &component.text {
                element = element.child(text.clone());
            }

            let element = set_attributes::<Div>(element, &component.attributes);
            ComponentType::Div(element)
        }
        "img" => {
            // Get attribute "src"
            let src = component
                .attributes
                .iter()
                .find(|(k, _)| k == "src")
                .map(|(_, v)| v.clone());

            if let Some(src) = src {
                let mut element = img(src);
                element = set_attributes::<Img>(element, &component.attributes);
                ComponentType::Img(element)
            } else {
                ComponentType::Div(div().child("Error: img element must have src attribute"))
            }
        }
        "svg" => {
            // Get attribute "src"
            let path = component
                .attributes
                .iter()
                .find(|(k, _)| k == "path")
                .map(|(_, v)| v.clone());

            if let Some(path) = path {
                let mut element = svg().path(path);
                element = set_attributes::<Svg>(element, &component.attributes);
                ComponentType::Svg(element)
            } else {
                ComponentType::Div(div().child("Error: img element must have src attribute"))
            }
        }
        _ => ComponentType::Div(div()),
    };

    element
}

// Convert #RRGGBB to rgb(0x000000) format where 0x000000 is the hex value of the color in integer
// rgb is function call to convert hex to rgb
fn hex_to_rgba(hex: &str) -> Rgba {
    let hex = hex.trim_start_matches('#');
    let r = u32::from_str_radix(&hex[0..2], 16).unwrap();
    let g = u32::from_str_radix(&hex[2..4], 16).unwrap();
    let b = u32::from_str_radix(&hex[4..6], 16).unwrap();
    // Get also the alpha channel if it exists
    let a = if hex.len() == 8 {
        u32::from_str_radix(&hex[6..8], 16).unwrap()
    } else {
        255
    };
    // u32 is the hex value of the color with alpha
    let value = (r << 24) | (g << 16) | (b << 8) | a;
    rgba(value)
}

fn set_attributes<T: Styled>(mut element: T, attributes: &Vec<(String, String)>) -> T {
    // Font attribute
    if let Some(font_attr_value) = attributes.iter().find(|(k, _)| k == "font").map(|(_, v)| v) {
        let font: SharedString = SharedString::from(font_attr_value.clone());
        element = element.font(font);
    }
    // Class attribute
    if let Some(class_attr_value) = attributes
        .iter()
        .find(|(k, _)| k == "class")
        .map(|(_, v)| v)
    {
        // Split the class attribute into individual classes
        let classes = class_attr_value.split_whitespace();

        // Iterate over classes with a loop to allow mutable access to `element`
        for class_name in classes {
            // Macro magick to convert tailwind classes to gpui. Creates "match class_name { "class-name" => element.class_name() }"
            element = tailwind_to_gpui!(element, class_name,
                // Flex
                [ "flex", "flex-grow", "flex-shrink", "flex-shrink-0" ],
                // Flex wrap
                [ "flex-wrap", "flex-wrap-reverse", "flex-nowrap" ],
                // Align content
                [ "content-normal", "content-center", "content-start", "content-end", "content-between", "content-around", "content-evenly", "content-stretch" ],
                // Flex general
                [ "block", "absolute", "relative", "visible", "invisible", "overflow-hidden", "overflow-x-hidden", "overflow-y-hidden" ],
                // Align
                [ "items-start", "items-end", "items-center" ],
                // Top
                [ "top-0", "top-1", "top-2", "top-3", "top-4", "top-5", "top-6", "top-8", "top-10", "top-12", "top-16", "top-20", "top-24", "top-32", "top-40", "top-48", "top-56", "top-64", "top-72", "top-80", "top-96", "top-auto", "top-full", "top-1/2", "top-1/3", "top-2/3", "top-1/4", "top-2/4", "top-3/4", "top-1/5", "top-2/5", "top-3/5" ],
                // Right
                [ "right-0", "right-1", "right-2", "right-3", "right-4", "right-5", "right-6", "right-8", "right-10", "right-12", "right-16", "right-20", "right-24", "right-32", "right-40", "right-48", "right-56", "right-64", "right-72", "right-80", "right-96", "right-auto", "right-full", "right-1/2", "right-1/3", "right-2/3", "right-1/4", "right-2/4", "right-3/4", "right-1/5", "right-2/5", "right-3/5" ],
                // Bottom
                [ "bottom-0", "bottom-1", "bottom-2", "bottom-3", "bottom-4", "bottom-5", "bottom-6", "bottom-8", "bottom-10", "bottom-12", "bottom-16", "bottom-20", "bottom-24", "bottom-32", "bottom-40", "bottom-48", "bottom-56", "bottom-64", "bottom-72", "bottom-80", "bottom-96", "bottom-auto", "bottom-full", "bottom-1/2", "bottom-1/3", "bottom-2/3", "bottom-1/4", "bottom-2/4", "bottom-3/4", "bottom-1/5", "bottom-2/5", "bottom-3/5" ],
                // Left
                [ "left-0", "left-1", "left-2", "left-3", "left-4", "left-5", "left-6", "left-8", "left-10", "left-12", "left-16", "left-20", "left-24", "left-32", "left-40", "left-48", "left-56", "left-64", "left-72", "left-80", "left-96", "left-auto", "left-full", "left-1/2", "left-1/3", "left-2/3", "left-1/4", "left-2/4", "left-3/4", "left-1/5", "left-2/5", "left-3/5" ],
                // Cursor
                [ "cursor-default", "cursor-pointer", "cursor-text", "cursor-move", "cursor-not-allowed", "cursor-context-menu", "cursor-crosshair", "cursor-vertical-text", "cursor-alias", "cursor-copy", "cursor-no-drop", "cursor-grab", "cursor-grabbing", "cursor-col-resize", "cursor-row-resize", "cursor-n-resize", "cursor-e-resize", "cursor-s-resize", "cursor-w-resize" ],
                // Justify
                [ "justify-center", "justify-between", "justify-around", "justify-start", "justify-end" ],
                // Flex
                [ "flex-col", "flex-row", "flex-col_reverse", "flex-row_reverse", "flex-1", "flex-auto", "flex-initial", "flex-none" ],
                // Shadow
                [ "shadow-sm", "shadow-md", "shadow-lg", "shadow-xl", "shadow-2xl" ],
                // Height
                [ "h-0", "h-1", "h-2", "h-3", "h-4", "h-5", "h-6", "h-8", "h-10", "h-12", "h-16", "h-20", "h-24", "h-32", "h-40", "h-48", "h-56", "h-64", "h-72", "h-80", "h-96", "h-auto", "h-full", "h-1/2", "h-1/3", "h-2/3", "h-1/4", "h-2/4", "h-3/4", "h-1/5", "h-2/5", "h-3/5", "h-4/5", "h-1/6", "h-5/6", "h-1/12" ],
                // Width
                [ "w-0", "w-1", "w-2", "w-3", "w-4", "w-5", "w-6", "w-8", "w-10", "w-12", "w-16", "w-20", "w-24", "w-32", "w-40", "w-48", "w-56", "w-64", "w-72", "w-80", "w-96", "w-auto", "w-full", "w-1/2", "w-1/3", "w-2/3", "w-1/4", "w-2/4", "w-3/4", "w-1/5", "w-2/5", "w-3/5", "w-4/5", "w-1/6", "w-5/6", "w-1/12" ],
                // Max and min height and width
                [ "min-h-0", "min-h-full", "min-w-0", "min-w-full", "max-h-0", "max-h-full", "max-w-0", "max-w-full" ],
                // Padding
                [ "p-0", "p-1", "p-2", "p-3", "p-4", "p-5", "p-6", "p-8", "p-10", "p-12", "p-16", "p-20", "p-24", "p-32", "p-40", "p-48", "p-56", "p-64", "p-72", "p-80", "p-96", "p-full", "p-1/2", "p-1/3", "p-2/3", "p-1/4", "p-2/4", "p-3/4", "p-1/5", "p-2/5", "p-3/5", "p-4/5", "p-1/6", "p-5/6", "p-1/12" ],
                [ "px-0", "px-1", "px-2", "px-3", "px-4", "px-5", "px-6", "px-8", "px-10", "px-12", "px-16", "px-20", "px-24", "px-32", "px-40", "px-48", "px-56", "px-64", "px-72", "px-80", "px-96", "px-full", "px-1/2", "px-1/3", "px-2/3", "px-1/4", "px-2/4", "px-3/4", "px-1/5", "px-2/5", "px-3/5", "px-4/5", "px-1/6", "px-5/6", "px-1/12" ],
                [ "py-0", "py-1", "py-2", "py-3", "py-4", "py-5", "py-6", "py-8", "py-10", "py-12", "py-16", "py-20", "py-24", "py-32", "py-40", "py-48", "py-56", "py-64", "py-72", "py-80", "py-96", "py-full", "py-1/2", "py-1/3", "py-2/3", "py-1/4", "py-2/4", "py-3/4", "py-1/5", "py-2/5", "py-3/5", "py-4/5", "py-1/6", "py-5/6", "py-1/12" ],
                [ "pt-0", "pt-1", "pt-2", "pt-3", "pt-4", "pt-5", "pt-6", "pt-8", "pt-10", "pt-12", "pt-16", "pt-20", "pt-24", "pt-32", "pt-40", "pt-48", "pt-56", "pt-64", "pt-72", "pt-80", "pt-96", "pt-full", "pt-1/2", "pt-1/3", "pt-2/3", "pt-1/4", "pt-2/4", "pt-3/4", "pt-1/5", "pt-2/5", "pt-3/5", "pt-4/5", "pt-1/6", "pt-5/6", "pt-1/12" ],
                [ "pr-0", "pr-1", "pr-2", "pr-3", "pr-4", "pr-5", "pr-6", "pr-8", "pr-10", "pr-12", "pr-16", "pr-20", "pr-24", "pr-32", "pr-40", "pr-48", "pr-56", "pr-64", "pr-72", "pr-80", "pr-96", "pr-full", "pr-1/2", "pr-1/3", "pr-2/3", "pr-1/4", "pr-2/4", "pr-3/4", "pr-1/5", "pr-2/5", "pr-3/5", "pr-4/5", "pr-1/6", "pr-5/6", "pr-1/12" ],
                [ "pb-0", "pb-1", "pb-2", "pb-3", "pb-4", "pb-5", "pb-6", "pb-8", "pb-10", "pb-12", "pb-16", "pb-20", "pb-24", "pb-32", "pb-40", "pb-48", "pb-56", "pb-64", "pb-72", "pb-80", "pb-96", "pb-full", "pb-1/2", "pb-1/3", "pb-2/3", "pb-1/4", "pb-2/4", "pb-3/4", "pb-1/5", "pb-2/5", "pb-3/5", "pb-4/5", "pb-1/6", "pb-5/6", "pb-1/12" ],
                [ "pl-0", "pl-1", "pl-2", "pl-3", "pl-4", "pl-5", "pl-6", "pl-8", "pl-10", "pl-12", "pl-16", "pl-20", "pl-24", "pl-32", "pl-40", "pl-48", "pl-56", "pl-64", "pl-72", "pl-80", "pl-96", "pl-full", "pl-1/2", "pl-1/3", "pl-2/3", "pl-1/4", "pl-2/4", "pl-3/4", "pl-1/5", "pl-2/5", "pl-3/5", "pl-4/5", "pl-1/6", "pl-5/6", "pl-1/12" ],
                // Margin
                [ "m-0", "m-1", "m-2", "m-3", "m-4", "m-5", "m-6", "m-8", "m-10", "m-12", "m-16", "m-20", "m-24", "m-32", "m-40", "m-48", "m-56", "m-64", "m-72", "m-80", "m-96", "m-auto", "m-full", "m-1/2", "m-1/3", "m-2/3", "m-1/4", "m-2/4", "m-3/4", "m-1/5", "m-2/5", "m-3/5", "m-4/5", "m-1/6", "m-5/6", "m-1/12" ],
                [ "mx-0", "mx-1", "mx-2", "mx-3", "mx-4", "mx-5", "mx-6", "mx-8", "mx-10", "mx-12", "mx-16", "mx-20", "mx-24", "mx-32", "mx-40", "mx-48", "mx-56", "mx-64", "mx-72", "mx-80", "mx-96", "mx-auto", "mx-full", "mx-1/2", "mx-1/3", "mx-2/3", "mx-1/4", "mx-2/4", "mx-3/4", "mx-1/5", "mx-2/5", "mx-3/5", "mx-4/5", "mx-1/6", "mx-5/6", "mx-1/12" ],
                [ "my-0", "my-1", "my-2", "my-3", "my-4", "my-5", "my-6", "my-8", "my-10", "my-12", "my-16", "my-20", "my-24", "my-32", "my-40", "my-48", "my-56", "my-64", "my-72", "my-80", "my-96", "my-auto", "my-full", "my-1/2", "my-1/3", "my-2/3", "my-1/4", "my-2/4", "my-3/4", "my-1/5", "my-2/5", "my-3/5", "my-4/5", "my-1/6", "my-5/6", "my-1/12" ],
                [ "mt-0", "mt-1", "mt-2", "mt-3", "mt-4", "mt-5", "mt-6", "mt-8", "mt-10", "mt-12", "mt-16", "mt-20", "mt-24", "mt-32", "mt-40", "mt-48", "mt-56", "mt-64", "mt-72", "mt-80", "mt-96", "mt-auto", "mt-full", "mt-1/2", "mt-1/3", "mt-2/3", "mt-1/4", "mt-2/4", "mt-3/4", "mt-1/5", "mt-2/5", "mt-3/5", "mt-4/5", "mt-1/6", "mt-5/6", "mt-1/12" ],
                [ "mr-0", "mr-1", "mr-2", "mr-3", "mr-4", "mr-5", "mr-6", "mr-8", "mr-10", "mr-12", "mr-16", "mr-20", "mr-24", "mr-32", "mr-40", "mr-48", "mr-56", "mr-64", "mr-72", "mr-80", "mr-96", "mr-auto", "mr-full", "mr-1/2", "mr-1/3", "mr-2/3", "mr-1/4", "mr-2/4", "mr-3/4", "mr-1/5", "mr-2/5", "mr-3/5", "mr-4/5", "mr-1/6", "mr-5/6", "mr-1/12" ],
                [ "mb-0", "mb-1", "mb-2", "mb-3", "mb-4", "mb-5", "mb-6", "mb-8", "mb-10", "mb-12", "mb-16", "mb-20", "mb-24", "mb-32", "mb-40", "mb-48", "mb-56", "mb-64", "mb-72", "mb-80", "mb-96", "mb-auto", "mb-full", "mb-1/2", "mb-1/3", "mb-2/3", "mb-1/4", "mb-2/4", "mb-3/4", "mb-1/5", "mb-2/5", "mb-3/5", "mb-4/5", "mb-1/6", "mb-5/6", "mb-1/12" ],
                [ "ml-0", "ml-1", "ml-2", "ml-3", "ml-4", "ml-5", "ml-6", "ml-8", "ml-10", "ml-12", "ml-16", "ml-20", "ml-24", "ml-32", "ml-40", "ml-48", "ml-56", "ml-64", "ml-72", "ml-80", "ml-96", "ml-auto", "ml-full", "ml-1/2", "ml-1/3", "ml-2/3", "ml-1/4", "ml-2/4", "ml-3/4", "ml-1/5", "ml-2/5", "ml-3/5", "ml-4/5", "ml-1/6", "ml-5/6", "ml-1/12" ],
                // Border
                [ "border", "border-0", "border-1", "border-2", "border-3", "border-4", "border-5", "border-6", "border-8", "border-10", "border-12", "border-16", "border-20", "border-24", "border-32" ],
                // Border width
                [ "border-t", "border-t-0", "border-t-1", "border-t-2", "border-t-3", "border-t-4", "border-t-5", "border-t-6", "border-t-8", "border-t-10", "border-t-12", "border-t-16", "border-t-20", "border-t-24", "border-t-32" ],
                [ "border-r", "border-r-0", "border-r-1", "border-r-2", "border-r-3", "border-r-4", "border-r-5", "border-r-6", "border-r-8", "border-r-10", "border-r-12", "border-r-16", "border-r-20", "border-r-24", "border-r-32" ],
                [ "border-b", "border-b-0", "border-b-1", "border-b-2", "border-b-3", "border-b-4", "border-b-5", "border-b-6", "border-b-8", "border-b-10", "border-b-12", "border-b-16", "border-b-20", "border-b-24", "border-b-32" ],
                [ "border-l", "border-l-0", "border-l-1", "border-l-2", "border-l-3", "border-l-4", "border-l-5", "border-l-6", "border-l-8", "border-l-10", "border-l-12", "border-l-16", "border-l-20", "border-l-24", "border-l-32" ],
                // Border radius
                [ "rounded-none", "rounded-sm", "rounded-md", "rounded-lg", "rounded-xl", "rounded-2xl", "rounded-3xl", "rounded-full" ],
                [ "rounded-t-none", "rounded-t-sm", "rounded-t-md", "rounded-t-lg", "rounded-t-xl", "rounded-t-2xl", "rounded-t-3xl", "rounded-t-full" ],
                [ "rounded-r-none", "rounded-r-sm", "rounded-r-md", "rounded-r-lg", "rounded-r-xl", "rounded-r-2xl", "rounded-r-3xl", "rounded-r-full" ],
                [ "rounded-b-none", "rounded-b-sm", "rounded-b-md", "rounded-b-lg", "rounded-b-xl", "rounded-b-2xl", "rounded-b-3xl", "rounded-b-full" ],
                [ "rounded-l-none", "rounded-l-sm", "rounded-l-md", "rounded-l-lg", "rounded-l-xl", "rounded-l-2xl", "rounded-l-3xl", "rounded-l-full" ],
                [ "rounded-tl-none", "rounded-tl-sm", "rounded-tl-md", "rounded-tl-lg", "rounded-tl-xl", "rounded-tl-2xl", "rounded-tl-3xl", "rounded-tl-full" ],
                [ "rounded-tr-none", "rounded-tr-sm", "rounded-tr-md", "rounded-tr-lg", "rounded-tr-xl", "rounded-tr-2xl", "rounded-tr-3xl", "rounded-tr-full" ],
                [ "rounded-br-none", "rounded-br-sm", "rounded-br-md", "rounded-br-lg", "rounded-br-xl", "rounded-br-2xl", "rounded-br-3xl", "rounded-br-full" ],
                [ "rounded-bl-none", "rounded-bl-sm", "rounded-bl-md", "rounded-bl-lg", "rounded-bl-xl", "rounded-bl-2xl", "rounded-bl-3xl", "rounded-bl-full" ],
                // Font
                [ "font-thin", "font-extralight", "font-light", "font-normal", "font-medium", "font-semibold", "font-bold", "font-extrabold", "font-black" ],
                // Text
                [ "text-xs", "text-sm", "text-base", "text-lg", "text-xl", "text-2xl", "text-3xl" ],
                // Sizes
                [ "size-0", "size-0.5", "size-1", "size-1.5", "size-2", "size-2.5", "size-3", "size-3.5", "size-4", "size-5", "size-6", "size-8", "size-10", "size-12", "size-16", "size-20", "size-24", "size-32", "size-40", "size-48", "size-56", "size-64", "size-72", "size-80", "size-96", "size-1/2", "size-1/3", "size-2/3", "size-1/4", "size-2/4", "size-3/4", "size-1/5", "size-2/5", "size-3/5", "size-4/5", "size-1/6", "size-5/6", "size-1/12", "size-full", "size-auto" ],

                // Dynamic sizes and colors
                _ => {
                    // Handle dynamic background colors
                    if class_name.starts_with("bg-[#") {
                        let hex = &class_name["bg-[#".len()..class_name.len() - 1];
                        let color = hex_to_rgba(hex);
                        element.bg(color)
                    }
                    // Handle dynamic text colors
                    else if class_name.starts_with("text-color-[#") {
                        let hex = &class_name["text-color-[#".len()..class_name.len() - 1];
                        let color = hex_to_rgba(hex);
                        element.text_color(color)
                    }
                    // Handle dynamic border colors
                    else if class_name.starts_with("border-[#") {
                        let hex = &class_name["border-[#".len()..class_name.len() - 1];
                        let color = hex_to_rgba(hex);
                        element.border_color(color)
                    }
                    // Rounded with any px or rem value
                    else if let Some(suffix) = class_name.strip_prefix("rounded-") {
                        let absolute_length = extract_length_from_class_name(suffix);

                        match suffix.split('-').next() {
                            Some("t") => element.rounded_t(absolute_length),
                            Some("r") => element.rounded_r(absolute_length),
                            Some("b") => element.rounded_b(absolute_length),
                            Some("l") => element.rounded_l(absolute_length),
                            Some("tl") => element.rounded_tl(absolute_length),
                            Some("tr") => element.rounded_tr(absolute_length),
                            Some("br") => element.rounded_br(absolute_length),
                            Some("bl") => element.rounded_bl(absolute_length),
                            _ => element.rounded(absolute_length), // Default to applying rounding to all corners
                        }
                    }
                    // Border with any px or rem value
                    else if let Some(suffix) = class_name.strip_prefix("border-") {
                        let absolute_length = extract_length_from_class_name(suffix);
                        match suffix.split('-').next() {
                            Some("t") => element.border_t_width(absolute_length),
                            Some("r") => element.border_r_width(absolute_length),
                            Some("b") => element.border_b_width(absolute_length),
                            Some("l") => element.border_l_width(absolute_length),
                            _ => element.rounded(absolute_length), // Default to applying rounding to all corners
                        }
                    }
                    else {
                        println!("Unrecognized class: {}", class_name);
                        element
                    }
                }
            );
        }
    }

    element
}

// Extracts the numeric value and unit from the class name, returning an AbsoluteLength
fn extract_length_from_class_name(class_name: &str) -> AbsoluteLength {
    let numeric_part: String = class_name
        .chars()
        .skip_while(|c| !c.is_digit(10) && *c != '.')
        .take_while(|c| c.is_digit(10) || *c == '.')
        .collect();

    let unit_part: String = class_name
        .chars()
        .skip_while(|c| c.is_digit(10) || *c == '.')
        .collect();

    let rounded_value = numeric_part.parse::<f32>().unwrap_or_default();

    match unit_part.as_str() {
        "px" => AbsoluteLength::Pixels(px(rounded_value)),
        "rem" => AbsoluteLength::Rems(rems(rounded_value)),
        _ => AbsoluteLength::Pixels(px(0.0)), // Default case for unrecognized units
    }
}
