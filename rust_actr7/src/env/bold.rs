//! BOLD 2D and 3D brain views, ported from the Tcl Environment dialogs.
//!
//! The 2D view overlays the Tcl `ref-brain.gif` image with fixed module
//! regions. The 3D view renders the Tcl voxel mesh with a small perspective
//! camera and painter's-algorithm hidden-surface removal.

use eframe::egui;

use crate::json::Json;

use super::bold_data::{
    BOLD_2D_CANVAS, BOLD_2D_IMAGE_POS, BOLD_2D_IMAGE_SIZE, BOLD_2D_LABELS, BOLD_2D_REGIONS,
    BOLD_3D_CANVAS, BOLD_3D_FACES, BOLD_3D_SCREEN, BOLD_3D_VERTICES, MODULES,
};

const TWO_PI: f32 = std::f32::consts::PI * 2.0;
const RHO: f32 = 5.0;
const DEFAULT_DIST: f32 = 1200.0;

pub(crate) const COLORS_2D: [egui::Color32; 8] = [
    egui::Color32::from_rgb(255, 0, 0),
    egui::Color32::from_rgb(0, 255, 0),
    egui::Color32::from_rgb(0, 0, 255),
    egui::Color32::from_rgb(255, 255, 0),
    egui::Color32::from_rgb(255, 0, 255),
    egui::Color32::from_rgb(255, 0, 255),
    egui::Color32::from_rgb(0, 255, 0),
    egui::Color32::from_rgb(0, 0, 255),
];

pub(crate) const COLORS_3D: [egui::Color32; 8] = [
    egui::Color32::from_rgb(255, 0, 0),
    egui::Color32::from_rgb(0, 153, 0),
    egui::Color32::from_rgb(0, 0, 255),
    egui::Color32::from_rgb(170, 170, 0),
    egui::Color32::from_rgb(0, 170, 170),
    egui::Color32::from_rgb(170, 0, 170),
    egui::Color32::from_rgb(0, 238, 0),
    egui::Color32::from_rgb(51, 102, 170),
];

pub(crate) struct BoldRow {
    pub(crate) name: String,
    pub(crate) values: Vec<Option<f32>>,
    pub(crate) module: Option<usize>,
}

pub(crate) struct BoldData {
    pub(crate) times: Vec<f64>,
    pub(crate) rows: Vec<BoldRow>,
}

