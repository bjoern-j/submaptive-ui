use eframe::egui;
use image::GenericImageView;

fn main() {
    eframe::run_native(
        "Submaptive",
        Default::default(),
        Box::new(|_| Box::new(App::new())),
    )
    .unwrap();
}

#[derive(Clone)]
struct ImageData {
    image: image::DynamicImage,
    handle: egui::TextureHandle,
}

#[derive(Clone)]
enum ProjectionData {
    Equirectangular(submaptive::Equirectangular),
}

impl ProjectionData {
    pub fn kind(&self) -> ProjectionKind {
        use ProjectionData::*;
        match self {
            Equirectangular(_) => ProjectionKind::Equirectangular,
        }
    }
}

impl submaptive::Projection for ProjectionData {
    fn dimensions(&self) -> submaptive::Dimensions {
        match self {
            ProjectionData::Equirectangular(data) => data.dimensions(),
        }
    }

    fn project(&self, point: &submaptive::Point) -> (f64, f64) {
        match self {
            ProjectionData::Equirectangular(data) => data.project(point),
        }
    }

    fn invert(&self, projected_point: (f64, f64)) -> submaptive::Point {
        match self {
            ProjectionData::Equirectangular(data) => data.invert(projected_point),
        }
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum ProjectionKind {
    Equirectangular,
}

impl ProjectionKind {
    pub fn all() -> impl Iterator<Item = Self> {
        use ProjectionKind::*;
        vec![Equirectangular].into_iter()
    }

    pub fn default_projection_data(&self) -> ProjectionData {
        use ProjectionKind::*;
        match self {
            Equirectangular => {
                ProjectionData::Equirectangular(submaptive::Equirectangular::new().build())
            }
        }
    }
}

impl std::fmt::Display for ProjectionKind {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ProjectionKind::*;
        fmt.write_str(match self {
            Equirectangular => "Equirectangular",
        })
    }
}

struct App {
    error: Option<String>,
    source_image: Option<ImageData>,
    source_projection: ProjectionData,
    target_projection: ProjectionData,
    projected_image: Option<ImageData>,
}

impl App {
    pub fn new() -> Self {
        App {
            error: None,
            source_image: None,
            source_projection: ProjectionData::Equirectangular(
                submaptive::Equirectangular::new().build(),
            ),
            target_projection: ProjectionData::Equirectangular(
                submaptive::Equirectangular::new().build(),
            ),
            projected_image: None,
        }
    }

    fn load_source_image(&mut self, path: std::path::PathBuf, ctx: &egui::Context) {
        let image = image::io::Reader::open(path).map(|data| data.decode());
        match image {
            Ok(image) => match image {
                Ok(image) => {
                    self.source_image = Some(ImageData {
                        image: image.clone(),
                        handle: ctx.load_texture(
                            "Source image",
                            egui::ColorImage::from_rgba_unmultiplied(
                                [image.width() as usize, image.height() as usize],
                                image.to_rgba8().as_flat_samples().as_slice(),
                            ),
                            Default::default(),
                        ),
                    })
                }
                Err(e) => {
                    self.error = Some(e.to_string());
                }
            },
            Err(e) => {
                self.error = Some(e.to_string());
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("Controls")
            .width_range(100.0..=1000.0)
            .show(ctx, |ui| {
                if ui.button("Choose source map...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_file() {
                        self.load_source_image(path, ctx);
                    }
                }
                projection_ui(ui, &mut self.source_projection, "Source projection");
                projection_ui(ui, &mut self.target_projection, "Target projection");
                if self.source_image.is_some() && ui.button("Project!").clicked() {
                    let image = submaptive::Map::new(
                        self.source_image.clone().unwrap().image,
                        self.source_projection.clone(),
                    )
                    .convert_to(self.target_projection.clone())
                    .to_image();
                    let handle = ctx.load_texture(
                        "Source image",
                        egui::ColorImage::from_rgba_unmultiplied(
                            [image.width() as usize, image.height() as usize],
                            image.to_rgba8().as_flat_samples().as_slice(),
                        ),
                        Default::default(),
                    );
                    self.projected_image = Some(ImageData { image, handle });
                }
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(source_image) = &self.source_image {
                let dimensions = source_image.image.dimensions();
                let dimensions = (dimensions.0 as f32, dimensions.1 as f32);
                let dimensions = (400. * (dimensions.0 / dimensions.1), 400.);
                ui.image(source_image.handle.id(), dimensions);
            }
            if let Some(target_image) = &self.projected_image {
                let dimensions = target_image.image.dimensions();
                let dimensions = (dimensions.0 as f32, dimensions.1 as f32);
                let dimensions = (400. * (dimensions.0 / dimensions.1), 400.);
                ui.image(target_image.handle.id(), dimensions);
            }
        });
        if let Some(error) = &self.error {
            egui::TopBottomPanel::bottom("Dialogue").show(ctx, |ui| {
                ui.colored_label(egui::Color32::RED, error);
            });
        }
    }
}

fn projection_ui(ui: &mut egui::Ui, projection: &mut ProjectionData, label: &str) {
    egui::ComboBox::new(label, label)
        .selected_text(projection.kind().to_string())
        .show_ui(ui, |ui| {
            let mut projection_kind = projection.kind();
            for projection in ProjectionKind::all() {
                ui.selectable_value(&mut projection_kind, projection, projection.to_string());
            }
            if projection_kind != projection.kind() {
                *projection = projection_kind.default_projection_data();
            }
        });
    match projection {
        ProjectionData::Equirectangular(equirect_data) => {
            let mut central_long = equirect_data.central_long();
            let mut true_scale_lat = equirect_data.true_scale_lat();
            ui.add(
                egui::Slider::new(&mut central_long, -180.0..=180.)
                    .suffix("°")
                    .clamp_to_range(true)
                    .text("Central longitude"),
            );
            ui.add(
                egui::Slider::new(&mut true_scale_lat, -90.0..=90.0)
                    .suffix("°")
                    .clamp_to_range(true)
                    .text("True scale latitude"),
            );
            *projection = ProjectionData::Equirectangular(
                submaptive::Equirectangular::new()
                    .central_long(central_long)
                    .true_scale_lat(true_scale_lat)
                    .build(),
            );
        }
    }
}
