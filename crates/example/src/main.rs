use gpui::*;

mod hello;

use hello::HelloWorld;

fn main() {
    App::new().run(|cx: &mut AppContext| {
        cx.open_window(WindowOptions::default(), |cx| {
            // Root view
            HelloWorld::new(cx)
        });
    });
}
