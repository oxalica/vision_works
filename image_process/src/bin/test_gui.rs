use failure::{format_err, Error};
use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use glib::value::Value;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder};

const GLADE_SRC_PATH: &str = "glade/test_gui.glade";
const IMAGE_MAX_SIZE: i32 = 600;

fn main() {
    let app =
        Application::new(None, Default::default()).expect("Failed to initialize GTK application");
    app.connect_activate(|app| {
        let src = std::fs::read_to_string(GLADE_SRC_PATH).expect("Failed to read glade file");
        let builder = Builder::new_from_string(&src);
        let window: ApplicationWindow = builder.get_object("wnd_main").expect("Missing wnd_main");
        window.set_application(Some(app));
        builder
            .connect_signals(move |builder, handler_name| resolve_handler(builder, handler_name));
        window.show_all();
    });
    app.run(&std::env::args().collect::<Vec<_>>());
}

fn resolve_handler(
    builder: &Builder,
    handler_name: &str,
) -> Box<dyn Fn(&[Value]) -> Option<Value> + 'static> {
    let builder = builder.clone();
    match handler_name {
        "on_select_image" => Box::new(move |_| {
            let file_img: gtk::FileChooser = builder.get_object("file_img").unwrap();
            if let Some(file_name) = file_img.get_filename() {
                match load_image(&file_name) {
                    Ok(pixbuf) => {
                        let img: gtk::Image = builder.get_object("img").unwrap();
                        img.set_from_pixbuf(Some(&pixbuf));
                    }
                    Err(err) => {
                        eprintln!("Load failed: {}", err);
                    }
                }
            }
            None
        }),
        _ => unreachable!("Unknow handle_name: {}", handler_name),
    }
}

fn load_image(path: &std::path::Path) -> Result<Pixbuf, Error> {
    let buf = Pixbuf::new_from_file(&path)?;
    let (w, h) = (buf.get_width(), buf.get_height());
    let (w2, h2) = if w <= h && IMAGE_MAX_SIZE < h {
        let w2 = (w as f64 / h as f64 * IMAGE_MAX_SIZE as f64) as i32;
        (w2, IMAGE_MAX_SIZE)
    } else if h <= w && IMAGE_MAX_SIZE < w {
        let h2 = (h as f64 / w as f64 * IMAGE_MAX_SIZE as f64) as i32;
        (IMAGE_MAX_SIZE, h2)
    } else {
        (w, h)
    };

    buf.scale_simple(w2, h2, gdk_pixbuf::InterpType::Tiles)
        .ok_or_else(|| format_err!("Scale failed"))
}
