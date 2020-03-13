use failure::{ensure, Error, ResultExt as _};
use gio::prelude::*;
use glib::value::Value;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder};
use opencv::prelude::*;

const GLADE_SRC_PATH: &str = "glade/img_process.glade";
const IMAGE_MAX_SIZE: i32 = 600;

type Result<T> = std::result::Result<T, Error>;

mod error;
use error::OptionExt as _;

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

// https://mail.gnome.org/archives/gtk-list/2007-May/msg00034.html
fn log(builder: &Builder, content: &str) {
    let txt: gtk::TextView = builder.get_object("txt_log").unwrap();
    let buf = txt.get_buffer().unwrap();
    let mark = buf.get_insert().unwrap();
    let iter = buf.get_end_iter();
    buf.move_mark(&mark, &iter);
    buf.insert_at_cursor(content);
    buf.insert_at_cursor("\n");
    txt.scroll_to_mark(&mark, 0.0, false, 0.0, 0.0);
}

fn resolve_handler(
    builder: &Builder,
    handler_name: &str,
) -> Box<dyn Fn(&[Value]) -> Option<Value> + 'static> {
    let builder = builder.clone();
    match handler_name {
        "on_select_source_file" => Box::new(move |_| {
            on_select_source_file(&builder);
            None
        }),
        "on_clear_log" => Box::new(move |_| {
            let txt_log: gtk::TextView = builder.get_object("txt_log").unwrap();
            txt_log.get_buffer().unwrap().set_text("");
            None
        }),
        _ => unreachable!("Unknow handle_name: {}", handler_name),
    }
}

fn on_select_source_file(builder: &Builder) {
    let file_source: gtk::FileChooser = builder.get_object("file_source").unwrap();
    if let Some(file_name) = file_source.get_filename() {
        log(&builder, &format!("Loading file {}", file_name.display()));
        match (|| -> Result<_> {
            let img_mat = load_image(&file_name).context("Load image")?;
            ensure!(
                img_mat.rows() <= IMAGE_MAX_SIZE && img_mat.cols() <= IMAGE_MAX_SIZE,
                "Image should be smaller than {}x{}",
                IMAGE_MAX_SIZE,
                IMAGE_MAX_SIZE,
            );
            let img_left = builder.get_object("img_left").unwrap();
            show_image(&img_left, &img_mat).context("Show image")?;
            log(
                &builder,
                &format!("Loaded {}x{}", img_mat.rows(), img_mat.cols()),
            );
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) => log(&builder, &format!("Error: {}", err)),
        }
    }
}

fn load_image(path: &std::path::Path) -> Result<Mat> {
    use opencv::imgcodecs::*;
    let path_str = path.to_str().context("Path is not valid UTF8")?;
    let img = imread(path_str, IMREAD_COLOR).context("imread")?;
    Ok(img)
}

fn show_image(img: &gtk::Image, data: &Mat) -> Result<()> {
    use gdk_pixbuf::{Colorspace, Pixbuf};
    use opencv::core::Vec3b;

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
