use std::{
    fmt::Display,
    ops::Not,
    path::{Path, PathBuf},
};

use crate::world::VideoWorld;

use iced::{futures::SinkExt, widget};
use typst::{
    foundations::{Array, Label, Selector, Str, Value},
    layout::Frame,
    model::Document,
    visualize::Color,
};
use video_rs::{encode::Settings, Encoder, Time};

pub struct Gui {
    frame: usize,
    selected_scene: Scene,
    world: VideoWorld,
    document: Document,
    project_settings: ProjectSettings,
    running: bool,
}

impl Gui {
    pub fn recompile(&mut self) {
        println!("recompile");
        self.document = self.world.compile().unwrap();
        let relative_frame = self.relative_frame();
        self.project_settings = ProjectSettings::new(&self.document);
        self.selected_scene = self
            .project_settings
            .scenes
            .iter()
            .find(|scene| scene.name == self.selected_scene.name)
            .unwrap_or(&self.project_settings.scenes[0])
            .clone();
        self.frame = (self.min_frame() + relative_frame).min(self.max_frame());
    }

    fn current_frame(&self) -> widget::image::Handle {
        let pixels =
            typst_render::render(&self.document.pages[self.frame].frame, 1.0, Color::WHITE);

        widget::image::Handle::from_pixels(
            self.project_settings.width as u32,
            self.project_settings.height as u32,
            pixels.take(),
        )
    }

    pub fn export(&mut self) {
        self.recompile();
        let settings = Settings::preset_h264_yuv420p(
            self.project_settings.width,
            self.project_settings.height,
            false,
        );
        let mut encoder =
            Encoder::new(Path::new("output.mp4"), settings).expect("failed to create encoder");
        let duration = Time::from_nth_of_a_second(self.project_settings.fps);
        let mut position = Time::zero();

        let total = self.document.pages.len();

        for (i, page) in self.document.pages.iter().enumerate() {
            let pixels = typst_render::render(&page.frame, 1.0, Color::WHITE);

            let frame = bytes_to_frame(
                pixels.take(),
                self.project_settings.width,
                self.project_settings.height,
            );

            encoder.encode(&frame, position).unwrap();

            position = position.aligned_with(duration).add();
            println!("{:5}/{:5}", i, total);
        }
        encoder.finish().unwrap();
        println!("exported");
    }

    fn min_frame(&self) -> usize {
        self.selected_scene.start
    }
    fn max_frame(&self) -> usize {
        self.selected_scene.end
    }

    fn relative_frame(&self) -> usize {
        self.frame - self.min_frame()
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Recompile,
    NextFrame,
    SetFrame(u32),
    ChangeScene(Scene),
    PrevFrame,
    Export,
    Start,
    Continue,
    Pause,
}

type Command = iced::Command<Message>;

impl iced::Application for Gui {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Theme = iced::theme::Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command) {
        let world = VideoWorld::new("./example/main.typ".into(), None);
        let document = world.compile().unwrap();
        let project_settings = ProjectSettings::new(&document);

        (
            Self {
                frame: 0,
                selected_scene: project_settings.scenes[0].clone(),
                world,
                document,
                project_settings,
                running: false,
            },
            iced::Command::none(),
        )
    }

    fn title(&self) -> String {
        if let Some(title) = &self.document.title {
            format!("Typst Video | {}", title)
        } else {
            "Typst Video".into()
        }
    }

