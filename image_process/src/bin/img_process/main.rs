use failure::ResultExt as _;
use gdk_pixbuf::Pixbuf;
use gio::prelude::*;
use glib::value::Value;
use gtk::{prelude::*, Application, ApplicationWindow, Builder};
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
    WorkerError,
}

#[derive(Debug)]
struct GuiState {
    image_input: Option<(Image, Pixbuf)>,
    image_output: Option<(Image, Pixbuf)>,
    processing: bool,
    auto_shrink: bool,
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            image_input: None,
            image_output: None,
            processing: false,
            auto_shrink: true,
        }
    }
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
            let pixbuf = img.render();
            let mut st = state.borrow_mut();
            auto_rerender(
                builder,
                &st,
                &builder.object::<gtk::Image>("img_output"),
                &pixbuf,
            );
            st.image_output = Some((img, pixbuf));
            st.processing = false;
        }
        GuiEvent::WorkerError => {
            state.borrow_mut().processing = false;
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
    let state_ = state.clone();
    let check_processing = move || {
        if state_.borrow().processing {
            log!("Error: Anothing job is running. Please wait.");
            true
        } else {
            false
        }
    };
    match handler_name {
        "on_select_source_file" => Box::new(move |_| {
            if !check_processing() {
                on_select_source_file(&builder, &state);
            }
            None
        }),
        "on_reload_input" => Box::new(move |_| {
            if !check_processing() {
                on_select_source_file(&builder, &state);
            }
            None
        }),
        "on_clear_log" => Box::new(move |_| {
            let txt_log: gtk::TextView = builder.object("txt_log");
            txt_log.get_buffer().unwrap().set_text("");
            None
        }),
        "on_swap_img" => Box::new(move |_| {
            if check_processing() {
                return None;
            }
            let st = &mut *state.borrow_mut();
            std::mem::swap(&mut st.image_input, &mut st.image_output);
            let img1: gtk::Image = builder.object("img_input");
            let img2: gtk::Image = builder.object("img_output");
            let (buf1, buf2) = (img1.get_pixbuf(), img2.get_pixbuf());
            img1.set_from_pixbuf(buf2.as_ref());
            img2.set_from_pixbuf(buf1.as_ref());
            None
        }),
        "on_wnd_resize" => Box::new(move |_| {
            on_resize(&builder, &state.borrow(), false);
            None
        }),
        "on_toggle_auto_shrink" => Box::new(move |_| {
            let mut st = state.borrow_mut();
            st.auto_shrink = !st.auto_shrink;
            on_resize(&builder, &st, true);
            None
        }),
        _ => {
            for pro in processors {
                let builder_ = builder.clone();
                let pro_ = pro.clone();
                let state_ = state.clone();
                let run = Box::new(move |args| {
                    processor_runner(&builder_, &state_, pro_.clone(), args);
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
    builder: &Builder,
    state: &Rc<RefCell<GuiState>>,
    pro: Arc<dyn ImageProcessor>,
    args: Box<dyn std::any::Any + Send>,
) {
    let mut st = state.borrow_mut();
    if st.processing {
        log!("Error: Anothing job is running. Please wait.");
        return;
    }

    // Clear output buffer.
    builder
        .object::<gtk::Image>("img_output")
        .set_from_pixbuf(None);

    let img = match st.image_input.as_ref() {
        Some((img, _)) => img.clone(),
        None => {
            log!("Error: No input image");
            return;
        }
    };

    st.processing = true;
    log!("Running processor...");

    let worker_handle = std::thread::spawn(move || {
        let t = std::time::Instant::now();
        let ret = pro.run(args, img);
        let ns = t.elapsed().as_nanos();
        (ret, ns)
    });

    // Watching dog
    std::thread::spawn(move || {
        match worker_handle.join() {
            Ok((Ok(ret_img), ns)) => {
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
                );
                return;
            }
            Ok((Err(err), _)) => log!("Error: {}", err),
            Err(err) => {
                log!("Error: Worker panicked: {:?}", err);
            }
        }
        GUI_EVENT_TX
            .get()
            .unwrap()
            .send(GuiEvent::WorkerError)
            .unwrap();
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
                let mut st = state.borrow_mut();
                auto_rerender(builder, &st, &img_ctl, &pixbuf);
                st.image_input = Some((img, pixbuf));
            }
        }
    }
}

fn on_resize(builder: &Builder, st: &GuiState, force: bool) {
    if force || st.auto_shrink {
        if let Some((_, pixbuf)) = &st.image_input {
            auto_rerender(&builder, st, &builder.object("img_input"), pixbuf);
        }
        if let Some((_, pixbuf)) = &st.image_output {
            auto_rerender(&builder, st, &builder.object("img_output"), pixbuf);
        }
    }
}

fn auto_rerender(builder: &Builder, st: &GuiState, img_ctl: &gtk::Image, pixbuf: &Pixbuf) {
    let alloc = builder
        .object::<gtk::ScrolledWindow>("scw_img_input")
        .get_allocation();
    let (h, w) = (pixbuf.get_height(), pixbuf.get_width());
    let (mxh, mxw) = (alloc.height - 2, alloc.width - 2); // Border
    let (scale_h, scale_w) = (mxh as f32 / h as f32, mxw as f32 / w as f32);
    let scale = if scale_h < scale_w { scale_h } else { scale_w };

    if st.auto_shrink && 0.0 < scale && scale < 1.0 {
        let dest_h = mxh.min((h as f32 * scale) as i32);
        let dest_w = mxw.min((w as f32 * scale) as i32);
        let scaled = pixbuf
            .scale_simple(dest_w, dest_h, gdk_pixbuf::InterpType::Hyper)
            .unwrap();
        img_ctl.set_from_pixbuf(Some(&scaled));
    } else {
        img_ctl.set_from_pixbuf(Some(&pixbuf));
    }
}
