use failure::{Error, ResultExt as _};
use gio::prelude::*;
use glib::value::Value;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder};
use once_cell::sync::OnceCell;
use opencv::prelude::*;
use std::sync::{Arc, Mutex};

mod ext;
use ext::{BuilderExtManualExt as _, OptionExt as _};

type Result<T> = std::result::Result<T, Error>;

const GLADE_SRC_PATH: &str = "glade/img_process.glade";
static GUI_EVENT_TX: OnceCell<glib::Sender<GuiEvent>> = OnceCell::new();

#[derive(Debug)]
enum GuiEvent {
    Log(String),
}

#[derive(Debug, Default)]
struct GuiState {
    image_input: Option<Mat>,
    image_output: Option<Mat>,
}

fn main() {
    let app =
        Application::new(None, Default::default()).expect("Failed to initialize GTK application");
    app.connect_activate(|app| {
        let src = std::fs::read_to_string(GLADE_SRC_PATH).expect("Failed to read glade file");
        let builder = Builder::new_from_string(&src);
        let window: ApplicationWindow = builder.object("wnd_main");
        window.set_application(Some(app));

        let state: Arc<Mutex<GuiState>> = Default::default();
        builder.connect_signals(|builder, handler_name| {
            resolve_handler(&builder, &state, handler_name)
        });

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        GUI_EVENT_TX.set(tx).expect("Initialize more than once");
        let builder = builder.clone();
        rx.attach(None, move |event| {
            on_gui_event(&builder, event);
            glib::Continue(true)
        });
        window.show_all();
    });
    app.run(&std::env::args().collect::<Vec<_>>());
}

fn on_gui_event(builder: &Builder, event: GuiEvent) {
    match event {
        GuiEvent::Log(content) => {
            // https://mail.gnome.org/archives/gtk-list/2007-May/msg00034.html
            let txt: gtk::TextView = builder.object("txt_log");
            let buf = txt.get_buffer().unwrap();
            let mark = buf.get_insert().unwrap();
            let iter = buf.get_end_iter();
            buf.move_mark(&mark, &iter);
            buf.insert_at_cursor(&content);
            txt.scroll_to_mark(&mark, 0.0, false, 0.0, 0.0);
        }
    }
}

fn send_log_event(content: String) {
    GUI_EVENT_TX
        .get()
        .unwrap()
        .send(GuiEvent::Log(content))
        .unwrap();
}

macro_rules! log {
    ($fmt:literal $($tt:tt)*) => {
        crate::send_log_event(format!(concat!($fmt, "\n") $($tt)*))
    };
}

fn resolve_handler(
    builder: &Builder,
    state: &Arc<Mutex<GuiState>>,
    handler_name: &str,
) -> Box<dyn Fn(&[Value]) -> Option<Value> + 'static> {
    let builder = builder.clone();
    let state = state.clone();
    match handler_name {
        "on_select_source_file" => Box::new(move |_| {
            on_select_source_file(&builder);
            None
        }),
        "on_reload_input" => Box::new(move |_| {
            on_select_source_file(&builder);
            None
        }),
        "on_clear_log" => Box::new(move |_| {
            let txt_log: gtk::TextView = builder.object("txt_log");
            txt_log.get_buffer().unwrap().set_text("");
            None
        }),
        "on_swap_img" => Box::new(move |_| {
            let st = &mut *state.lock().unwrap();
            std::mem::swap(&mut st.image_input, &mut st.image_output);
            let img1: gtk::Image = builder.object("img_input");
            let img2: gtk::Image = builder.object("img_output");
            let (buf1, buf2) = (img1.get_pixbuf(), img2.get_pixbuf());
            img1.set_from_pixbuf(buf2.as_ref());
            img2.set_from_pixbuf(buf1.as_ref());
            None
        }),
        _ => unreachable!("Unknow handle_name: {}", handler_name),
    }
}

fn on_select_source_file(builder: &Builder) {
    let fin: gtk::FileChooser = builder.object("file_input");
    if let Some(file_name) = fin.get_filename() {
        log!("Loading file {}", file_name.display());
        match (|| -> Result<_> {
            let mat = load_image(&file_name).context("Load image")?;
            let img = builder.object("img_input");
            render_output_image(&img, Some(&mat)).context("Render image")?;
            log!("Loaded {}x{}", mat.rows(), mat.cols());
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) => log!("Error: {}", err),
        }
    }
}

fn load_image(path: &std::path::Path) -> Result<Mat> {
    use opencv::imgcodecs::*;
    let path_str = path.to_str().context("Path is not valid UTF8")?;
    let img = imread(path_str, IMREAD_COLOR).context("imread")?;
    Ok(img)
}

fn render_output_image(img: &gtk::Image, data: Option<&Mat>) -> Result<()> {
    use gdk_pixbuf::{Colorspace, Pixbuf};
    use opencv::core::Vec3b;

    let data = match data {
        Some(data) => data,
        None => {
            img.set_from_pixbuf(None);
            return Ok(());
        }
    };

    let (h, w) = (data.rows(), data.cols());
    let pixbuf = Pixbuf::new(Colorspace::Rgb, false, 8, w, h).context("Pixbuf::new")?;
    for x in 0..h {
        for y in 0..w {
            let [b, g, r] = data.at_2d::<Vec3b>(x, y).context("Read Mat")?.0;
            pixbuf.put_pixel(y, x, r, g, b, 0);
        }
    }

    img.set_from_pixbuf(Some(&pixbuf));
    Ok(())
}