impl BoldData {
    pub(crate) fn parse(records: &[Json]) -> Self {
        let mut times = Vec::new();
        let mut rows = Vec::new();
        for record in records {
            let Json::Arr(fields) = record else {
                continue;
            };
            let name = fields.first().and_then(|x| x.as_str()).unwrap_or("");
            let values: Vec<Option<f32>> = fields
                .iter()
                .skip(1)
                .map(|v| v.as_f64().map(|n| n as f32))
                .collect();
            if name.eq_ignore_ascii_case("time") {
                times = fields.iter().skip(1).filter_map(|v| v.as_f64()).collect();
            } else {
                rows.push(BoldRow {
                    name: name.to_string(),
                    values,
                    module: module_slot(name),
                });
            }
        }
        let max_len = rows
            .iter()
            .map(|r| r.values.len())
            .max()
            .unwrap_or(0)
            .max(times.len());
        if times.is_empty() {
            times = (0..max_len).map(|i| i as f64).collect();
        }
        BoldData { times, rows }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(crate) fn max_scan(&self) -> usize {
        let n = self
            .times
            .len()
            .max(self.rows.iter().map(|r| r.values.len()).max().unwrap_or(0));
        n.saturating_sub(1)
    }

    pub(crate) fn value(&self, module: usize, scan: usize) -> Option<f32> {
        self.rows
            .iter()
            .rev()
            .find(|r| r.module == Some(module))
            .and_then(|r| r.values.get(scan).copied().flatten())
    }

    pub(crate) fn has_module(&self, module: usize) -> bool {
        self.rows.iter().any(|r| r.module == Some(module))
    }
}

fn module_slot(name: &str) -> Option<usize> {
    match name.to_ascii_lowercase().as_str() {
        "manual" => Some(0),
        "goal" => Some(1),
        "vocal" => Some(2),
        "imaginal" => Some(3),
        "retrieval" => Some(4),
        "procedural" | "production" => Some(5),
        "aural" => Some(6),
        "visual" => Some(7),
        _ => None,
    }
}

pub(crate) fn decode_brain_image() -> Result<egui::ColorImage, String> {
    const BYTES: &[u8] = include_bytes!("../data/ref-brain.gif");
    let image = image::load_from_memory_with_format(BYTES, image::ImageFormat::Gif)
        .map_err(|e| format!("could not decode ref-brain.gif: {e}"))?;
    let rgba = image.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

fn module_tint(module: usize, level: f32) -> egui::Color32 {
    let color = COLORS_2D[module];
    let level = level.clamp(0.0, 1.0);
    let scale = |c: u8| (c as f32 * level).round() as u8;
    egui::Color32::from_rgb(scale(color.r()), scale(color.g()), scale(color.b()))
}

fn gray(value: Option<f32>) -> egui::Color32 {
    match value {
        Some(value) => egui::Color32::from_gray((value.clamp(0.0, 1.0) * 255.0).round() as u8),
        None => egui::Color32::BLACK,
    }
}

pub(crate) fn draw_2d(
    ui: &mut egui::Ui,
    data: &BoldData,
    scan: usize,
    texture: Option<&egui::TextureHandle>,
    image_error: Option<&str>,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(BOLD_2D_CANVAS[0], BOLD_2D_CANVAS[1]),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::GRAY);

    if let Some(texture) = texture {
        painter.image(
            texture.id(),
            egui::Rect::from_min_size(
                rect.min + egui::vec2(BOLD_2D_IMAGE_POS[0], BOLD_2D_IMAGE_POS[1]),
                egui::vec2(BOLD_2D_IMAGE_SIZE[0], BOLD_2D_IMAGE_SIZE[1]),
            ),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else if let Some(error) = image_error {
        painter.text(
            rect.min + egui::vec2(12.0, 12.0),
            egui::Align2::LEFT_TOP,
            error,
            egui::FontId::monospace(11.0),
            egui::Color32::RED,
        );
    }

    for &(module, [x1, y1, x2, y2]) in BOLD_2D_REGIONS {
        if !data.has_module(module as usize) {
            continue;
        }
        let region =
            egui::Rect::from_min_max(rect.min + egui::vec2(x1, y1), rect.min + egui::vec2(x2, y2));
        let outline = module_tint(module as usize, 0.75);
        let fill = if scan == 0 {
            egui::Color32::TRANSPARENT
        } else {
            module_tint(
                module as usize,
                data.value(module as usize, scan).unwrap_or(0.0),
            )
        };
        painter.rect(
            region,
            0.0,
            fill,
            egui::Stroke::new(1.0, outline),
            egui::StrokeKind::Inside,
        );
    }

    for &(module, [x, y]) in BOLD_2D_LABELS {
        let color = if data.has_module(module as usize) {
            module_tint(module as usize, 0.75)
        } else {
            egui::Color32::GRAY
        };
        painter.text(
            rect.min + egui::vec2(x, y),
            egui::Align2::LEFT_CENTER,
            MODULES[module as usize],
            egui::FontId::monospace(10.0),
            color,
        );
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Camera {
    pub(crate) theta: f32,
    pub(crate) phi: f32,
    pub(crate) dist: f32,
    pub(crate) pan: egui::Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            theta: 0.0,
            phi: 0.0,
            dist: DEFAULT_DIST,
            pan: egui::Vec2::ZERO,
        }
    }
}

fn wrap_angle(angle: f32) -> f32 {
    if angle > std::f32::consts::PI {
        angle - TWO_PI
    } else if angle < -std::f32::consts::PI {
        angle + TWO_PI
    } else {
        angle
    }
}

fn viewing_transform(theta: f32, phi: f32) -> [[f32; 4]; 4] {
    let sinth = theta.sin();
    let costh = theta.cos();
    let sinph = phi.sin();
    let cosph = phi.cos();
    [
        [-sinth, -costh * cosph, -costh * sinph, 0.0],
        [costh, -sinth * cosph, -sinth * sinph, 0.0],
        [0.0, sinph, -cosph, 0.0],
        [0.0, 0.0, RHO, 1.0],
    ]
}

fn eye_point(theta: f32, phi: f32) -> [f32; 3] {
    let sinph = phi.sin();
    let cosph = phi.cos();
    [
        RHO * sinph * theta.cos(),
        RHO * sinph * theta.sin(),
        RHO * cosph,
    ]
}

fn transform_point(m: &[[f32; 4]; 4], point: [f32; 3]) -> [f32; 4] {
    let mut out = [0.0; 4];
    for (k, slot) in out.iter_mut().enumerate() {
        *slot = m[0][k] * point[0] + m[1][k] * point[1] + m[2][k] * point[2] + m[3][k];
    }
    out
}

fn project_point(m: &[[f32; 4]; 4], camera: &Camera, point: [f32; 3]) -> egui::Pos2 {
    let transformed = transform_point(m, point);
    let z = if transformed[2].abs() < 1e-6 {
        1e-6
    } else {
        transformed[2]
    };
    egui::pos2(
        BOLD_3D_SCREEN[0] / 2.0 + camera.dist * transformed[0] / z + camera.pan.x,
        BOLD_3D_SCREEN[1] / 2.0 - camera.dist * transformed[1] / z - camera.pan.y,
    )
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

struct PaintedFace {
    distance: f32,
    points: Vec<egui::Pos2>,
    module: Option<u8>,
}

pub(crate) fn draw_3d(ui: &mut egui::Ui, data: &BoldData, scan: usize, camera: &mut Camera) {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(BOLD_3D_CANVAS[0], BOLD_3D_CANVAS[1]),
        egui::Sense::click_and_drag(),
    );
    let delta = ui.input(|i| i.pointer.delta());
    if response.dragged_by(egui::PointerButton::Primary) {
        camera.theta = wrap_angle(camera.theta - delta.x * 0.01);
        camera.phi = wrap_angle(camera.phi - delta.y * 0.01);
    }
    if response.dragged_by(egui::PointerButton::Middle) {
        camera.pan.x += delta.x;
        camera.pan.y -= delta.y;
    }
    if response.dragged_by(egui::PointerButton::Secondary) {
        camera.dist += -delta.y * 2.0;
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

    let matrix = viewing_transform(camera.theta, camera.phi);
    let eye = eye_point(camera.theta, camera.phi);
    let mut faces = Vec::with_capacity(BOLD_3D_FACES.len());
    for face in BOLD_3D_FACES {
        let v1 = BOLD_3D_VERTICES[face.verts[0]];
        let line_of_sight = [eye[0] - v1[0], eye[1] - v1[1], eye[2] - v1[2]];
        if dot(face.normal, line_of_sight) < 0.0 && face.module.is_none() {
            continue;
        }
        let mut points = Vec::with_capacity(face.len as usize);
        let mut total_distance = 0.0;
        for i in 0..face.len as usize {
            let vertex = BOLD_3D_VERTICES[face.verts[i]];
            points.push(rect.min + project_point(&matrix, camera, vertex).to_vec2());
            total_distance += distance(vertex, eye);
        }
        faces.push(PaintedFace {
            distance: total_distance / face.len as f32,
            points,
            module: face.module,
        });
    }
    faces.sort_by(|a, b| b.distance.total_cmp(&a.distance));

    for face in &faces {
        let (fill, stroke) = match face.module {
            Some(module) => (
                gray(data.value(module as usize, scan)),
                egui::Stroke::new(3.0, COLORS_3D[module as usize]),
            ),
            None => (
                egui::Color32::TRANSPARENT,
                egui::Stroke::new(1.0, egui::Color32::from_gray(0x8f)),
            ),
        };
        painter.add(egui::Shape::convex_polygon(
            face.points.clone(),
            fill,
            stroke,
        ));
    }

    for (module, &name) in MODULES.iter().enumerate() {
        let y = 200.0 + module as f32 * 40.0;
        painter.text(
            rect.min + egui::vec2(10.0, y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::monospace(10.0),
            COLORS_3D[module],
        );
        let value = data.value(module, scan);
        painter.text(
            rect.min + egui::vec2(70.0, y),
            egui::Align2::LEFT_CENTER,
            value
                .map(|v| format!("{v:.3}"))
                .unwrap_or_else(|| "---".to_string()),
            egui::FontId::monospace(10.0),
            egui::Color32::from_gray(0xcc),
        );
    }

    painter.text(
        rect.min + egui::vec2(10.0, 10.0),
        egui::Align2::LEFT_CENTER,
        "0.0",
        egui::FontId::monospace(9.0),
        egui::Color32::from_gray(0xdd),
    );
    painter.text(
        rect.min + egui::vec2(10.0, 170.0),
        egui::Align2::LEFT_CENTER,
        "1.0",
        egui::FontId::monospace(9.0),
        egui::Color32::from_gray(0xdd),
    );
    for c in (0..256).step_by(3) {
        let y = (c / 3 * 2) as f32;
        painter.rect_filled(
            egui::Rect::from_min_size(rect.min + egui::vec2(30.0, y), egui::vec2(10.0, 2.0)),
            0.0,
            egui::Color32::from_gray(c as u8),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_missing_bold_values_and_module_aliases() {
        let records = vec![
            Json::Arr(vec![
                Json::Str("time".into()),
                Json::Float(0.0),
                Json::Float(2.0),
            ]),
            Json::Arr(vec![Json::Str("goal".into()), Json::Float(0.1), Json::Null]),
            Json::Arr(vec![
                Json::Str("production".into()),
                Json::Null,
                Json::Float(0.7),
            ]),
        ];
        let data = BoldData::parse(&records);
        assert_eq!(data.times, vec![0.0, 2.0]);
        assert_eq!(data.value(1, 0), Some(0.1));
        assert_eq!(data.value(1, 1), None);
        assert_eq!(data.value(5, 1), Some(0.7));
        assert!(!data.has_module(0));
        assert!(data.has_module(5));
    }

    #[test]
    fn brain_image_decodes_at_reference_size() {
        let image = decode_brain_image().expect("ref-brain.gif");
        assert_eq!(image.size, [640, 640]);
    }

    #[test]
    fn projection_matches_tcl_center_and_rotates() {
        let camera = Camera::default();
        let matrix = viewing_transform(camera.theta, camera.phi);
        let center = project_point(&matrix, &camera, [0.0, 0.0, 0.0]);
        assert!((center.x - 375.0).abs() < 0.01);
        assert!((center.y - 250.0).abs() < 0.01);

        let rotated = Camera {
            theta: 0.3,
            ..Camera::default()
        };
        let matrix = viewing_transform(rotated.theta, rotated.phi);
        let moved = project_point(&matrix, &rotated, [0.5, 0.0, 0.0]);
        assert!((moved.x - center.x).abs() > 1.0);
    }

    #[test]
    fn tcl_mesh_and_regions_are_intact() {
        assert_eq!(BOLD_3D_VERTICES.len(), 479);
        assert_eq!(BOLD_3D_FACES.len(), 271);
        assert_eq!(BOLD_2D_REGIONS.len(), 62);
        assert_eq!(BOLD_2D_LABELS.len(), 8);
        for face in BOLD_3D_FACES {
            for &index in &face.verts[..face.len as usize] {
                assert!(index < BOLD_3D_VERTICES.len());
            }
            let mag = dot(face.normal, face.normal);
            assert!((mag - 1.0).abs() < 1e-4, "normal magnitude {mag}");
        }
        let module_faces = BOLD_3D_FACES.iter().filter(|f| f.module.is_some()).count();
        assert_eq!(module_faces, 96);
    }
}
