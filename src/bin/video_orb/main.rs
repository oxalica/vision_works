use failure::{ensure, format_err, Error, ResultExt as _};
use gio::prelude::*;
use glib::{value::Value, IsA, Object};
use gtk::{prelude::*, Application, ApplicationWindow, Builder};
use once_cell::sync::OnceCell;
use std::{cell::RefCell, path::PathBuf, rc::Rc, sync::Arc};

const GLADE_SRC_PATH: &str = "glade/video_orb.glade";

static GUI_EVENT_TX: OnceCell<glib::Sender<GuiEvent>> = OnceCell::new();

mod worker;

/// RGB Frame
pub struct Frame {
    height: usize,
    width: usize,
    row_stride: usize,
    data: Vec<u8>,
}

pub enum GuiEvent {
    Frame(Frame),
    Error(String),
}

#[derive(Default)]
struct GuiState {
    running: bool,
    file_name: Option<PathBuf>,
    stop_guard: Option<Arc<()>>,
    frame_counter: usize,
}

type RcGuiState = Rc<RefCell<GuiState>>;

pub trait BuilderExtManualExt {
    fn object<T: IsA<Object>>(&self, name: &str) -> T;
}

impl<U: BuilderExtManual> BuilderExtManualExt for U {
    fn object<T: IsA<Object>>(&self, name: &str) -> T {
        self.get_object(name).unwrap_or_else(|| {
            panic!(
                "Missing object `{}` of type `{}`",
                name,
                std::any::type_name::<T>(),
            );
        })
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

        let state = RcGuiState::default();
        builder.connect_signals(|builder, handler_name| {
            resolve_handler(builder, handler_name, &state)
        });

        // 1s timer for calculating current FPS.
        let state_ = state.clone();
        let builder_ = builder.clone();
        gtk::timeout_add_seconds(1, move || {
            on_calc_fps(&builder_, &state_);
            glib::Continue(true)
        });

        let (tx, rx) = glib::MainContext::channel(glib::PRIORITY_DEFAULT);
        GUI_EVENT_TX.set(tx).expect("Initialize more than once");
        rx.attach(None, move |event| {
            on_event(event, &builder, &state);
            glib::Continue(true)
        });

        window.show_all();
    });
    app.run(&std::env::args().collect::<Vec<_>>());
}

fn on_calc_fps(builder: &Builder, state: &RcGuiState) {
    let fps = std::mem::replace(&mut state.borrow_mut().frame_counter, 0);
    builder
        .object::<gtk::Label>("lbl_current_fps")
        .set_label(&fps.to_string());
}

fn on_event(event: GuiEvent, builder: &Builder, state: &RcGuiState) {
    match event {
        GuiEvent::Error(err) => popup_error(builder, &err),
        GuiEvent::Frame(mut frame) => {
            use gdk_pixbuf::{Colorspace, Pixbuf};
            let pixbuf = Pixbuf::new_from_mut_slice(
                &mut frame.data,
                Colorspace::Rgb,
                false,
                8,
                frame.width as i32,
                frame.height as i32,
                frame.row_stride as i32,
            );

            builder
                .object::<gtk::Image>("img_out")
                .set_from_pixbuf(Some(&pixbuf));
            state.borrow_mut().frame_counter += 1;
        }
    }
}

fn resolve_handler(
    builder: &Builder,
    handler_name: &str,
    state: &RcGuiState,
) -> Box<dyn Fn(&[Value]) -> Option<Value> + 'static> {
    let builder = builder.clone();
    let state = state.clone();
    match handler_name {
        "on_select_src" => Box::new(move |_| {
            let file_img: gtk::FileChooser = builder.object("file_src");
            state.borrow_mut().file_name = file_img.get_filename();
            None
        }),
        "on_run" => Box::new(move |_| {
            if let Err(err) = run_process(&builder, &state) {
                popup_error(&builder, &format!("Error: {}", err));
            }
            None
        }),
        "on_stop" => Box::new(move |_| {
            builder.object::<gtk::Button>("btn_run").set_sensitive(true);
            builder
                .object::<gtk::Button>("btn_stop")
                .set_sensitive(false);
            let mut state = state.borrow_mut();
            state.stop_guard = None;
            state.running = false;
            None
        }),
        _ => unreachable!("Unknow handle_name: {}", handler_name),
    }
}

fn popup_error(builder: &Builder, msg: &str) {
    let window: gtk::ApplicationWindow = builder.object("wnd_main");
    let dialog = gtk::MessageDialog::new(
        Some(&window),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Error,
        gtk::ButtonsType::Ok,
        msg,
    );
    dialog.run();
    dialog.destroy();
}

fn run_process(builder: &Builder, state: &RcGuiState) -> Result<(), Error> {
    use opencv::videoio::{VideoCapture, VideoCaptureTrait as _, CAP_FFMPEG, CAP_PROP_FPS};

    let mut state = state.borrow_mut();
    let file_name_str = state
        .file_name
        .as_ref()
        .ok_or_else(|| format_err!("No file opened"))?
        .to_str()
        .ok_or_else(|| format_err!("Invalid path"))?;

    let cap = VideoCapture::from_file(file_name_str, CAP_FFMPEG).context("Invalid video file")?;
    ensure!(cap.is_opened()?, "Invalid video file");
    let fps = cap.get(CAP_PROP_FPS)?;

    let guard = Arc::new(());
    state.stop_guard = Some(guard.clone());
    let guard_weak = Arc::downgrade(&guard);
    let event_tx = GUI_EVENT_TX.get().unwrap().clone();
    let worker_thread = std::thread::spawn(move || worker::worker(cap, fps, event_tx, guard_weak));
    // Watching dog thread
    std::thread::spawn(move || {
        let err_str = match worker_thread.join() {
            Ok(Ok(())) => return,
            // Worker returns Err.
            Ok(Err(err)) => err.to_string(),
            // Worker panicked.
            Err(err) => format!("Worker panicked: {:?}", err),
        };
        GUI_EVENT_TX
            .get()
            .unwrap()
            .send(GuiEvent::Error(err_str))
            .unwrap();
    });

    builder
        .object::<gtk::Label>("lbl_video_fps")
        .set_label(&format!("{:.3}", fps));
    builder
        .object::<gtk::Button>("btn_run")
        .set_sensitive(false);
    builder
        .object::<gtk::Button>("btn_stop")
        .set_sensitive(true);
    state.running = true;
    Ok(())
}
