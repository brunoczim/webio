webio::run_tests_in_browser! {}

use webio::event::Type;

fn load_document() -> web_sys::Document {
    web_sys::window().expect("test only in browser").document().unwrap()
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

#[webio::test]
async fn onclick() {
    let element = TempElement::create("button");
    let mut count = 0;
    let listener =
        webio::event::Click.add_sync_listener(&element.js_object, move |_| {
            println!("Event called");
            assert!(true);
            let event_id = count;
            count += 1;
            event_id
        });
    element
        .js_object
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    assert_eq!(listener.listen_next().await.unwrap(), 0);
    element
        .js_object
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    assert_eq!(listener.listen_next().await.unwrap(), 1);
    element
        .js_object
        .dispatch_event(&web_sys::MouseEvent::new("click").unwrap())
        .unwrap();
    assert_eq!(listener.listen_next().await.unwrap(), 2);
}
