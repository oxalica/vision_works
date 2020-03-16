use failure::ResultExt as _;
use gio::prelude::*;
use glib::value::Value;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder};
use once_cell::sync::OnceCell;
use std::{cell::RefCell, rc::Rc, sync::Arc};

macro_rules! log {
    ($fmt:literal $($tt:tt)*) => {
        crate::send_log_event(format!(concat!($fmt, "\n") $($tt)*))
    };
}

mod processor;
mod util;
use processor::{load_processors, ImageProcessor};
use util::{BuilderExtManualExt as _, Image};

const GLADE_SRC_PATH: &str = "glade/img_process.glade";
static GUI_EVENT_TX: OnceCell<glib::Sender<GuiEvent>> = OnceCell::new();

#[derive(Debug)]
enum GuiEvent {
    Log(String),
    ImageOutput(Image),
}

#[derive(Debug, Default)]
struct GuiState {
    image_input: Option<Image>,
    image_output: Option<Image>,
}

fn main() {
    let app =
        Application::new(None, Default::default()).expect("Failed to initialize GTK application");
    app.connect_activate(|app| {
        let src = std::fs::read_to_string(GLADE_SRC_PATH).expect("Failed to read glade file");
        let builder = Builder::new_from_string(&src);
        let window: ApplicationWindow = builder.object("wnd_main");
        window.set_application(Some(app));

        let processors = load_processors();

        let state: Rc<RefCell<GuiState>> = Default::default();
        builder.connect_signals(|builder, handler_name| {
            resolve_handler(&builder, &state, &processors, handler_name)
        });

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        GUI_EVENT_TX.set(tx).expect("Initialize more than once");
        let builder = builder.clone();
        let state = state.clone();
        rx.attach(None, move |event| {
            on_gui_event(&builder, &state, event);
            glib::Continue(true)
        });
        window.show_all();
    });
    app.run(&std::env::args().collect::<Vec<_>>());
}

fn on_gui_event(builder: &Builder, state: &Rc<RefCell<GuiState>>, event: GuiEvent) {
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
        GuiEvent::ImageOutput(img) => {
            builder
                .object::<gtk::Image>("img_output")
                .set_from_pixbuf(Some(&img.render()));
            state.borrow_mut().image_output = Some(img);
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

fn resolve_handler(
    builder: &Builder,
    state: &Rc<RefCell<GuiState>>,
    processors: &[Arc<dyn ImageProcessor>],
    handler_name: &str,
) -> Box<dyn Fn(&[Value]) -> Option<Value> + 'static> {
    let builder = builder.clone();
    let state = state.clone();
    match handler_name {
        "on_select_source_file" => Box::new(move |_| {
            on_select_source_file(&builder, &state);
            None
        }),
        "on_reload_input" => Box::new(move |_| {
            on_select_source_file(&builder, &state);
            None
        }),
        "on_clear_log" => Box::new(move |_| {
            let txt_log: gtk::TextView = builder.object("txt_log");
            txt_log.get_buffer().unwrap().set_text("");
            None
        }),
        "on_swap_img" => Box::new(move |_| {
            let st = &mut *state.borrow_mut();
            std::mem::swap(&mut st.image_input, &mut st.image_output);
            let img1: gtk::Image = builder.object("img_input");
            let img2: gtk::Image = builder.object("img_output");
            let (buf1, buf2) = (img1.get_pixbuf(), img2.get_pixbuf());
            img1.set_from_pixbuf(buf2.as_ref());
            img2.set_from_pixbuf(buf1.as_ref());
            None
        }),
        _ => {
            for pro in processors {
                let pro_ = pro.clone();
                let state = state.clone();
                let run = Box::new(move |args| {
                    processor_runner(pro_.clone(), args, state.clone());
                });
                if let Some(h) = pro.register_handler(&builder, handler_name, run) {
                    return Box::new(move |_| {
                        h();
                        None
                    });
                }
            }
            unreachable!("Unhandled event: {}", handler_name)
        }
    }
}

fn processor_runner(
    pro: Arc<dyn ImageProcessor>,
    args: Box<dyn std::any::Any + Send>,
    state: Rc<RefCell<GuiState>>,
) {
    let img = match state.borrow().image_input.clone() {
        Some(img) => img,
        None => {
            log!("Error: No input image");
            return;
        }
    };

    std::thread::spawn(move || {
        log!("Running processor...");
        let t = std::time::Instant::now();
        let ret = pro.run(args, img);
        let ns = t.elapsed().as_nanos();
        match ret {
            Err(err) => log!("Failed: {}", err),
            Ok(ret_img) => {
                GUI_EVENT_TX
                    .get()
                    .unwrap()
                    .send(GuiEvent::ImageOutput(ret_img))
                    .unwrap();
                log!(
                    "Done in {}.{:03} {:03} {:03} s",
                    ns / 1_000_000_000,
                    ns / 1_000_000 % 1_000,
                    ns / 1_000 % 1_000,
                    ns / 1 % 1_000,
                )
            }
        }
    });
}

fn on_select_source_file(builder: &Builder, state: &Rc<RefCell<GuiState>>) {
    let fin: gtk::FileChooser = builder.object("file_input");
    if let Some(file_name) = fin.get_filename() {
        let img_ctl: gtk::Image = builder.object("img_input");

        log!("Loading file {}", file_name.display());
        match Image::open(&file_name).context("Load image") {
            Err(err) => {
                log!("Error: {}", err);
                // Clear input image.
                state.borrow_mut().image_input = None;
                img_ctl.set_from_pixbuf(None);
            }
            Ok((img, pixbuf)) => {
                log!("Loaded {}x{}", pixbuf.get_width(), pixbuf.get_height());
                state.borrow_mut().image_input = Some(img);
                img_ctl.set_from_pixbuf(Some(&pixbuf));
            }
        }
    }
}