    fn update(&mut self, message: Self::Message) -> Command {
        match message {
            Message::Recompile => self.recompile(),
            Message::NextFrame => {
                self.frame = (self.frame + 1).min(self.max_frame());
                if self.frame == self.max_frame() {
                    self.running = false;
                }
            }
            Message::SetFrame(frame) => {
                self.frame = frame as usize;
                self.running = false;
            }
            Message::PrevFrame => self.frame = self.frame.saturating_sub(1).max(self.min_frame()),
            Message::Start => {
                self.frame = self.min_frame();
                self.running = true;
            }
            Message::ChangeScene(scene) => {
                self.selected_scene = scene;
                self.frame = self.selected_scene.start;
                self.running = false;
            }
            Message::Continue => self.running = true,
            Message::Pause => self.running = false,
            Message::Export => self.export(),
        }
        Command::none()
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let handle = self.current_frame();

        let image = widget::container(
            widget::image(handle)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        )
        .padding(5)
        .style(
            widget::container::Appearance::default()
                .with_background(iced::Color::from_rgb(0.3, 0.7, 0.9)),
        );

        let scene_select = widget::pick_list(
            self.project_settings.scenes.as_slice(),
            Some(&self.selected_scene),
            Message::ChangeScene,
        )
        .width(iced::Length::Fill);

        let ui = widget::column![
            widget::text(format!(
                "{} ({})",
                self.relative_frame() + 1,
                self.frame + 1
            ))
            .size(25),
            widget::button("Increment")
                .width(iced::Length::Fill)
                .on_press_maybe(self.running.not().then_some(Message::NextFrame)),
            widget::button("Decrement")
                .width(iced::Length::Fill)
                .on_press_maybe(self.running.not().then_some(Message::PrevFrame)),
            widget::button("Export")
                .width(iced::Length::Fill)
                .on_press(Message::Export),
            widget::button("Start")
                .width(iced::Length::Fill)
                .on_press_maybe(self.running.not().then_some(Message::Start)),
            widget::button("Continue")
                .width(iced::Length::Fill)
                .on_press_maybe(
                    (self.running.not() && self.frame < self.max_frame())
                        .then_some(Message::Continue)
                ),
            widget::button("Pause")
                .width(iced::Length::Fill)
                .on_press_maybe(self.running.then_some(Message::Pause)),
            scene_select,
        ]
        .width(200.0)
        .align_items(iced::Alignment::Center);

        widget::column!(
            widget::slider(
                (self.min_frame() as u32)..=(self.max_frame() as u32),
                self.frame as u32,
                Message::SetFrame
            ),
            widget::row![ui, image].spacing(10),
        )
        .spacing(10)
        .padding(20)
        .into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        let mut subscriptions = Vec::new();
        subscriptions.push(file_watcher(self.world.root().to_owned()));

        if self.running {
            let update = iced::time::every(std::time::Duration::from_secs_f64(
                1.0 / self.project_settings.fps as f64,
            ))
            .map(|_| Message::NextFrame);
            subscriptions.push(update);
        }

        iced::Subscription::batch(subscriptions)
    }
}

fn file_watcher(path: PathBuf) -> iced::Subscription<Message> {
    struct FileWatcher;

    iced::subscription::channel(
        std::any::TypeId::of::<FileWatcher>(),
        2,
        |mut output| async move {
            let (mut sender, mut reciever) = iced::futures::channel::mpsc::channel(1);

            let mut watcher = notify_debouncer_mini::new_debouncer(
                std::time::Duration::from_secs_f64(0.1),
                move |_| {
                    iced::futures::executor::block_on(async {
                        sender.send(Ok(Message::Recompile)).await.unwrap();
                    })
                },
            )
            .unwrap();
            println!("new watcher");

            watcher
                .watcher()
                .watch(&path, notify::RecursiveMode::Recursive)
                .unwrap();

            output.send_all(&mut reciever).await.unwrap();
            unreachable!()
        },
    )
}

fn frame_size(frame: &Frame) -> (usize, usize) {
    let size = frame.size();
    let width = size.x.to_pt();
    let height = size.y.to_pt();
    assert_eq!(width, width.trunc());
    assert_eq!(height, height.trunc());
    (width as usize, height as usize)
}

fn bytes_to_frame(mut data: Vec<u8>, width: usize, height: usize) -> ndarray::Array3<u8> {
    for y in 0..height {
        for x in 0..width {
            let offset = y * width + x;
            for c in 0..3 {
                data[offset * 3 + c] = data[offset * 4 + c];
            }
        }
    }
    data.resize(width * height * 3, 0);
    ndarray::Array3::from_shape_vec((height, width, 3), data).unwrap()
}

fn query(document: &Document, name: &str) -> Value {
    let meta = document
        .introspector
        .query_unique(&Selector::Label(Label::new(name)))
        .unwrap();
    meta.get_by_name("value").unwrap()
}

struct ProjectSettings {
    width: usize,
    height: usize,
    fps: usize,
    scenes: Vec<Scene>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Scene {
    name: String,
    start: usize,
    end: usize,
}

impl Display for Scene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.name.fmt(f)
    }
}

impl ProjectSettings {
    pub fn new(document: &Document) -> Self {
        let (width, height) = frame_size(&document.pages[0].frame);
        let scenes = match query(document, "final-scenes") {
            Value::Array(array) => array,
            other => panic!("scenes are {:?}, not dictionary", other),
        };
        let mut scenes = scenes
            .into_iter()
            .map(|scene| {
                let scene = scene.cast::<Array>().unwrap();
                let name = scene.at(0, None).unwrap().cast::<Str>().unwrap();
                let start = scene.at(1, None).unwrap().cast::<usize>().unwrap();
                let end = scene.at(2, None).unwrap().cast::<usize>().unwrap();

                Scene {
                    name: name.into(),
                    start: start - 1,
                    end: end - 1,
                }
            })
            .collect::<Vec<_>>();
        scenes.insert(
            0,
            Scene {
                name: "All Scenes".into(),
                start: 0,
                end: scenes.last().unwrap().end,
            },
        );

        for page in document.pages.iter().skip(1) {
            let size = frame_size(&page.frame);
            assert_eq!((width, height), size);
        }

        let fps = match query(document, "fps") {
            Value::Int(value) => value as usize,
            other => panic!("fps are {:?}, not number", other),
        };
        Self {
            width,
            height,
            fps,
            scenes,
        }
    }
}
