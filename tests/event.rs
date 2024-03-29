webio::run_tests_in_browser! {}

use webio::{event::EventType, EventType};

macro_rules! make_event {
    (
        $fn_name:ident,
        $elem_name:expr,
        $evt_name:ident,
        $create_evt:expr $(,)?
    ) => {
        #[webio::test]
        async fn $fn_name() {
            let element = TempElement::create($elem_name);
            let listener =
                webio::event::$evt_name.add_listener(&element.js_object);
            element.js_object.dispatch_event(&$create_evt).unwrap();
            listener.listen_next().await.unwrap();
            element.js_object.dispatch_event(&$create_evt).unwrap();
            listener.listen_next().await.unwrap();
            element.js_object.dispatch_event(&$create_evt).unwrap();
            listener.listen_next().await.unwrap();
        }
    };
}

fn load_window() -> web_sys::Window {
    web_sys::window().expect("test only in browser")
}

fn load_document() -> web_sys::Document {
    load_window().document().unwrap()
}

#[derive(Debug)]
struct TempElement {
    js_object: web_sys::Element,
}

impl TempElement {
    pub fn create(name: &str) -> Self {
        let document = load_document();
        let element = document.create_element(name).unwrap();
        document.body().unwrap().append_child(&element).unwrap();
        Self { js_object: element }
    }
}

impl Drop for TempElement {
    fn drop(&mut self) {
        load_document().body().unwrap().remove_child(&self.js_object).unwrap();
    }
}

make_event! {
    keyup,
    "input",
    KeyUp,
    web_sys::KeyboardEvent::new("keyup").unwrap(),
}

make_event! {
    keydown,
    "input",
    KeyDown,
    web_sys::KeyboardEvent::new("keydown").unwrap(),
}

make_event! {
    click,
    "button",
    Click,
    web_sys::MouseEvent::new("click").unwrap(),
}

make_event! {
    mousedown,
    "button",
    MouseDown,
    web_sys::MouseEvent::new("mousedown").unwrap(),
}

make_event! {
    mouseup,
    "button",
    MouseUp,
    web_sys::MouseEvent::new("mouseup").unwrap(),
}

make_event! {
    mouseenter,
    "p",
    MouseEnter,
    web_sys::MouseEvent::new("mouseenter").unwrap(),
}

make_event! {
    mouseleave,
    "p",
    MouseLeave,
    web_sys::MouseEvent::new("mouseleave").unwrap(),
}

make_event! {
    mousemove,
    "p",
    MouseMove,
    web_sys::MouseEvent::new("mousemove").unwrap(),
}

make_event! {
    mouseover,
    "p",
    MouseOver,
    web_sys::MouseEvent::new("mouseover").unwrap(),
}

make_event! {
    mouseout,
    "p",
    MouseOut,
    web_sys::MouseEvent::new("mouseout").unwrap(),
}

make_event! {
    drag,
    "p",
    Drag,
    web_sys::DragEvent::new("drag").unwrap(),
}

make_event! {
    dragstart,
    "p",
    DragStart,
    web_sys::DragEvent::new("dragstart").unwrap(),
}

make_event! {
    dragend,
    "p",
    DragEnd,
    web_sys::DragEvent::new("dragend").unwrap(),
}

make_event! {
    dragenter,
    "p",
    DragEnter,
    web_sys::DragEvent::new("dragenter").unwrap(),
}

make_event! {
    dragleave,
    "p",
    DragLeave,
    web_sys::DragEvent::new("dragleave").unwrap(),
}

make_event! {
    dragover,
    "p",
    DragOver,
    web_sys::DragEvent::new("dragover").unwrap(),
}

make_event! {
    drop,
    "p",
    DragDrop,
    web_sys::DragEvent::new("drop").unwrap(),
}

make_event! {
    touchstart,
    "p",
    TouchStart,
    web_sys::Event::new("touchstart").unwrap(),
}

make_event! {
    touchend,
    "p",
    TouchEnd,
    web_sys::Event::new("touchend").unwrap(),
}

make_event! {
    touchmove,
    "p",
    TouchMove,
    web_sys::Event::new("touchmove").unwrap(),
}

make_event! {
    touchcancel,
    "p",
    TouchCancel,
    web_sys::Event::new("touchcancel").unwrap(),
}

make_event! {
    blur,
    "p",
    Blur,
    web_sys::FocusEvent::new("blur").unwrap(),
}

make_event! {
    focus,
    "p",
    Focus,
    web_sys::FocusEvent::new("focus").unwrap(),
}

make_event! {
    focusout,
    "p",
    FocusOut,
    web_sys::FocusEvent::new("focusout").unwrap(),
}

make_event! {
    focusin,
    "p",
    FocusIn,
    web_sys::FocusEvent::new("focusin").unwrap(),
}

#[webio::test]
async fn window_resize() {
    let element = load_window();
    let listener = webio::event::WindowResize.add_listener(&element);
    element.dispatch_event(&web_sys::UiEvent::new("resize").unwrap()).unwrap();
    listener.listen_next().await.unwrap();
    element.dispatch_event(&web_sys::UiEvent::new("resize").unwrap()).unwrap();
    listener.listen_next().await.unwrap();
    element.dispatch_event(&web_sys::UiEvent::new("resize").unwrap()).unwrap();
    listener.listen_next().await.unwrap();
}

#[derive(Debug, Clone, Copy, EventType)]
#[event_type(name = "click", data = web_sys::MouseEvent)]
struct CustomClick;

#[webio::test]
async fn custom_click() {
    let element = TempElement::create("button");
    let listener = CustomClick.add_listener(&element.js_object);
    element
        .js_object
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    listener.listen_next().await.unwrap();
    element
        .js_object
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    listener.listen_next().await.unwrap();
}
